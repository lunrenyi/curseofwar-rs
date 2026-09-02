//! Quit-confirmation dialog (main.c:258-276, output.c:258-276).

use cow_core::state::State;
use cow_core::ui::UiState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::i18n::{Lang, TextKey};

pub fn draw_quit(_state: &State, _ui: &UiState, lang: &Lang, area: Rect, buf: &mut Buffer) {
    let cx = area.x + area.width / 2 - 9;
    let cy = area.y + area.height / 2 - 2;
    let style = Style::default().fg(Color::White).bg(Color::Black);
    if cy + 3 < area.y + area.height {
        buf.set_string(cx, cy, "                 ", style);
        buf.set_string(cx, cy + 1, lang.t(TextKey::QuitPrompt).as_ref(), style);
        buf.set_string(cx, cy + 2, lang.t(TextKey::QuitHint).as_ref(), style);
        buf.set_string(cx, cy + 3, "                 ", style);
    }
}
