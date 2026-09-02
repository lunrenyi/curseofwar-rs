//! Status bar (output.c:194-255) — gold, prices, speed, date, population
//! at the cursor. All strings come from the i18n table.

use cow_core::consts::MAX_PLAYER;
use cow_core::sim::time_to_ymd;
use cow_core::state::State;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::{player_style, FrameCtx};
use crate::i18n::TextKey;

pub fn draw(state: &State, ctx: FrameCtx<'_>, area: Rect, buf: &mut Buffer) {
    let lang = ctx.lang;
    let y_base = area.y + state.grid.height as u16 + 2;
    if y_base >= area.y + area.height {
        return;
    }

    // Row 0: gold + date.
    let gold_label = lang.t(TextKey::LabelGold);
    buf.set_string(
        area.x,
        y_base,
        format!("  {} ", gold_label).as_str(),
        Style::default().fg(Color::White),
    );
    let gold_str = format!("{:6}", state.gold[state.controlled.index()]);
    buf.set_string(
        area.x + gold_label.chars().count() as u16 + 3,
        y_base,
        &gold_str,
        player_style(state.controlled.0),
    );

    let (mut y, mut m, mut d) = (0u64, 0u32, 0u32);
    time_to_ymd(state.time, &mut y, &mut m, &mut d);
    let date = format!("{:04}-{:02}-{:02}", y, m, d);
    let date_label = lang.t(TextKey::LabelDate);
    buf.set_string(
        area.x + 65,
        y_base,
        format!("{} ", date_label).as_str(),
        Style::default().fg(Color::White),
    );
    buf.set_string(
        area.x + 65 + date_label.chars().count() as u16 + 1,
        y_base,
        &date,
        player_style(state.controlled.0),
    );

    // Row 1: prices + population header.
    let prices = lang.t(TextKey::LabelPrices);
    buf.set_string(
        area.x,
        y_base + 1,
        format!(" {} ", prices).as_str(),
        Style::default().fg(Color::White),
    );

    let pop_header = lang.t(TextKey::LabelPopulationAtCursor);
    let pop_x = area.x + 30;
    buf.set_string(
        pop_x,
        y_base + 1,
        format!("  {} ", pop_header).as_str(),
        Style::default().fg(Color::White),
    );

    // Row 2: speed + per-player counts.
    let speed_label = lang.t(TextKey::LabelSpeed);
    buf.set_string(
        area.x + 1,
        y_base + 2,
        format!("{} ", speed_label).as_str(),
        Style::default().fg(Color::White),
    );
    let speed_text = lang.t(TextKey::SpeedName(state.speed.into()));
    let padded = format!("{:<6}", speed_text);
    buf.set_string(
        area.x + 1 + speed_label.chars().count() as u16 + 1,
        y_base + 2,
        &padded,
        player_style(state.controlled.0),
    );

    let cursor = state.grid.get(ctx.ui.cursor);
    for p in 1..MAX_PLAYER {
        let x = pop_x + p as u16 * 5;
        if x + 3 > area.x + area.width {
            break;
        }
        let pop = cursor.map(|t| t.pop[p]).unwrap_or(0);
        let s = format!("{:3}", pop);
        buf.set_string(x, y_base + 2, &s, player_style(p as u8));
    }

    // Outcome overlay (also drawn in mod.rs — kept here so the status bar
    // is self-contained for snapshot tests).
    use cow_core::types::Outcome;
    match state.win_or_lose() {
        Outcome::Victory => {
            let x = area.x + 31;
            buf.set_string(
                x,
                y_base,
                lang.t(TextKey::YouWon).as_ref(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            );
        }
        Outcome::Defeat => {
            let x = area.x + 31;
            buf.set_string(
                x,
                y_base,
                lang.t(TextKey::YouLost).as_ref(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            );
        }
        Outcome::Undecided => {}
    }
}
