//! Key-binding help block (output.c:225-233). All strings from i18n.

use cow_core::state::State;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use super::{player_style, FrameCtx};
use crate::i18n::TextKey;

pub fn draw(state: &State, ctx: FrameCtx<'_>, area: Rect, buf: &mut Buffer) {
    let lang = ctx.lang;
    let y_base = area.y + state.grid.height as u16 + 5;
    if y_base >= area.y + area.height {
        return;
    }
    let player = player_style(state.controlled.0);
    let txt = Style::default().fg(Color::White);

    let entries1: [(TextKey, TextKey); 3] = [
        (TextKey::HelpFlagKey, TextKey::HelpAddRemoveFlag),
        (TextKey::HelpClearAllKey, TextKey::HelpClearAllFlags),
        (TextKey::HelpSpeedUpKey, TextKey::HelpSpeedUp),
    ];
    let entries2: [(TextKey, TextKey); 3] = [
        (TextKey::HelpBuildKey, TextKey::HelpBuild),
        (TextKey::HelpClearHalfKey, TextKey::HelpClearHalfFlags),
        (TextKey::HelpSlowDownKey, TextKey::HelpSlowDown),
    ];
    let entries3: [(TextKey, TextKey); 3] = [
        (TextKey::HelpQuitKey, TextKey::HelpQuit),
        (TextKey::Noop, TextKey::Noop),
        (TextKey::HelpPauseKey, TextKey::HelpPause),
    ];

    let column_x: [u16; 3] = [area.x + 1, area.x + 30, area.x + 57];

    let draw_row = |row: &[(TextKey, TextKey); 3], y: u16, buf: &mut Buffer| {
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
    };

    draw_row(&entries1, y_base, buf);
    draw_row(&entries2, y_base + 1, buf);
    draw_row(&entries3, y_base + 2, buf);
}

#[cfg(test)]
mod tests {}
