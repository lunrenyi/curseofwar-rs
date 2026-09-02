//! Map rendering (output.c — ascii hex map).
//!
//! Implements the same POSY/POSX positioning used by the original C source,
//! so the output lines up column-for-column with the C version's rendering.

use cow_core::state::State;
use cow_core::types::{TileClass, NEUTRAL};
use cow_core::ui::UiState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::{player_style, FrameCtx};

/// ASCII glyphs that match the original C source character-for-character.
/// We keep these as static slices so the Rust rendering is byte-identical
/// to the C version at every position.
const MOUNTAIN_GLYPH: &str = " /\\^ ";
const MINE_GLYPH: &str = " /$\\ ";
const GRASSLAND_GLYPH: &str = "  -  ";
const VILLAGE_GLYPH: &str = "  n  ";
const TOWN_GLYPH: &str = " i=i ";
const CASTLE_GLYPH: &str = " W#W ";

/// `POSY` (output.c:26). Row of tile (i, j) = `j + 1`.
#[inline]
fn posy(j: i16) -> i16 {
    j + 1
}

/// `POSX` (output.c:27). Column of tile (i, j) = `4*i + 2*j + 1 - 4*xskip`.
#[inline]
fn posx(i: i16, j: i16, xskip: i32) -> i16 {
    4 * i + 2 * j + 1 - 4 * (xskip as i16)
}

/// `pop_to_symbol` (output-common.c:32) — duplicated here to keep the
/// renderer self-contained. Returns the 3-char ASCII density glyph.
fn pop_glyph(num: i32) -> &'static str {
    if num > 400 {
        ":::"
    } else if num > 200 {
        ".::"
    } else if num > 100 {
        " ::"
    } else if num > 50 {
        ".:."
    } else if num > 25 {
        ".: "
    } else if num > 12 {
        " : "
    } else if num > 6 {
        "..."
    } else if num > 3 {
        ".. "
    } else if num > 0 {
        " . "
    } else {
        ""
    }
}

fn terrain_glyph(cl: TileClass) -> &'static str {
    match cl {
        TileClass::Mountain => MOUNTAIN_GLYPH,
        TileClass::Mine => MINE_GLYPH,
        TileClass::Grassland => GRASSLAND_GLYPH,
        TileClass::Village => VILLAGE_GLYPH,
        TileClass::Town => TOWN_GLYPH,
        TileClass::Castle => CASTLE_GLYPH,
        TileClass::Abyss => "",
    }
}

pub fn draw(state: &State, ui: &UiState, _ctx: FrameCtx<'_>, area: Rect, buf: &mut Buffer) {
    for i in 0..state.grid.width {
        for j in 0..state.grid.height {
            let l = cow_core::types::Loc::new(i as i16, j as i16);
            let Some(tile) = state.grid.get(l) else {
                continue;
            };
            let y = area.y + (posy(j as i16) as u16);
            let x = area.x + (posx(i as i16, j as i16, ui.xskip) as u16);
            if y >= area.y + area.height || x >= area.x + area.width {
                continue;
            }
            // Terrain.
            let glyph = terrain_glyph(tile.cl);
            let style = match tile.cl {
                TileClass::Mountain | TileClass::Grassland => Style::default().fg(Color::Green),
                TileClass::Mine => Style::default().fg(Color::Yellow),
                TileClass::Village | TileClass::Town | TileClass::Castle => player_style(tile.pl.0),
                TileClass::Abyss => Style::default(),
            };
            if !glyph.is_empty() && x >= 1 {
                buf.set_string(x - 1, y, glyph, style);
            }
            // Population glyph on grassland only (C: output.c:143-147).
            if matches!(tile.cl, TileClass::Grassland) {
                let mut total = 0i32;
                for p in 0..cow_core::consts::MAX_PLAYER {
                    total += tile.pop[p];
                }
                let g = pop_glyph(total);
                if !g.is_empty() {
                    buf.set_string(x, y, g, player_style(tile.pl.0));
                }
            }
            // Enemy AI flags (small 'x' on the left side of the tile).
            for p in 0..cow_core::consts::MAX_PLAYER {
                if p as u8 == state.controlled.0 {
                    continue;
                }
                if state.flags[p].flag_at(l) {
                    buf.set_string(x, y, "x", player_style(p as u8));
                }
            }
            // Player flag (uppercase 'P' on the right).
            if state.flags[state.controlled.index()].flag_at(l) {
                buf.set_string(
                    x + 2,
                    y,
                    "P",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            }
        }
    }

    // Cursor brackets: '(' on the left of (i,j), ')' on the left of (i+1, j).
    let cy = area.y + (posy(ui.cursor.j as i16) as u16);
    let cx = area.x + (posx(ui.cursor.i, ui.cursor.j, ui.xskip) as u16);
    if cy < area.y + area.height && cx >= area.x + 1 {
        buf.set_string(
            cx - 1,
            cy,
            "(",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    }
    if (ui.cursor.i as usize) + 1 < state.grid.width {
        let ny = cy;
        let nx = area.x + (posx(ui.cursor.i + 1, ui.cursor.j, ui.xskip) as u16);
        if ny < area.y + area.height && nx >= area.x + 1 {
            buf.set_string(
                nx - 1,
                ny,
                ")",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
    let _ = NEUTRAL;
}
