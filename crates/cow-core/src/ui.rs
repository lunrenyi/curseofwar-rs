//! Cursor / view state (state.h:37-46, state.c:147-203).
//!
//! Lives separately from `State` because the cursor moves in response to
//! player input and is otherwise irrelevant to the simulation. The TUI owns
//! the actual `UiState` instance; `State` only needs to know how to clamp
//! a candidate cursor position to a visible tile.

use crate::grid::Grid;
use crate::types::{Loc, NEUTRAL};

/// View state (state.h:37).
#[derive(Clone, Debug)]
pub struct UiState {
    pub cursor: Loc,
    pub xskip: i32, // (state.h:39) number of tiles to skip at the start of every line
    pub xlength: i32, // (state.h:40) total max number of tiles in horizontal direction
}

impl UiState {
    pub fn new() -> Self {
        UiState {
            cursor: Loc::new(0, 0),
            xskip: 0,
            xlength: 0,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// `ui_init` (state.c:147). Pick the cursor starting position (the tile
/// where the controlled player has the most population), and compute the
/// horizontal scroll such that every visible tile fits.
pub fn ui_init(grid: &Grid, controlled: crate::types::PlayerId, ui: &mut UiState) {
    let mut best = (Loc::new(0, 0), 0i32);
    for (l, t) in grid.iter() {
        if let Some(pop) = t.pop.get(controlled.index()) {
            if *pop > best.1 {
                best = (l, *pop);
            }
        }
    }
    ui.cursor = best.0;

    // xskip/xlength: in the C source these are derived from the hex layout.
    // Each tile at (i, j) sits at "column" i*2 + j, so the visible range is
    // computed by sweeping every visible tile.
    let mut x_skip = i32::MAX;
    let mut x_right = i32::MIN;
    for (l, t) in grid.iter() {
        if !t.cl.is_visible() {
            continue;
        }
        let x = (l.i as i32) * 2 + (l.j as i32);
        if x < x_skip {
            x_skip = x;
        }
        if x > x_right {
            x_right = x;
        }
    }
    ui.xskip = x_skip / 2;
    ui.xlength = ((x_right + 1) / 2) - (x_skip / 2);
}

/// `adjust_cursor` (state.c:177). Given a desired cursor position, find the
/// closest visible tile by the original fallback chain.
pub fn adjust_cursor(grid: &Grid, ui: &mut UiState, mut cursi: i16, mut cursj: i16) {
    cursi = cursi.clamp(0, grid.width as i16 - 1);
    cursj = cursj.clamp(0, grid.height as i16 - 1);
    let in_bounds = |i: i16, j: i16| -> bool {
        i >= 0 && j >= 0 && (i as usize) < grid.width && (j as usize) < grid.height
    };
    let visible = |i: i16, j: i16| -> bool {
        grid.get(Loc::new(i, j))
            .map(|t| t.cl.is_visible())
            .unwrap_or(false)
    };
    // 1. Exact target.
    if visible(cursi, cursj) {
        ui.cursor = Loc::new(cursi, cursj);
        return;
    }
    // 2. Same row, current i.
    if in_bounds(ui.cursor.i, cursj) && visible(ui.cursor.i, cursj) {
        ui.cursor.j = cursj;
        return;
    }
    // 3. Same row, neighbour i (i-1 first, then i+1).
    for &di in &[-1i16, 1i16] {
        let i = ui.cursor.i + di;
        if in_bounds(i, cursj) && visible(i, cursj) {
            ui.cursor.i = i;
            ui.cursor.j = cursj;
            return;
        }
    }
    // 4. Fallback: search the entire grid for any visible tile. This branch
    // covers edge cases where the cursor is currently on abyss and the target
    // row is entirely abyss (e.g. a hex stencil with very few visible rows).
    for i in 0..grid.width as i16 {
        for j in 0..grid.height as i16 {
            if visible(i, j) {
                ui.cursor = Loc::new(i, j);
                return;
            }
        }
    }
    // Give up; cursor stays where it was.
    let _ = NEUTRAL;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::rng::CowRng;
    use crate::types::{PlayerId, TileClass};

    #[test]
    fn ui_init_picks_top_pop_tile_as_cursor() {
        let mut g = Grid::empty(5, 5);
        let idx = 1 * g.height + 1;
        g.tiles[idx].cl = TileClass::Grassland;
        g.tiles[idx].pl = PlayerId(1);
        g.tiles[idx].pop[1] = 50;
        let mut ui = UiState::new();
        ui_init(&g, PlayerId(1), &mut ui);
        assert_eq!(ui.cursor, Loc::new(1, 1));
    }

    #[test]
    fn adjust_cursor_clamps_to_visible_tile() {
        let mut g = Grid::empty(5, 5);
        // Only (3,3) is visible (grassland); everything else abyss.
        let idx = 3 * g.height + 3;
        g.tiles[idx].cl = TileClass::Grassland;
        let mut ui = UiState::new();
        ui_init(&g, PlayerId(1), &mut ui);
        // Try to move to (0,0) — abyss. The fallback should keep the cursor
        // on (3,3) or move to a visible neighbour.
        adjust_cursor(&g, &mut ui, 0, 0);
        assert!(g.get(ui.cursor).unwrap().cl.is_visible());
    }

    #[test]
    fn adjust_cursor_moves_when_target_visible() {
        let mut g = Grid::empty(5, 5);
        for i in 0..5 {
            for j in 0..5 {
                if let Some(t) = g.get_mut(Loc::new(i as i16, j as i16)) {
                    t.cl = TileClass::Grassland;
                }
            }
        }
        let mut ui = UiState::new();
        ui_init(&g, PlayerId(1), &mut ui);
        adjust_cursor(&g, &mut ui, 4, 4);
        assert_eq!(ui.cursor, Loc::new(4, 4));
    }

    #[allow(dead_code)]
    fn _force_rng(_r: CowRng) {}
}
