//! Map grid: terrain, ownership, and per-tile population buckets.
//!
//! Faithful re-implementation of `struct grid`/`struct tile` from grid.h and
//! `grid_init`/`conflict`/`is_connected` from grid.c. See PLAN.md for the
//! "Faithful re-implementation" section and the quirk checklist.

#[cfg(test)]
use crate::consts::MAX_POP;
use crate::consts::{
    DIRECTIONS, MAX_AVLBL_LOC, MAX_HEIGHT, MAX_PLAYER, MAX_WIDTH, RANDOM_INEQUALITY,
};
use crate::rng::CowRng;
use crate::types::{Loc, PlayerId, Shape, TileClass, DIRS, NEUTRAL};

/// One tile of the map (grid.h:81). Population is a flat `[i32; MAX_PLAYER]`
/// (one bucket per player, only citizens — `MAX_CLASS == 1`).
#[derive(Clone, Debug)]
pub struct Tile {
    pub cl: TileClass,
    pub pl: PlayerId,
    pub pop: [i32; MAX_PLAYER],
}

impl Tile {
    fn new(cl: TileClass, pl: PlayerId) -> Self {
        Tile {
            cl,
            pl,
            pop: [0; MAX_PLAYER],
        }
    }
}

/// The map (grid.h:108). Tiles are stored row-major as `tiles[i*height + j]`,
/// matching the C `tiles[i][j]` layout exactly (so orientation-sensitive code
/// such as the migration scan and the mine-ownership loop behave identically).
#[derive(Clone, Debug)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    /// Public within the crate so the simulator can iterate without bouncing
    /// through a getter. The struct stays opaque to anything outside.
    pub tiles: Vec<Tile>,
}

impl Grid {
    /// Build an empty grid (all abyss, no population). Useful for tests.
    pub fn empty(width: usize, height: usize) -> Self {
        let width = width.min(MAX_WIDTH);
        let height = height.min(MAX_HEIGHT);
        let tiles = (0..width * height)
            .map(|_| Tile::new(TileClass::Abyss, NEUTRAL))
            .collect();
        Grid {
            width,
            height,
            tiles,
        }
    }

    /// Linear index lookup. Returns `None` for negative / out-of-range coords.
    pub fn get(&self, l: Loc) -> Option<&Tile> {
        if l.i < 0 || l.j < 0 {
            return None;
        }
        let (i, j) = (l.i as usize, l.j as usize);
        if i >= self.width || j >= self.height {
            return None;
        }
        let idx = i * self.height + j;
        self.tiles.get(idx)
    }

    pub fn get_mut(&mut self, l: Loc) -> Option<&mut Tile> {
        if l.i < 0 || l.j < 0 {
            return None;
        }
        let (i, j) = (l.i as usize, l.j as usize);
        if i >= self.width || j >= self.height {
            return None;
        }
        let idx = i * self.height + j;
        self.tiles.get_mut(idx)
    }

    #[inline]
    pub fn in_bounds(&self, l: Loc) -> bool {
        l.i >= 0 && l.j >= 0 && (l.i as usize) < self.width && (l.j as usize) < self.height
    }

    /// Iterate tiles in row-major order (i ascending, j ascending within i).
    pub fn iter(&self) -> impl Iterator<Item = (Loc, &Tile)> {
        let h = self.height;
        self.tiles.iter().enumerate().map(move |(k, t)| {
            let i = k / h;
            let j = k % h;
            (Loc::new(i as i16, j as i16), t)
        })
    }

