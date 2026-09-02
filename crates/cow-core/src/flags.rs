//! Per-player flag grid and the spread/even operators that maintain the
//! "call" (attraction) values.
//!
//! Faithful re-implementation of `struct flag_grid`, `add_flag`,
//! `remove_flag`, `remove_flags_with_prob`, and the `spread` / `even`
//! helpers from grid.h/grid.c.

use crate::consts::{MAX_HEIGHT, MAX_WIDTH};
use crate::grid::Grid;
use crate::rng::CowRng;
use crate::types::{Loc, DIRS};

/// A per-player flag overlay on top of the map (grid.h:159).
///
/// * `flag[i*h+j]` is `true` when the player has placed a flag at that tile.
/// * `call[i*h+j]` is the player's attraction field there (can be negative
///   after a removal that subtracts from prior placements).
#[derive(Clone, Debug)]
pub struct FlagGrid {
    pub width: usize,
    pub height: usize,
    pub flag: Vec<bool>,
    pub call: Vec<i32>,
}

impl FlagGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.min(MAX_WIDTH);
        let height = height.min(MAX_HEIGHT);
        FlagGrid {
            width,
            height,
            flag: vec![false; width * height],
            call: vec![0; width * height],
        }
    }

    #[inline]
    pub fn in_bounds(&self, l: Loc) -> bool {
        l.i >= 0 && l.j >= 0 && (l.i as usize) < self.width && (l.j as usize) < self.height
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        i * self.height + j
    }

    #[inline]
    pub fn flag_at(&self, l: Loc) -> bool {
        if !self.in_bounds(l) {
            return false;
        }
        self.flag[self.idx(l.i as usize, l.j as usize)]
    }

    #[inline]
    pub fn call_at(&self, l: Loc) -> i32 {
        if !self.in_bounds(l) {
            return 0;
        }
        self.call[self.idx(l.i as usize, l.j as usize)]
    }
}

/// `add_flag` (grid.c:526). Places a flag of power `val` at `l,` then spreads
/// `val/2` (truncating) to each inhabitable neighbour, recursively.
pub fn add_flag(g: &Grid, fg: &mut FlagGrid, l: Loc, val: i32) {
    if !fg.in_bounds(l)
        || !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false)
        || fg.flag_at(l)
    {
        return;
    }
    let mut u = vec![0i32; fg.flag.len()];
    let idx = (l.i as usize) * fg.height + (l.j as usize);
    fg.flag[idx] = true;
    spread(g, &mut u, &mut fg.call, l, val, 1);
}

/// `remove_flag` (grid.c:545). Mirrors `add_flag` but with `factor = -1` so
/// the call field is decremented rather than incremented.
pub fn remove_flag(g: &Grid, fg: &mut FlagGrid, l: Loc, val: i32) {
    if !fg.in_bounds(l)
        || !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false)
        || !fg.flag_at(l)
    {
        return;
    }
    let mut u = vec![0i32; fg.flag.len()];
    let idx = (l.i as usize) * fg.height + (l.j as usize);
    fg.flag[idx] = false;
    spread(g, &mut u, &mut fg.call, l, val, -1);
}

/// `remove_flags_with_prob` (grid.c:564). Iterates every tile and removes
/// each existing flag with probability `prob` (the C code uses `<=` so a
/// `prob` of 1.0 removes everything).
pub fn remove_flags_with_prob(rng: &mut CowRng, g: &Grid, fg: &mut FlagGrid, prob: f32) {
    let mut to_remove: Vec<Loc> = Vec::new();
    for i in 0..fg.width {
        for j in 0..fg.height {
            if fg.flag[i * fg.height + j] && rng.unit() <= prob {
                to_remove.push(Loc::new(i as i16, j as i16));
            }
        }
    }
    for l in to_remove {
        remove_flag(g, fg, l, 8); // FLAG_POWER == 8
    }
}

/// `spread` (grid.c:500). Recursive decay spread: at each cell visited the
/// call value is increased by `(val - u[x][y]) * factor`, then the recursion
/// passes `val/2` (integer-divided) down to each inhabitable neighbour.
///
/// `u` acts as the "previously credited at this cell" accumulator: it tracks
/// how much call has already been added by earlier recursion branches at the
/// same tile, so the increment is `d * factor` rather than `val * factor`.
pub fn spread(g: &Grid, u: &mut [i32], v: &mut [i32], l: Loc, val: i32, factor: i32) {
    if !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false) {
        return;
    }
    let h = g.height;
    let idx = (l.i as usize) * h + (l.j as usize);
    let d = val - u[idx];
    if d > 0 {
        let cur = v[idx];
        let incr = d * factor;
        v[idx] = if incr < 0 {
            (cur + incr).max(0)
        } else {
            cur + incr
        };
        u[idx] += d;
        let next_val = val / 2;
        if next_val <= 0 {
            return;
        }
        for dir in DIRS.iter() {
            let n = Loc::new(l.i + dir.i, l.j + dir.j);
            spread(g, u, v, n, next_val, factor);
        }
    }
}

