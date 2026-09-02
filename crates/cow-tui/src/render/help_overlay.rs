//! Full-screen gameplay help overlay shown when `Modal::Help` is active.
//!
//! Centred box drawn on top of the game frame (mirrors `dialog::draw_quit`).
//! All strings come from i18n — see `crate::i18n::TextKey::HelpOverlay*` and
//! the existing `Help*` key/description pairs (reused from `help::key_table_rows`).

use cow_core::state::State;
use cow_core::ui::UiState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

use crate::i18n::{Lang, TextKey};

/// Overlay box dimensions in cells. Must leave room for margins on
/// 80×24-class terminals.
const OVERLAY_W: u16 = 64;
const OVERLAY_H: u16 = 22;

/// One row in the vertical key-binding list. Each entry is `(key, desc)`
/// where `desc == TextKey::Noop` skips the row (used for blank separators).
type KeyRow = (TextKey, TextKey);

/// Draw the overlay. Falls back silently on too-small terminals — the
/// game keeps running underneath, matching the safety net in
/// `dialog::draw_quit`.
pub fn draw_help(_state: &State, _ui: &UiState, lang: &Lang, area: Rect, buf: &mut Buffer) {
    if area.width < OVERLAY_W + 4 || area.height < OVERLAY_H + 4 {
        return;
    }
    let cx = area.x + area.width / 2 - OVERLAY_W / 2;
    let cy = area.y + area.height / 2 - OVERLAY_H / 2;

    let bg = Style::default().fg(Color::White).bg(Color::Black);
    let heading = bg.add_modifier(Modifier::BOLD);

    // Clear the box.
    let blank = " ".repeat(OVERLAY_W as usize);
    for dy in 0..OVERLAY_H {
        buf.set_string(cx, cy + dy, &blank, bg);
    }
    // Top/bottom borders.
    let bar = "─".repeat(OVERLAY_W as usize);
    buf.set_string(cx, cy, &bar, heading);
    buf.set_string(cx, cy + OVERLAY_H - 1, &bar, heading);

    // Title (centred).
    let title = lang.t(TextKey::HelpOverlayTitle);
    let title_x = cx + (OVERLAY_W - UnicodeWidthStr::width(title.as_ref()) as u16) / 2;
    buf.set_string(title_x, cy + 1, title.as_ref(), heading);

    // Section: Goal.
    let y = cy + 3;
    buf.set_string(cx + 2, y, lang.t(TextKey::HelpOverlayGoalHeading).as_ref(), heading);
    buf.set_string(cx + 2, y + 1, lang.t(TextKey::HelpOverlayGoalLine).as_ref(), bg);

    // Section: Mechanics.
    let y = cy + 6;
    buf.set_string(cx + 2, y, lang.t(TextKey::HelpOverlayMechanicsHeading).as_ref(), heading);
    buf.set_string(
        cx + 2,
        y + 1,
        lang.t(TextKey::HelpOverlayMechanicsPopulation).as_ref(),
        bg,
    );
    buf.set_string(
        cx + 2,
        y + 2,
        lang.t(TextKey::HelpOverlayMechanicsGold).as_ref(),
        bg,
    );
    buf.set_string(
        cx + 2,
        y + 3,
        lang.t(TextKey::HelpOverlayMechanicsCapture).as_ref(),
        bg,
    );

    // Section: Key bindings (vertical list, reuses existing Help* keys).
    let y = cy + 11;
    buf.set_string(
        cx + 2,
        y,
        lang.t(TextKey::HelpOverlayKeysHeading).as_ref(),
        heading,
    );
    let rows: [KeyRow; 8] = [
        (TextKey::HelpFlagKey, TextKey::HelpAddRemoveFlag),
        (TextKey::HelpBuildKey, TextKey::HelpBuild),
        (TextKey::HelpClearAllKey, TextKey::HelpClearAllFlags),
        (TextKey::HelpClearHalfKey, TextKey::HelpClearHalfFlags),
        (TextKey::HelpQuitKey, TextKey::HelpQuit),
        (TextKey::HelpSpeedUpKey, TextKey::HelpSpeedUp),
        (TextKey::HelpSlowDownKey, TextKey::HelpSlowDown),
        (TextKey::HelpPauseKey, TextKey::HelpPause),
    ];
    for (i, (k_key, d_key)) in rows.iter().enumerate() {
        write_row(lang, cx + 2, y + 1 + i as u16, buf, k_key.clone(), d_key.clone(), bg);
    }

    // Dismiss hint.
    let y = cy + OVERLAY_H - 2;
    let dismiss = lang.t(TextKey::HelpOverlayDismiss);
    let dismiss_x = cx + (OVERLAY_W - UnicodeWidthStr::width(dismiss.as_ref()) as u16) / 2;
    buf.set_string(dismiss_x, y, dismiss.as_ref(), heading);
}

fn write_row(lang: &Lang, x: u16, y: u16, buf: &mut Buffer, key: TextKey, desc: TextKey, style: Style) {
    let key_text = lang.t(key);
    let desc_text = lang.t(desc);
    buf.set_string(x, y, key_text.as_ref(), style);
    let pad_x = x + UnicodeWidthStr::width(key_text.as_ref()) as u16 + 2;
    buf.set_string(pad_x, y, desc_text.as_ref(), style);
}

#[cfg(test)]
mod tests {}
