//! Key-binding help block (output.c:225-233). All strings from i18n.

use cow_core::state::State;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use super::{player_style, FrameCtx};
use crate::i18n::TextKey;

/// The 3x3 key-binding table shared between the bottom help block and the
/// `?` overlay. Each row is `(key_label, description)`; `TextKey::Noop`
/// entries leave a cell blank.
pub fn key_table_rows() -> [[(TextKey, TextKey); 3]; 3] {
    [
        [
            (TextKey::HelpFlagKey, TextKey::HelpAddRemoveFlag),
            (TextKey::HelpClearAllKey, TextKey::HelpClearAllFlags),
            (TextKey::HelpSpeedUpKey, TextKey::HelpSpeedUp),
        ],
        [
            (TextKey::HelpBuildKey, TextKey::HelpBuild),
            (TextKey::HelpClearHalfKey, TextKey::HelpClearHalfFlags),
            (TextKey::HelpSlowDownKey, TextKey::HelpSlowDown),
        ],
        [
            (TextKey::HelpQuitKey, TextKey::HelpQuit),
            (TextKey::Noop, TextKey::Noop),
            (TextKey::HelpPauseKey, TextKey::HelpPause),
        ],
    ]
}

pub fn draw(state: &State, ctx: FrameCtx<'_>, area: Rect, buf: &mut Buffer) {
    let lang = ctx.lang;
    let y_base = area.y + state.grid.height as u16 + 5;
    if y_base >= area.y + area.height {
        return;
    }
    let player = player_style(state.controlled.0);
    let txt = Style::default().fg(Color::White);

    let rows = key_table_rows();
    let column_x: [u16; 3] = [area.x + 1, area.x + 30, area.x + 57];

    for (r, row) in rows.iter().enumerate() {
        let y = y_base + r as u16;
        for (col, (k_key, d_key)) in row.iter().enumerate() {
            if matches!(k_key, TextKey::Noop) {
                continue;
            }
            let key_text = lang.t(k_key.clone());
            let desc_text = lang.t(d_key.clone());
            let x = column_x[col];
            buf.set_string(x, y, key_text.as_ref(), player);
            let label_x = x + UnicodeWidthStr::width(key_text.as_ref()) as u16 + 1;
            buf.set_string(label_x, y, desc_text.as_ref(), txt);
        }
    }
}

#[cfg(test)]
mod tests {}