/// `even` (grid.c:515). Re-initialise the entire spread map at `val` for
/// every tile in the connected component. The original function is used to
/// reset the `u` scratch array between `spread` runs; we re-implement it
/// for completeness even though the Rust `spread` uses a fresh scratch
/// buffer each call.
#[allow(dead_code)]
pub fn even(g: &Grid, v: &mut [i32], l: Loc, val: i32) {
    if !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false) {
        return;
    }
    let h = g.height;
    let idx = (l.i as usize) * h + (l.j as usize);
    if v[idx] == val {
        return;
    }
    v[idx] = val;
    for dir in DIRS.iter() {
        let n = Loc::new(l.i + dir.i, l.j + dir.j);
        even(g, v, n, val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::rng::CowRng;
    use crate::types::TileClass;

    fn flat_grid(w: usize, h: usize) -> Grid {
        let g = Grid::empty(w, h);
        // We need inhabitable tiles for flags to be placeable.
        g
    }

    fn make_grid_with_grass(w: usize, h: usize) -> Grid {
        // Build a grid where every tile is grassland so flags can be placed.
        let mut g = Grid::empty(w, h);
        for i in 0..w {
            for j in 0..h {
                if let Some(t) = g.get_mut(Loc::new(i as i16, j as i16)) {
                    t.cl = TileClass::Grassland;
                }
            }
        }
        g
    }

    #[test]
    fn add_then_remove_restores_call_zero() {
        let mut g = make_grid_with_grass(7, 7);
        let mut fg = FlagGrid::new(7, 7);
        let l = Loc::new(3, 3);
        add_flag(&g, &mut fg, l, 8);
        // After add, some tiles should have positive call.
        let mut sum: i32 = fg.call.iter().sum();
        assert!(sum > 0, "expected positive call sum after add, got {}", sum);
        remove_flag(&g, &mut fg, l, 8);
        sum = fg.call.iter().sum();
        assert_eq!(
            sum, 0,
            "call field should be exactly 0 after add+remove, got {}",
            sum
        );
    }

    #[test]
    fn add_then_remove_specific_tile_call_returns_to_zero() {
        // After removing the flag the exact tile where we placed it must
        // come back to 0 (because we used the same value with factor=-1).
        let mut g = make_grid_with_grass(7, 7);
        let mut fg = FlagGrid::new(7, 7);
        let l = Loc::new(3, 3);
        add_flag(&g, &mut fg, l, 8);
        remove_flag(&g, &mut fg, l, 8);
        assert_eq!(fg.call_at(l), 0);
        assert!(!fg.flag_at(l));
    }

    #[test]
    fn remove_flags_with_prob_one_removes_all() {
        let mut g = make_grid_with_grass(5, 5);
        let mut rng = CowRng::from_seed(1);
        let mut fg = FlagGrid::new(5, 5);
        for i in 0..5 {
            for j in 0..5 {
                add_flag(&g, &mut fg, Loc::new(i as i16, j as i16), 8);
            }
        }
        let mut sum: i32 = fg.flag.iter().map(|&b| if b { 1 } else { 0 }).sum();
        assert_eq!(sum, 25);
        remove_flags_with_prob(&mut rng, &g, &mut fg, 1.0);
        sum = fg.flag.iter().map(|&b| if b { 1 } else { 0 }).sum();
        assert_eq!(sum, 0);
        let call_sum: i32 = fg.call.iter().sum();
        assert_eq!(call_sum, 0);
    }

    #[test]
    fn add_flag_on_uninhabitable_tile_does_nothing() {
        let g = Grid::empty(5, 5); // all abyss
        let mut fg = FlagGrid::new(5, 5);
        add_flag(&g, &mut fg, Loc::new(2, 2), 8);
        assert!(!fg.flag_at(Loc::new(2, 2)));
        assert_eq!(fg.call_at(Loc::new(2, 2)), 0);
    }

    #[test]
    fn double_add_only_registers_once() {
        let mut g = make_grid_with_grass(5, 5);
        let mut fg = FlagGrid::new(5, 5);
        let l = Loc::new(2, 2);
        add_flag(&g, &mut fg, l, 8);
        let sum_after_first: i32 = fg.call.iter().sum();
        add_flag(&g, &mut fg, l, 8); // should be no-op
        let sum_after_second: i32 = fg.call.iter().sum();
        assert_eq!(sum_after_first, sum_after_second);
    }

    #[allow(dead_code)]
    fn _unused_flat() {
        let _ = flat_grid(3, 3);
    }
}
