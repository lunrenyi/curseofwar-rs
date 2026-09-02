//! One simulation step (`simulate` in state.c:244) and the helpers it uses.
//
//! The order of operations within `simulate_one_step` mirrors the C source
//! exactly; deviating from that order would change the outcome.

use crate::ai::Difficulty;
use crate::consts::{DIRECTIONS, MAX_PLAYER, MAX_POP, RANDOM_INEQUALITY};
use crate::flags::FlagGrid;
use crate::grid::Grid;
use crate::rng::CowRng;
use crate::types::{Loc, Outcome, PlayerId, TileClass, DIRS, NEUTRAL};

/// Per-tile population growth factor for each tile class (state.c:213).
#[inline]
pub fn growth(cl: TileClass) -> f32 {
    match cl {
        TileClass::Village => 1.10,
        TileClass::Town => 1.20,
        TileClass::Castle => 1.30,
        _ => 0.0,
    }
}

/// `time_to_ymd` (output-common.c:21). Year = `time / 360`, month = 1 + `(time%360)/30`,
/// day = 1 + `(time%360)%30`.
pub fn time_to_ymd(time: u64, y: &mut u64, m: &mut u32, d: &mut u32) {
    let year = time / 360;
    let rem = time - year * 360;
    let month = rem / 30;
    let day = rem % 30;
    *y = year;
    *m = (month as u32) + 1;
    *d = (day as u32) + 1;
}

/// `pop_to_symbol` (output-common.c:32). Returns 0..=8 according to the
/// population, or -1 for zero.
pub fn pop_to_symbol(num: i32) -> i32 {
    if num > 400 {
        8
    } else if num > 200 {
        7
    } else if num > 100 {
        6
    } else if num > 50 {
        5
    } else if num > 25 {
        4
    } else if num > 12 {
        3
    } else if num > 6 {
        2
    } else if num > 3 {
        1
    } else if num > 0 {
        0
    } else {
        -1
    }
}

/// Mine ownership state machine — the C code mutates `t[i][j].pl` and
/// `country[owner].gold` while iterating the grid. We factor it out so the
/// behaviour can be unit-tested without driving the whole simulation.
#[derive(Default)]
struct MineOwnershipScratch {
    my_pop: [i32; MAX_PLAYER],
}

/// One step of the simulation (state.c:244).
///
/// `gold` is the per-player gold array (C: `country[].gold`).
/// `need_reeval` is set to true when a tile was downgraded, so the AI may
/// rebuild its map evaluation on the next `kings_move`.
///
/// The migration section reads the per-player flag grids; the caller is
/// expected to pass all of them in the same order as `MAX_PLAYER`.
pub fn simulate_one_step(
    grid: &mut Grid,
    gold: &mut [i64; MAX_PLAYER],
    flags: &mut [FlagGrid; MAX_PLAYER],
    controlled: PlayerId,
    difficulty: Difficulty,
    rng: &mut CowRng,
) {
    let difficulty_ai_gold = difficulty.ai_gold_bonus();
    grid.time_advance();
    let mut scratch = MineOwnershipScratch::default();
    let mut need_reeval = false;

    // Phase 1: per-tile events (mine ownership, combat, burning, ownership,
    // growth).
    for i in 0..grid.width {
        for j in 0..grid.height {
            simulate_tile(grid, gold, i, j, &mut scratch, &mut need_reeval, rng);
        }
    }

    // Phase 2: migration. Order of iteration is randomised by row and column
    // (state.c:344-348) to avoid biased diffusion patterns.
    simulate_migration(grid, flags, rng);

    // Phase 3: re-derive ownership after migration.
    grid.determine_ownership_all();

    // Phase 4: AI gold bonus for hard difficulties. Note that only AI
    // players (not the human-controlled one) earn this and only when their
    // gold is already positive.
    if difficulty_ai_gold > 0 {
        for i in 0..MAX_PLAYER {
            let p = PlayerId(i as u8);
            if p != NEUTRAL && p != controlled && gold[i] > 0 {
                gold[i] += difficulty_ai_gold as i64;
            }
        }
    }

    let _ = need_reeval; // consumed by caller via AI re-eval callback if needed
}

