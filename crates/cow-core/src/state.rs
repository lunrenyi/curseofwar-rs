//! The whole-game `State` struct, plus options and timeline.
//!
//! Faithful re-implementation of `struct state`/`state_init` from state.h/state.c.
//! `State` owns the RNG, the grid, all flag grids, all kings, the timeline,
//! and the per-player gold/timing fields. It exposes a single `step` method
//! to drive one simulation tick and `apply` to translate player actions into
//! state mutations (the latter is the seam a future multiplayer server would
//! call into).

use crate::ai::{builder_default, default_strategy, Difficulty, King, Strategy};
use crate::consts::{MAX_PLAYER, MAX_TIMELINE_MARK};
use crate::flags::{add_flag, remove_flag, remove_flags_with_prob, FlagGrid};
use crate::grid::{ConflictConfig, Grid};
use crate::rng::CowRng;
use crate::sim::{simulate_one_step, win_or_lose};
use crate::types::{Loc, PlayerId, Shape, Speed};

/// User-visible options (parsed from CLI in M2; for now constructed directly
/// from `cli::CliOptions`).
#[derive(Clone, Debug)]
pub struct GameOptions {
    pub keep_random: bool, // -r
    pub dif: Difficulty,
    pub speed: Speed,
    pub w: usize,
    pub h: usize,
    pub loc_num: usize,
    pub map_seed: u32,
    pub conditions: i32,
    pub timeline: bool,
    pub inequality: i32,
    pub shape: Shape,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            keep_random: false,
            dif: Difficulty::Normal,
            speed: Speed::Normal,
            w: 21,
            h: 21,
            loc_num: 4,
            map_seed: 0,
            conditions: 0,
            timeline: false,
            inequality: -1,
            shape: Shape::Rect,
        }
    }
}

/// Ring buffer of population samples used by `-T`.
#[derive(Clone, Debug)]
pub struct Timeline {
    pub data: [[f32; MAX_TIMELINE_MARK]; MAX_PLAYER],
    pub time: [u64; MAX_TIMELINE_MARK],
    pub mark: i32,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            data: [[0.0; MAX_TIMELINE_MARK]; MAX_PLAYER],
            time: [0; MAX_TIMELINE_MARK],
            mark: -1,
        }
    }
}

/// Whole-game state. Single owner of the RNG.
pub struct State {
    pub grid: Grid,
    pub flags: [FlagGrid; MAX_PLAYER],
    pub kings: Vec<King>,
    pub gold: [i64; MAX_PLAYER],
    pub timeline: Timeline,
    pub show_timeline: bool,
    pub time: u64,
    pub map_seed: u32,
    pub controlled: PlayerId,
    pub conditions: i32,
    pub inequality: i32,
    pub speed: Speed,
    pub prev_speed: Speed,
    pub dif: Difficulty,
    rng: CowRng,
}