    /// `grid_init` (grid.c:49) — populate terrain and a random owner per tile.
    ///
    /// - 1/20 of tiles become cities (1/6 castle, 2/6 town, 3/6 village).
    /// - Of the remaining tiles, 4/20 (x in 1..=4) become mountains or mines
    ///   (1/10 chance for a mine).
    /// - Other tiles get a random owner in 1..=7. Cities start with 10 pop.
    pub fn generate_random_terrain(&mut self, rng: &mut CowRng) {
        for i in 0..self.width {
            for j in 0..self.height {
                let x = rng.below(20);
                let (cl, pl) = if x == 0 {
                    let y = rng.below(6);
                    match y {
                        0 => (TileClass::Castle, random_owner(rng)),
                        1 | 2 => (TileClass::Town, random_owner(rng)),
                        _ => (TileClass::Village, random_owner(rng)),
                    }
                } else if x > 0 && x < 5 {
                    // Mountains and mines (grid.c:68-75): 1/10 of these become mines.
                    // Owner stays NEUTRAL (the C code explicitly sets it there).
                    let cl = if rng.below(10) == 0 {
                        TileClass::Mine
                    } else {
                        TileClass::Mountain
                    };
                    (cl, NEUTRAL)
                } else {
                    (TileClass::Grassland, random_owner(rng))
                };

                let idx = i * self.height + j;
                if cl.is_city() {
                    // grid.c:90-93: cities start with 10 population in their owner's bucket.
                    let mut t = Tile::new(cl, pl);
                    t.pop[pl.index()] = 10;
                    self.tiles[idx] = t;
                } else {
                    self.tiles[idx] = Tile::new(cl, pl);
                }
            }
        }
    }

    /// Apply the chosen stencil, carving the play area out and producing up to
    /// `MAX_AVLBL_LOC` candidate starting locations. Mirrors
    /// `apply_stencil` (grid.c:177) which dispatches to the three shape
    /// functions.
    pub fn apply_stencil(&mut self, shape: Shape) -> Vec<Loc> {
        match shape {
            Shape::Rhombus => self.stencil_rhombus(),
            Shape::Rect => self.stencil_rect(),
            Shape::Hex => self.stencil_hex(),
        }
    }

    /// Diamond layout (grid.c:112). Loc num = 4.
    fn stencil_rhombus(&mut self) -> Vec<Loc> {
        let d: i16 = 2;
        let locs = vec![
            Loc::new(d, d),
            Loc::new(self.width as i16 - 1 - d, self.height as i16 - 1 - d),
            Loc::new(d, self.height as i16 - 1 - d),
            Loc::new(self.width as i16 - 1 - d, d),
        ];
        // Rhombus doesn't carve out any tiles, just returns the four corners.
        locs
    }

    /// Rectangular layout (grid.c:123). Cuts corners off with epsilon=0.1.
    fn stencil_rect(&mut self) -> Vec<Loc> {
        const EPSILON: f32 = 0.1;
        let x0 = 0.5 * (self.height as f32 - 1.0) - EPSILON;
        let y0 = -EPSILON;
        let x1 = 0.5 * (self.height as f32 - 1.0) + (self.width as f32 - 1.0) + EPSILON;
        let y1 = (self.height as f32 - 1.0) + EPSILON;
        let x_of = |i: i16, j: i16| -> f32 { 0.5 * (j as f32) + (i as f32) };
        let y_of = |_i: i16, j: i16| -> f32 { j as f32 };

        for i in 0..self.width {
            for j in 0..self.height {
                let x = x_of(i as i16, j as i16);
                let y = y_of(i as i16, j as i16);
                if x < x0 || x > x1 || y < y0 || y > y1 {
                    let idx = i * self.height + j;
                    let t = &mut self.tiles[idx];
                    t.cl = TileClass::Abyss;
                    t.pl = NEUTRAL;
                    t.pop = [0; MAX_PLAYER];
                }
            }
        }

        let dx = self.height / 2;
        let d: i16 = 2;
        vec![
            Loc::new(dx as i16 + d - 1, d),
            Loc::new(
                self.width as i16 - dx as i16 - 1 - d + 1,
                self.height as i16 - 1 - d,
            ),
            Loc::new(d + 1, self.height as i16 - 1 - d),
            Loc::new(self.width as i16 - 1 - d - 1, d),
        ]
    }