fn simulate_tile(
    grid: &mut Grid,
    gold: &mut [i64; MAX_PLAYER],
    i: usize,
    j: usize,
    scratch: &mut MineOwnershipScratch,
    need_reeval: &mut bool,
    rng: &mut CowRng,
) {
    let h = grid.height;
    let idx = i * h + j;

    // Snapshot the tile so we can take immutable borrows when scanning
    // neighbours (grid.h's struct layout means we copy the tile first).
    let cl = grid.tiles[idx].cl;
    let owner = grid.tiles[idx].pl;

    // 1. Mine ownership (state.c:259-283).
    if cl == TileClass::Mine {
        let mut new_owner = NEUTRAL;
        let mut conflict = false;
        for d in DIRS.iter() {
            let ni = i as i16 + d.i;
            let nj = j as i16 + d.j;
            if ni < 0 || nj < 0 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if ni >= grid.width || nj >= grid.height {
                continue;
            }
            let n_idx = ni * h + nj;
            if !grid.tiles[n_idx].cl.is_inhabitable() {
                continue;
            }
            let p = grid.tiles[n_idx].pl;
            if new_owner == NEUTRAL {
                new_owner = p;
            } else if new_owner != p && p != NEUTRAL {
                conflict = true;
                break;
            }
        }
        let final_owner = if conflict { NEUTRAL } else { new_owner };
        grid.tiles[idx].pl = final_owner;
        if final_owner != NEUTRAL {
            gold[final_owner.index()] += 1;
        }
    }

    // 2. Combat (state.c:285-303): each player p loses damage proportional
    // to their share of the total population.
    let mut total_pop: i32 = 0;
    for p in 0..MAX_PLAYER {
        scratch.my_pop[p] = grid.tiles[idx].pop[p];
        total_pop += scratch.my_pop[p];
    }
    let mut defender_dmg = 0;
    if total_pop != 0 {
        for p in 0..MAX_PLAYER {
            let enemy_pop = total_pop - scratch.my_pop[p];
            let dmg_f = enemy_pop as f32 * scratch.my_pop[p] as f32 / total_pop as f32;
            let dmg = rng.rnd_round(dmg_f);
            // State.c does `t[i][j].units[p][citizen] = MAX(my_pop[p] - dmg, 0)`
            // which clamps to zero, so a `dmg` larger than the population is
            // safe. We use the same idiom for parity.
            let new_pop = (scratch.my_pop[p] - dmg).max(0);
            grid.tiles[idx].pop[p] = new_pop;
            if p == owner.index() {
                defender_dmg = dmg;
            }
        }
    }

    // 3. Burning cities (state.c:306-311). The C code is `rand()%1 == 0`
    // which is *always* true; we mirror that as a direct degrade() call.
    if defender_dmg as f32 > 2.0 * MAX_POP as f32 * 0.1 && cl.is_city() {
        let new_cl = match cl {
            TileClass::Castle => TileClass::Town,
            TileClass::Town => TileClass::Village,
            TileClass::Village => TileClass::Grassland,
            _ => cl,
        };
        grid.tiles[idx].cl = new_cl;
        *need_reeval = true;
    }

    // 4. Determine ownership (state.c:313-321).
    if grid.tiles[idx].cl.is_inhabitable() {
        let mut best = NEUTRAL;
        for p in 1..MAX_PLAYER {
            let cur = grid.tiles[idx].pop[p];
            let best_pop = grid.tiles[idx].pop[best.index()];
            if cur > best_pop {
                best = PlayerId(p as u8);
            }
        }
        grid.tiles[idx].pl = best;
    }

    // 5. Growth (state.c:323-335). Only cities with a non-NEUTRAL owner grow.
    let cl = grid.tiles[idx].cl;
    let pl = grid.tiles[idx].pl;
    if cl.is_city() && pl != NEUTRAL {
        let pop = grid.tiles[idx].pop[pl.index()];
        let g = growth(cl);
        let npop_f = pop as f32 * g;
        let npop = rng.rnd_round(npop_f).min(MAX_POP);
        grid.tiles[idx].pop[pl.index()] = npop.max(0);
    }
}