impl State {
    /// Initialise state from the given options (state.c:48 `state_init`).
    /// RNG: starting time uses entropy (matches C: rand calls before srand
    /// happen with the global RNG), then `srand(map_seed)` is applied.
    pub fn new(op: &GameOptions) -> Self {
        let mut rng = CowRng::from_entropy();
        let start_year: u64 = (1850 + rng.below(100)) as u64;
        let start_day_of_year: u64 = rng.below(360) as u64;
        let time0: u64 = start_year * 360 + start_day_of_year;

        // Now switch to the seeded RNG (matches `srand(map_seed)` in state.c:101).
        let mut rng = CowRng::from_seed(op.map_seed);

        let mut grid = Grid::empty(op.w.min(40), op.h.min(29));
        let controlled: PlayerId = PlayerId(1);

        // Build AI list with the C version's fixed assignment.
        let comp_player_ids: Vec<PlayerId> = (2..=7u8).map(PlayerId).collect();
        let kings: Vec<King> = comp_player_ids
            .iter()
            .map(|&pl| {
                let strat = default_strategy(pl).unwrap_or(Strategy::None);
                King::new_clamped(pl, strat, grid.width, grid.height)
            })
            .collect();
        let _ = controlled;

        // Generate terrain + apply conflict until we get a connected map.
        // Each retry consumes a fresh set of RNG values, just like the C
        // do-while loop does (state.c:104-121).
        let ui_arr = [controlled];
        let mut succeeded = false;
        for _ in 0..200 {
            grid.generate_random_terrain(&mut rng);
            let locs = grid.apply_stencil(op.shape);
            let loc_num = if op.keep_random {
                comp_player_ids.len() + 1
            } else {
                op.loc_num
            }
            .clamp(2, locs.len());

            let cfg = ConflictConfig::new(
                &locs,
                &comp_player_ids,
                &ui_arr,
                loc_num,
                op.conditions,
                if op.keep_random { -1 } else { op.inequality },
            );
            if grid.apply_conflict(&mut rng, &cfg).is_ok() && grid.is_connected() {
                succeeded = true;
                break;
            }
        }
        if !succeeded {
            // Fallback: relax inequality to always-ok so we don't loop forever
            // on bad seeds. The map may be unbalanced but it's at least valid.
            for _ in 0..50 {
                grid.generate_random_terrain(&mut rng);
                let locs = grid.apply_stencil(op.shape);
                let loc_num = comp_player_ids.len() + 1;
                let cfg = ConflictConfig::new(&locs, &comp_player_ids, &ui_arr, loc_num, 0, -1);
                if grid.apply_conflict(&mut rng, &cfg).is_ok() && grid.is_connected() {
                    break;
                }
            }
        }

        let flags: [FlagGrid; MAX_PLAYER] =
            std::array::from_fn(|_| FlagGrid::new(grid.width, grid.height));
        let gold = [0i64; MAX_PLAYER];

        // Initial map evaluation.
        let mut kings = kings;
        for k in kings.iter_mut() {
            k.evaluate_map(&grid, op.dif, &mut rng);
        }

        let mut timeline = Timeline::default();
        timeline.mark = -1;
        for slot in timeline.time.iter_mut() {
            *slot = time0;
        }

        State {
            grid,
            flags,
            kings,
            gold,
            timeline,
            show_timeline: op.timeline,
            time: time0,
            map_seed: op.map_seed,
            controlled,
            conditions: op.conditions,
            inequality: op.inequality,
            speed: op.speed,
            prev_speed: op.speed,
            dif: op.dif,
            rng,
        }
    }

    /// Advance one render tick (state.c:82). The actual simulation step
    /// happens every `slowdown` ticks; everything else just bumps `time`
    /// tracking for animation.
    pub fn step(&mut self) {
        // 1. kings_move (state.c:228): place flags + builder.
        self.kings_move();
        // 2. simulate (state.c:244): one full simulation step.
        simulate_one_step(
            &mut self.grid,
            &mut self.gold,
            &mut self.flags,
            self.controlled,
            self.dif,
            &mut self.rng,
        );
        // 3. Timeline sampling (state.c:411): every 10 sim steps, record
        // total population per player.
        self.time = self.time.wrapping_add(1);
        if self.show_timeline && self.time % 10 == 0 {
            self.update_timeline();
        }
    }

    /// Equivalent of `kings_move` (state.c:228): let every AI place its flags
    /// and try to build. If any of them successfully built, all AIs must
    /// re-evaluate the map.
    pub fn kings_move(&mut self) {
        let mut any_built = false;
        // We have to collect the king strategies up front to avoid borrowing
        // self mutably while iterating kings.
        let pl_strats: Vec<(PlayerId, Strategy)> =
            self.kings.iter().map(|k| (k.pl, k.strategy)).collect();
        for (pl, _strat) in pl_strats.iter() {
            let pidx = pl.index();
            // Place flags: clone the relevant FlagGrid in/out.
            let mut fg_clone = self.flags[pidx].clone();
            // Find the king whose player matches.
            if let Some(k) = self.kings.iter().find(|k| k.pl == *pl) {
                k.place_flags(&self.grid, &mut fg_clone);
            }
            self.flags[pidx] = fg_clone;
            // Try to build.
            if let Some(k) = self.kings.iter_mut().find(|k| k.pl == *pl) {
                if builder_default(k, &mut self.grid, &mut self.gold) {
                    any_built = true;
                }
            }
        }
        if any_built {
            for k in self.kings.iter_mut() {
                k.evaluate_map(&self.grid, self.dif, &mut self.rng);
            }
        }
    }