    /// Hexagonal layout (grid.c:153). Loc num = 6.
    fn stencil_hex(&mut self) -> Vec<Loc> {
        let dx = self.height / 2;
        let w = self.width as i16;
        let h = self.height as i16;
        let dx_i = dx as i16;
        for i in 0..self.width {
            for j in 0..self.height {
                let ii = i as i16;
                let jj = j as i16;
                if ii + jj < dx_i || ii + jj > w - 1 + h - 1 - dx_i {
                    let idx = i * self.height + j;
                    let t = &mut self.tiles[idx];
                    t.cl = TileClass::Abyss;
                    t.pl = NEUTRAL;
                    t.pop = [0; MAX_PLAYER];
                }
            }
        }

        let d: i16 = 2;
        vec![
            Loc::new(dx_i + d - 2, d),                 // tl
            Loc::new(d, h - 1 - d),                    // bl
            Loc::new(w - 1 - d, dx_i),                 // cr
            Loc::new(d, dx_i),                         // cl
            Loc::new(w - 1 - d - 2 + 2, d),            // tr
            Loc::new(w - 1 - dx_i - d + 2, h - 1 - d), // br
        ]
    }

    /// `is_connected` (grid.c:468) — flood-fill from the first non-neutral tile
    /// and verify every other owned tile is reached. Abyss tiles are skipped.
    pub fn is_connected(&self) -> bool {
        let mut marked = vec![0i32; self.tiles.len()];
        let mut found_one = false;
        for (idx, t) in self.tiles.iter().enumerate() {
            if t.pl != NEUTRAL {
                if found_one && marked[idx] == 0 {
                    return false;
                }
                found_one = true;
                floodfill(self, &mut marked, idx, 1);
            }
        }
        true
    }
}

fn random_owner(rng: &mut CowRng) -> PlayerId {
    // grid.c:77 — `x = 1 + rand() % (MAX_PLAYER-1)` ⇒ 1..=7
    let x = 1 + rng.below((MAX_PLAYER - 1) as u32);
    PlayerId(x as u8)
}

/// Recursive flood-fill that walks the inhabitable neighbours. Visited cells
/// are tagged by `m[idx]`, so the recursion depth is bounded by the connected
/// component size (≪ 40×29 = 1160 in practice).
fn floodfill(g: &Grid, m: &mut [i32], start: usize, val: i32) {
    if m[start] == val {
        return;
    }
    m[start] = val;
    let h = g.height;
    let i = (start / h) as i16;
    let j = (start % h) as i16;
    for d in DIRS.iter() {
        let ni = i + d.i;
        let nj = j + d.j;
        if ni < 0 || nj < 0 {
            continue;
        }
        let (ni, nj) = (ni as usize, nj as usize);
        if ni >= g.width || nj >= g.height {
            continue;
        }
        let idx = ni * h + nj;
        if g.tiles[idx].cl.is_inhabitable() {
            floodfill(g, m, idx, val);
        }
    }
}

/// Configuration input for [`Grid::apply_conflict`].
pub struct ConflictConfig<'a> {
    /// All candidate starting locations (from the stencil).
    pub loc_arr: &'a [Loc],
    /// Computer-controlled players (their IDs).
    pub comp_players: &'a [PlayerId],
    /// Human-controlled players (always `&[PlayerId(1)]` in single-player).
    pub ui_players: &'a [PlayerId],
    /// Number of starting locations to actually use (clamped to `[2, N]`).
    pub loc_num: usize,
    /// Spawn-quality preference: 0 = random, 1 = best, N = worst.
    pub conditions: i32,
    /// Inequality 0..4 (or `RANDOM_INEQUALITY` = -1 to skip).
    pub inequality: i32,
}

impl<'a> ConflictConfig<'a> {
    pub fn new(
        loc_arr: &'a [Loc],
        comp_players: &'a [PlayerId],
        ui_players: &'a [PlayerId],
        loc_num: usize,
        conditions: i32,
        inequality: i32,
    ) -> Self {
        ConflictConfig {
            loc_arr,
            comp_players,
            ui_players,
            loc_num,
            conditions,
            inequality,
        }
    }
}