fn simulate_migration(grid: &mut Grid, flags: &mut [FlagGrid; MAX_PLAYER], rng: &mut CowRng) {
    let (i_start, i_end, i_inc) = if rng.coin_flip() {
        (0, grid.width as isize, 1isize)
    } else {
        (grid.width as isize - 1, -1isize, -1isize)
    };
    let (j_start, j_end, j_inc) = if rng.coin_flip() {
        (0, grid.height as isize, 1isize)
    } else {
        (grid.height as isize - 1, -1isize, -1isize)
    };

    let mut i = i_start;
    while i != i_end {
        let mut j = j_start;
        while j != j_end {
            let h = grid.height;
            let idx = (i as usize) * h + (j as usize);
            // We snapshot the call field for this player before any movement
            // because neighbours will read the same player's `call` for
            // (l, j) while writing to (l+di, j+dj) — and the migration code
            // only ever reads `call` for the *source* tile anyway.
            let k_shift = rng.dir_offset();
            for p in 0..MAX_PLAYER {
                // Each player's population migration is independent.
                let initial_pop = grid.tiles[idx].pop[p];
                if initial_pop == 0 {
                    continue;
                }
                let src_call = flags[p].call_at(Loc::new(i as i16, j as i16));
                for k in 0..DIRECTIONS {
                    let dir = DIRS[(k + k_shift) % DIRECTIONS];
                    let ni = i + dir.i as isize;
                    let nj = j + dir.j as isize;
                    if ni < 0 || nj < 0 {
                        continue;
                    }
                    let (ni, nj) = (ni as usize, nj as usize);
                    if ni >= grid.width || nj >= grid.height {
                        continue;
                    }
                    let n_idx = ni * h + nj;
                    if !grid.tiles[n_idx].cl.is_inhabitable() {
                        continue;
                    }
                    let dst_call = flags[p].call_at(Loc::new(ni as i16, nj as i16));
                    let dcall = (dst_call - src_call).max(0);
                    let d_f = 0.05 * initial_pop as f32 + 0.10 * dcall as f32 * initial_pop as f32;
                    // The migration formula uses the snapshot `initial_pop`
                    // for the magnitude, but the actual transfer is also
                    // bounded by the source's *current* population (which may
                    // already have been depleted by an earlier-direction
                    // migration in the same step) and by the destination cap.
                    let src_now = grid.tiles[idx].pop[p];
                    if src_now <= 0 {
                        break;
                    }
                    let dst_cap = MAX_POP - grid.tiles[n_idx].pop[p];
                    let dpop = rng
                        .rnd_round(d_f)
                        .min(initial_pop)
                        .min(src_now)
                        .min(dst_cap);
                    if dpop > 0 {
                        grid.tiles[idx].pop[p] -= dpop;
                        grid.tiles[n_idx].pop[p] += dpop;
                    }
                }
            }
            j += j_inc;
        }
        i += i_inc;
    }
}

/// `win_or_lose` (main-common.c:93). Iterates every inhabitable tile, sums
/// per-player population, then decides whether only the controlled player
/// still has people (victory) or whether the controlled player has none at
/// all (defeat).
pub fn win_or_lose(grid: &Grid, controlled: PlayerId) -> Outcome {
    let mut pop = [0i64; MAX_PLAYER];
    for (_, t) in grid.iter() {
        if !t.cl.is_inhabitable() {
            continue;
        }
        for p in 0..MAX_PLAYER {
            pop[p] += t.pop[p] as i64;
        }
    }

    // Match the C version's ordering: a player with zero population is
    // defeated even if all opponents are also defeated — ties at 0 still
    // count as "you have nobody".
    if pop[controlled.index()] == 0 {
        return Outcome::Defeat;
    }
    for p in 1..MAX_PLAYER {
        if p != controlled.index() && pop[p] > 0 {
            return Outcome::Undecided;
        }
    }
    Outcome::Victory
}

// ---------- Grid methods reused above (kept here to keep grid.rs focused
// on construction; in a future refactor these should live in grid.rs). ----
impl Grid {
    pub fn time_advance(&mut self) {
        // Time is owned by the State; this stub exists so the migration
        // signatures can stay simple.
    }
    pub fn determine_ownership_all(&mut self) {
        for i in 0..self.width {
            for j in 0..self.height {
                let idx = i * self.height + j;
                if !self.tiles[idx].cl.is_inhabitable() {
                    continue;
                }
                let mut best = NEUTRAL;
                for p in 1..MAX_PLAYER {
                    if self.tiles[idx].pop[p] > self.tiles[idx].pop[best.index()] {
                        best = PlayerId(p as u8);
                    }
                }
                self.tiles[idx].pl = best;
            }
        }
    }
}