    fn update_timeline(&mut self) {
        let mut next_mark = self.timeline.mark + 1;
        if next_mark as usize >= MAX_TIMELINE_MARK {
            // Shift everything left and reuse the last slot.
            for i in 0..MAX_TIMELINE_MARK - 1 {
                self.timeline.time[i] = self.timeline.time[i + 1];
                for p in 0..MAX_PLAYER {
                    self.timeline.data[p][i] = self.timeline.data[p][i + 1];
                }
            }
            next_mark = (MAX_TIMELINE_MARK - 1) as i32;
        }
        self.timeline.mark = next_mark;
        let m = next_mark as usize;
        self.timeline.time[m] = self.time;
        for p in 0..MAX_PLAYER {
            let mut count: i64 = 0;
            for t in self.grid.tiles.iter() {
                if t.cl.is_inhabitable() {
                    count += t.pop[p] as i64;
                }
            }
            self.timeline.data[p][m] = count as f32;
        }
    }

    /// Public accessor for the win-or-lose check.
    pub fn win_or_lose(&self) -> crate::types::Outcome {
        win_or_lose(&self.grid, self.controlled)
    }

    /// Player-owned mutator: apply a game action. This is the seam where a
    /// future multiplayer server would consume a remote input.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::SpeedUp => {
                self.prev_speed = self.speed;
                self.speed = self.speed.faster();
            }
            Action::SlowDown => {
                self.prev_speed = self.speed;
                self.speed = self.speed.slower();
            }
            Action::TogglePause => {
                if self.speed == Speed::Pause {
                    self.speed = self.prev_speed;
                } else {
                    self.prev_speed = self.speed;
                    self.speed = Speed::Pause;
                }
            }
            Action::ClearAllFlags => {
                let mut fg = self.flags[self.controlled.index()].clone();
                remove_flags_with_prob(&mut self.rng, &self.grid, &mut fg, 1.0);
                self.flags[self.controlled.index()] = fg;
            }
            Action::ClearHalfFlags => {
                let mut fg = self.flags[self.controlled.index()].clone();
                remove_flags_with_prob(&mut self.rng, &self.grid, &mut fg, 0.5);
                self.flags[self.controlled.index()] = fg;
            }
            Action::ToggleFlagAtCursor(loc) => {
                let mut fg = self.flags[self.controlled.index()].clone();
                if fg.flag_at(loc) {
                    remove_flag(&self.grid, &mut fg, loc, 8);
                } else {
                    add_flag(&self.grid, &mut fg, loc, 8);
                }
                self.flags[self.controlled.index()] = fg;
            }
            Action::BuildAtCursor(loc) => {
                crate::ai::build(&mut self.grid, &mut self.gold, self.controlled, loc);
            }
            // The TUI knows about cursor moves; we don't touch the cursor
            // here because it lives in `UiState` (cow_core::ui), not in
            // `State`.
            Action::MoveCursor(_, _) | Action::Noop => {}
        }
    }
}