impl Grid {
    /// `conflict` (grid.c:311). Returns `Ok` if the placement satisfied all
    /// constraints (including inequality), `Err` if the caller should retry.
    ///
    /// Side effects: clears all cities, places a castle + two mines at each
    /// chosen starting location, and writes population + ownership into the
    /// chosen tiles.
    pub fn apply_conflict<'a>(
        &mut self,
        rng: &mut CowRng,
        cfg: &ConflictConfig<'a>,
    ) -> Result<(), ()> {
        // 1. Wipe existing cities.
        for t in self.tiles.iter_mut() {
            for p in 0..MAX_PLAYER {
                t.pop[p] = 0;
            }
            t.pl = NEUTRAL;
            if t.cl.is_city() {
                t.cl = TileClass::Grassland;
            }
        }

        let avlbl = cfg.loc_arr.len().min(MAX_AVLBL_LOC);
        let loc_num = cfg.loc_num.clamp(2, avlbl);
        if loc_num > cfg.comp_players.len() + cfg.ui_players.len() {
            return Err(());
        }

        // 2. Pick which `loc_num` of the candidate locations to use.
        let di = rng.below(avlbl as u32) as usize;
        let mut chosen: Vec<Loc> = Vec::with_capacity(loc_num);
        let mut i = 0;
        while i < loc_num {
            let ii = (i + di + avlbl) % avlbl;
            let l = cfg.loc_arr[ii];
            chosen.push(l);
            let d = rng.below(DIRECTIONS as u32) as usize;
            let (di_dir, dj_dir) = (DIRS[d].i, DIRS[d].j);

            // grid.c:351-368: place castle + mines. The C code offsets by `m=1`
            // and -2*m relative to a random direction; mine at +m and the
            // neutral grassland at -m. We mirror that exactly.
            self.set_at(l, TileClass::Castle, NEUTRAL);

            // +m
            let mine_a = Loc::new(l.i + di_dir, l.j + dj_dir);
            if let Some(t) = self.get_mut(mine_a) {
                t.cl = TileClass::Mine;
                t.pl = NEUTRAL;
            }
            // -2*m
            let mine_b = Loc::new(l.i - 2 * di_dir, l.j - 2 * dj_dir);
            if let Some(t) = self.get_mut(mine_b) {
                t.cl = TileClass::Mine;
                t.pl = NEUTRAL;
            }
            // -m: neutral grassland (clear whatever was there).
            let grass = Loc::new(l.i - di_dir, l.j - dj_dir);
            if let Some(t) = self.get_mut(grass) {
                t.cl = TileClass::Grassland;
                t.pl = NEUTRAL;
                t.pop = [0; MAX_PLAYER];
            }

            i += 1;
        }

        // 3. Evaluate the locations (floodfill + per-mine value).
        let mut eval_result = [0.0f64; MAX_AVLBL_LOC];
        eval_locations(self, &chosen, &mut eval_result, loc_num);

        // 4. Inequality check (grid.c:381-400). The C version returns -1 if the
        // variance × 1000 / mean falls outside the bucket for `ineq`.
        if cfg.inequality != RANDOM_INEQUALITY {
            let mut avg = 0.0;
            for k in 0..loc_num {
                avg += eval_result[k];
            }
            avg /= loc_num as f64;
            let mut var = 0.0;
            for k in 0..loc_num {
                let d = eval_result[k] - avg;
                var += d * d;
            }
            var /= loc_num as f64;
            let std = var.sqrt();
            let x = std * 1000.0 / avg;
            let ok = match cfg.inequality {
                0 => x <= 50.0,
                1 => x > 50.0 && x <= 100.0,
                2 => x > 100.0 && x <= 250.0,
                3 => x > 250.0 && x <= 500.0,
                4 => x > 500.0,
                _ => true,
            };
            if !ok {
                return Err(());
            }
        }

        // 5. Sort the locations by quality (best first), then assign players.
        let mut indices: Vec<usize> = (0..loc_num).collect();
        indices.sort_by(|&a, &b| eval_result[a].partial_cmp(&eval_result[b]).unwrap());

        // 5a. Shuffle the AI players (grid.c:402-406).
        let mut sh_players_comp: Vec<PlayerId> = cfg.comp_players.to_vec();
        rng.shuffle(&mut sh_players_comp);

        // 5b. Compose `sh_players` of length `num`: first `ui_players_num`
        // humans, then AI players wrapped by `dplayer`.
        let mut sh_players: Vec<PlayerId> = Vec::with_capacity(loc_num);
        let ui_players_num = cfg.ui_players.len();
        for up in cfg.ui_players.iter() {
            sh_players.push(*up);
        }
        if cfg.comp_players.len() > 0 {
            let dplayer = rng.below(cfg.comp_players.len() as u32) as usize;
            for k in ui_players_num..loc_num {
                let src = (k - ui_players_num + dplayer) % cfg.comp_players.len();
                sh_players.push(sh_players_comp[src]);
            }
        }
        rng.shuffle(&mut sh_players);

        // 5c. Pick which location the human goes to (if any).
        let ihuman = if cfg.conditions > 0 {
            let select = (loc_num as i32 - cfg.conditions).clamp(0, loc_num as i32 - 1) as usize;
            indices[select]
        } else {
            rng.below(loc_num as u32) as usize
        };

        // 5d. Assign owners and seed populations.
        for (i_owner, &idx) in indices.iter().enumerate() {
            let l = chosen[idx];
            let pl = if ui_players_num > 1 {
                sh_players[i_owner]
            } else if i_owner == ihuman {
                cfg.ui_players[0]
            } else if i_owner < sh_players_comp.len() {
                sh_players_comp[i_owner]
            } else {
                NEUTRAL
            };
            if let Some(t) = self.get_mut(l) {
                t.pl = pl;
                t.pop[pl.index()] = 10;
            }
        }

        Ok(())
    }

    fn set_at(&mut self, l: Loc, cl: TileClass, pl: PlayerId) {
        if let Some(t) = self.get_mut(l) {
            t.cl = cl;
            t.pl = pl;
        }
    }
}