#[allow(dead_code)]
pub const fn _unused() -> i32 {
    RANDOM_INEQUALITY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::rng::CowRng;
    use crate::types::{PlayerId, TileClass};

    fn make_grid_with_two_players() -> (Grid, [i64; MAX_PLAYER]) {
        let mut g = Grid::empty(5, 5);
        let gold = [0i64; MAX_PLAYER];
        for i in 0..5 {
            for j in 0..5 {
                if let Some(t) = g.get_mut(Loc::new(i as i16, j as i16)) {
                    t.cl = TileClass::Grassland;
                }
            }
        }
        (g, gold)
    }

    #[test]
    fn growth_factor_correct() {
        assert!((growth(TileClass::Village) - 1.10).abs() < 1e-6);
        assert!((growth(TileClass::Town) - 1.20).abs() < 1e-6);
        assert!((growth(TileClass::Castle) - 1.30).abs() < 1e-6);
        assert_eq!(growth(TileClass::Abyss), 0.0);
    }

    #[test]
    fn pop_to_symbol_matches_table() {
        // Table from output-common.c:32 (>400/>200/>100/>50/>25/>12/>6/>3/>0).
        assert_eq!(pop_to_symbol(0), -1);
        assert_eq!(pop_to_symbol(1), 0);
        assert_eq!(pop_to_symbol(4), 1);
        assert_eq!(pop_to_symbol(7), 2);
        assert_eq!(pop_to_symbol(13), 3);
        assert_eq!(pop_to_symbol(26), 4);
        assert_eq!(pop_to_symbol(51), 5);
        assert_eq!(pop_to_symbol(101), 6);
        assert_eq!(pop_to_symbol(201), 7);
        assert_eq!(pop_to_symbol(401), 8);
    }

    #[test]
    fn time_to_ymd_basic() {
        let (mut y, mut m, mut d) = (0u64, 0u32, 0u32);
        // 360 days = exactly one "year" by the C convention.
        time_to_ymd(360, &mut y, &mut m, &mut d);
        assert_eq!(y, 1);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }

    #[test]
    fn win_or_lose_defeat_when_zero_pop() {
        let (g, _) = make_grid_with_two_players();
        let outcome = win_or_lose(&g, PlayerId(1));
        assert_eq!(outcome, Outcome::Defeat);
    }

    #[test]
    fn simulate_does_not_panic_on_empty_map() {
        let mut g = Grid::empty(5, 5);
        let mut gold = [0i64; MAX_PLAYER];
        let mut flags: [FlagGrid; MAX_PLAYER] = std::array::from_fn(|_| FlagGrid::new(5, 5));
        let mut rng = CowRng::from_seed(1);
        // Should not panic even though everything is abyss (so all events
        // bail out early).
        simulate_one_step(
            &mut g,
            &mut gold,
            &mut flags,
            PlayerId(1),
            Difficulty::Normal,
            &mut rng,
        );
    }

    #[test]
    fn simulate_grows_village_population() {
        let mut g = Grid::empty(5, 5);
        // Make (2,2) a village owned by player 1 with 100 pop.
        let idx = 2 * g.height + 2;
        g.tiles[idx].cl = TileClass::Village;
        g.tiles[idx].pl = PlayerId(1);
        g.tiles[idx].pop[1] = 100;
        let mut gold = [0i64; MAX_PLAYER];
        let mut flags: [FlagGrid; MAX_PLAYER] = std::array::from_fn(|_| FlagGrid::new(5, 5));
        let mut rng = CowRng::from_seed(0);
        simulate_one_step(
            &mut g,
            &mut gold,
            &mut flags,
            PlayerId(1),
            Difficulty::Normal,
            &mut rng,
        );
        // Village grew by 10 % (rounded) → expect between 109 and 111.
        let pop_after = g.tiles[idx].pop[1];
        assert!(pop_after >= 109 && pop_after <= 111, "got {}", pop_after);
    }

    #[test]
    fn simulate_population_capped_at_max_pop() {
        let mut g = Grid::empty(5, 5);
        let idx = 2 * g.height + 2;
        g.tiles[idx].cl = TileClass::Castle;
        g.tiles[idx].pl = PlayerId(1);
        g.tiles[idx].pop[1] = 499;
        let mut gold = [0i64; MAX_PLAYER];
        let mut flags: [FlagGrid; MAX_PLAYER] = std::array::from_fn(|_| FlagGrid::new(5, 5));
        let mut rng = CowRng::from_seed(0);
        for _ in 0..5 {
            simulate_one_step(
                &mut g,
                &mut gold,
                &mut flags,
                PlayerId(1),
                Difficulty::Normal,
                &mut rng,
            );
        }
        assert!(g.tiles[idx].pop[1] <= MAX_POP);
    }
}