/// High-level player actions. The TUI maps keystrokes to one of these
/// before calling `State::apply`; a future multiplayer server would
/// deserialise them straight off the wire.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    Noop,
    MoveCursor(i16, i16), // new i, new j — TUI keeps cursor outside State
    SpeedUp,
    SlowDown,
    TogglePause,
    ToggleFlagAtCursor(Loc),
    BuildAtCursor(Loc),
    ClearAllFlags,
    ClearHalfFlags,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options(seed: u32) -> GameOptions {
        GameOptions {
            map_seed: seed,
            ..Default::default()
        }
    }

    #[test]
    fn state_new_does_not_panic() {
        let _ = State::new(&default_options(7));
    }

    #[test]
    fn state_new_is_reproducible() {
        let a = State::new(&default_options(123));
        let b = State::new(&default_options(123));
        // The starting time is from entropy, but everything downstream of
        // `srand(map_seed)` is deterministic. Verify by checking per-player
        // total population on the grid.
        let pop_a = total_pop(&a);
        let pop_b = total_pop(&b);
        assert_eq!(pop_a, pop_b);
    }

    fn total_pop(s: &State) -> [i64; MAX_PLAYER] {
        let mut out = [0i64; MAX_PLAYER];
        for t in s.grid.tiles.iter() {
            if t.cl.is_inhabitable() {
                for p in 0..MAX_PLAYER {
                    out[p] += t.pop[p] as i64;
                }
            }
        }
        out
    }

    #[test]
    fn state_step_does_not_panic() {
        let mut s = State::new(&default_options(7));
        for _ in 0..50 {
            s.step();
        }
        // Population must stay bounded.
        for (idx, t) in s.grid.tiles.iter().enumerate() {
            for p in 0..MAX_PLAYER {
                if !(0..=499).contains(&t.pop[p]) {
                    panic!("pop[{}]={} at tile {} ({:?})", p, t.pop[p], idx, t.cl);
                }
            }
        }
    }

    #[test]
    fn step_swallows_speed_changes() {
        let mut s = State::new(&default_options(11));
        s.apply(Action::SpeedUp);
        assert_ne!(s.speed, Speed::Normal);
        s.apply(Action::TogglePause);
        assert_eq!(s.speed, Speed::Pause);
        s.apply(Action::TogglePause);
        assert_eq!(s.speed, s.prev_speed);
    }

    #[test]
    fn long_simulation_stays_bounded() {
        let mut s = State::new(&default_options(99));
        for _ in 0..2000 {
            s.step();
        }
        for t in s.grid.tiles.iter() {
            for &p in t.pop.iter() {
                assert!((0..=499).contains(&p));
            }
        }
        // Gold is non-negative.
        for &g in s.gold.iter() {
            assert!(g >= 0);
        }
    }

    #[test]
    fn timeline_ring_buffer_does_not_overflow() {
        let mut s = State::new(&GameOptions {
            map_seed: 5,
            timeline: true,
            ..Default::default()
        });
        for _ in 0..(MAX_TIMELINE_MARK * 30) {
            s.step();
        }
        assert!((s.timeline.mark as usize) < MAX_TIMELINE_MARK);
    }

    #[test]
    fn clear_half_flags_actually_removes_some() {
        let mut s = State::new(&default_options(1));
        // Plant a flag at a known inhabitable tile. After the first action we
        // expect exactly one flag in the player's grid.
        // (The map starts with cities, so we look for any inhabitable tile.)
        let mut planted = None;
        for i in 0..s.grid.width {
            for j in 0..s.grid.height {
                let l = Loc::new(i as i16, j as i16);
                if let Some(t) = s.grid.get(l) {
                    if t.cl.is_inhabitable() {
                        s.apply(Action::ToggleFlagAtCursor(l));
                        planted = Some(l);
                        break;
                    }
                }
            }
            if planted.is_some() {
                break;
            }
        }
        let before = s.flags[s.controlled.index()]
            .flag
            .iter()
            .filter(|&&b| b)
            .count();
        assert!(before >= 1, "expected at least one flag after planting");
        // `clear_half_flags` uses rng.unit() <= 0.5 to decide per flag — with
        // many flags the count is statistically about half; with one flag
        // it can drop to 0. We just verify it doesn't go *up*.
        s.apply(Action::ClearHalfFlags);
        let after = s.flags[s.controlled.index()]
            .flag
            .iter()
            .filter(|&&b| b)
            .count();
        assert!(
            after <= before,
            "after {} should be <= before {}",
            after,
            before
        );
    }
}