/// `eval_locations` (grid.c:219). Computes a per-mine value by flood-filling
/// from each starting point, then summing `exp(-…)` for mines whose surrounding
/// tiles are all owned by the same starting player.
fn eval_locations(g: &Grid, chosen: &[Loc], result: &mut [f64], len: usize) {
    // Mark all tiles as "unreachable" (-1) initially.
    const UNREACHABLE: i32 = -1;
    const COMPETITION: i32 = -2;
    let mut u: Vec<i32> = vec![UNREACHABLE; g.tiles.len()];
    let mut d: Vec<i32> = vec![i32::MAX; g.tiles.len()];

    for (k, l) in chosen.iter().enumerate().take(len) {
        floodfill_closest(g, &mut u, &mut d, *l, k as i32, 0);
    }

    for (idx, t) in g.tiles.iter().enumerate() {
        if t.cl != TileClass::Mine {
            continue;
        }
        // Walk the six neighbours of this mine, checking for inhabitants.
        let h = g.height;
        let i = (idx / h) as i16;
        let j = (idx % h) as i16;
        let mut single_owner = UNREACHABLE;
        let mut max_dist = 0;
        let mut min_dist = i32::MAX;
        for dir in DIRS.iter() {
            let ni = i + dir.i;
            let nj = j + dir.j;
            if ni < 0 || nj < 0 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if ni >= g.width || nj >= g.height {
                continue;
            }
            if !g.tiles[ni * h + nj].cl.is_inhabitable() {
                continue;
            }
            let neigh_idx = ni * h + nj;
            let neigh_owner = u[neigh_idx];
            let neigh_dist = d[neigh_idx];
            if single_owner == UNREACHABLE {
                single_owner = neigh_owner;
                max_dist = neigh_dist;
                min_dist = neigh_dist;
            } else if neigh_owner == single_owner {
                if neigh_dist > max_dist {
                    max_dist = neigh_dist;
                }
                if neigh_dist < min_dist {
                    min_dist = neigh_dist;
                }
            } else if neigh_owner != UNREACHABLE {
                single_owner = COMPETITION;
            }
        }
        if single_owner != COMPETITION && single_owner != UNREACHABLE {
            let owner = single_owner as usize;
            let coeff = 100.0 * (MAX_WIDTH as f64 + MAX_HEIGHT as f64);
            let decay = -10.0 * (max_dist as f64) * (min_dist as f64)
                / ((MAX_WIDTH as f64) * (MAX_HEIGHT as f64));
            result[owner] += coeff * decay.exp();
        }
    }
}

