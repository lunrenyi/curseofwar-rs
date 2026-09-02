//! ratatui-based terminal rendering for the `curseofwar` binary.
//!
//! The map is drawn with absolute positioning to a `Buffer` (no Layout
//! slicing), matching the C source's `POSY`/`POSX` formulas from output.c:26-27.
//! Status/help/dialog/timeline renderers are all in their own modules.

pub mod dialog;
pub mod help;
pub mod map;
pub mod status;
pub mod timeline;

use cow_core::state::State;
use cow_core::ui::UiState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::i18n::Lang;

/// A bundled frame context — passed around to all renderers so they can
/// translate strings and colourise by player.
#[derive(Clone, Copy)]
#[allow(dead_code)] // `k` is reserved for future flag-flicker animation.
pub struct FrameCtx<'a> {
    pub lang: &'a Lang,
    pub ui: &'a UiState,
    pub k: u32, // animation counter (C: main.c:74-78)
}

/// Map player id to a colour, faithfully matching `player_color`
/// (output.c:30-42). The order is preserved exactly because the C code uses
/// it to look up `init_pair(2..=8)`.
pub fn player_color(p: u8) -> Color {
    match p {
        0 => Color::Yellow, // NEUTRAL
        1 => Color::Green,  // player
        2 => Color::Blue,
        3 => Color::Red,
        4 => Color::Yellow,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::DarkGray, // Black on black — bold makes it visible
        _ => Color::Reset,
    }
}

pub fn player_style(p: u8) -> Style {
    let mut s = Style::default().fg(player_color(p));
    if p != 0 {
        s = s.add_modifier(ratatui::style::Modifier::BOLD);
    }
    s
}

/// Top-level widget that draws the map, then optionally the status / help /
/// timeline below. The dialog (when active) is drawn last so it overlays
/// everything else.
pub struct GameFrame<'a> {
    pub state: &'a State,
    pub ui: &'a UiState,
    pub ctx: FrameCtx<'a>,
}

impl<'a> Widget for GameFrame<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        map::draw(self.state, self.ui, self.ctx, area, buf);
        status::draw(self.state, self.ctx, area, buf);
        if self.state.show_timeline {
            timeline::draw(self.state, self.ctx, area, buf);
        }
        help::draw(self.state, self.ctx, area, buf);
    }
}

/// Convenience constructor.
impl<'a> FrameCtx<'a> {
    pub fn new(lang: &'a Lang, ui: &'a UiState, k: u32) -> Self {
        FrameCtx { lang, ui, k }
    }
}

/// Dialog overlay (the y/n quit prompt). Drawn separately so the main frame
/// can still update behind it; we blit a small centred box on top.
pub fn draw_quit_dialog(state: &State, ui: &UiState, lang: &Lang, area: Rect, buf: &mut Buffer) {
    dialog::draw_quit(state, ui, lang, area, buf);
}

/// Outcome text overlays (main.c:54-69). Only drawn when the game has ended.
pub fn draw_outcome_banner(state: &State, lang: &Lang, area: Rect, buf: &mut Buffer) {
    use crate::i18n::TextKey;
    use cow_core::types::Outcome;
    match state.win_or_lose() {
        Outcome::Victory => {
            let y = area.y + area.height.saturating_sub(5);
            let x = area.x + 31;
            buf.set_string(
                x,
                y,
                lang.t(TextKey::YouWon).as_ref(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        }
        Outcome::Defeat => {
            let y = area.y + area.height.saturating_sub(5);
            let x = area.x + 31;
            buf.set_string(
                x,
                y,
                lang.t(TextKey::YouLost).as_ref(),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        }
        Outcome::Undecided => {}
    }
}