/// `floodfill_closest` (grid.c:203) — recursive, with the original
/// `d[x][y] <= dist` early-exit semantic.
fn floodfill_closest(g: &Grid, u: &mut [i32], d: &mut [i32], l: Loc, val: i32, dist: i32) {
    if l.i < 0 || l.j < 0 {
        return;
    }
    let (i, j) = (l.i as usize, l.j as usize);
    if i >= g.width || j >= g.height {
        return;
    }
    let idx = i * g.height + j;
    if !g.tiles[idx].cl.is_inhabitable() || d[idx] <= dist {
        return;
    }
    u[idx] = val;
    d[idx] = dist;
    for dir in DIRS.iter() {
        let n = Loc::new(l.i + dir.i, l.j + dir.j);
        floodfill_closest(g, u, d, n, val, dist + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::CowRng;

    fn fresh() -> Grid {
        Grid::empty(21, 21)
    }

    #[test]
    fn empty_grid_is_all_abyss() {
        let g = fresh();
        assert_eq!(g.width, 21);
        assert_eq!(g.height, 21);
        for (_, t) in g.iter() {
            assert_eq!(t.cl, TileClass::Abyss);
        }
    }

    #[test]
    fn generate_random_terrain_respects_population_cap() {
        let mut g = fresh();
        let mut rng = CowRng::from_seed(42);
        g.generate_random_terrain(&mut rng);
        for (_, t) in g.iter() {
            for &p in &t.pop {
                assert!((0..=MAX_POP).contains(&p));
            }
        }
    }

    #[test]
    fn apply_stencil_rect_carves_corners() {
        let mut g = fresh();
        g.apply_stencil(Shape::Rect);
        // The corners of the rect are expected to be abyss.
        assert_eq!(g.get(Loc::new(0, 0)).unwrap().cl, TileClass::Abyss);
        assert_eq!(
            g.get(Loc::new((g.width - 1) as i16, 0)).unwrap().cl,
            TileClass::Abyss
        );
    }

    #[test]
    fn apply_stencil_hex_returns_six_locs() {
        let mut g = fresh();
        let locs = g.apply_stencil(Shape::Hex);
        assert_eq!(locs.len(), 6);
    }

    #[test]
    fn conflict_produces_connected_grid_for_first_seed() {
        let mut g = fresh();
        let mut rng = CowRng::from_seed(0);
        // Try many seeds — conflict may legitimately fail (inequality check) and
        // the caller is expected to retry. We retry until we succeed or give up.
        for _attempt in 0..50 {
            g.generate_random_terrain(&mut rng);
            let locs = g.apply_stencil(Shape::Rect);
            let ui = [PlayerId(1)];
            let comp = [
                PlayerId(2),
                PlayerId(3),
                PlayerId(4),
                PlayerId(5),
                PlayerId(6),
                PlayerId(7),
            ];
            let cfg = ConflictConfig {
                loc_arr: &locs,
                comp_players: &comp,
                ui_players: &ui,
                loc_num: 4,
                conditions: 0,
                inequality: RANDOM_INEQUALITY,
            };
            // Reset cities step inside conflict() also wipes ownership, so we
            // don't need a separate "post-conflict" reset.
            if g.apply_conflict(&mut rng, &cfg).is_ok() {
                assert!(g.is_connected());
                return;
            }
        }
        // Even after 50 attempts we couldn't get one — possible but unlikely;
        // if this fails something is wrong with the inequality logic.
        panic!("conflict never succeeded in 50 attempts");
    }
}
