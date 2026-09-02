//! Timeline (`-T` flag). Mirrors output.c:309-437 — a 5-row ASCII chart
//! plotting each player's population over time, with year ticks and a
//! right-side value column sorted by current magnitude.

use cow_core::state::{State, Timeline};
use cow_core::types::PlayerId;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::{player_style, FrameCtx};

const TIMELINE_HEIGHT: u16 = 5;

pub fn draw(state: &State, _ctx: FrameCtx<'_>, area: Rect, buf: &mut Buffer) {
    if !state.show_timeline {
        return;
    }
    let tl = &state.timeline;
    if tl.mark < 0 {
        return;
    }

    let y0 = area.y + state.grid.height as u16 + 9;
    if y0 + TIMELINE_HEIGHT >= area.y + area.height {
        return;
    }
    let x0 = area.x + 2;

    // Determine whether each player has any non-zero data.
    let mut non_zero = [false; cow_core::consts::MAX_PLAYER];
    for p in 1..cow_core::consts::MAX_PLAYER {
        let mut nz = false;
        for i in 0..=tl.mark as usize {
            if tl.data[p][i] >= 0.1 {
                nz = true;
                break;
            }
        }
        non_zero[p] = nz;
    }

    let mut min = 0.0f32;
    let mut max = 0.0f32;
    let mut any = false;
    for p in 1..cow_core::consts::MAX_PLAYER {
        if !non_zero[p] {
            continue;
        }
        if !any {
            max = tl.data[p][0];
            min = max;
            any = true;
        }
        for i in 0..=tl.mark as usize {
            let v = tl.data[p][i];
            if v > max {
                max = v;
            }
            if v < min {
                min = v;
            }
        }
    }
    if !any {
        return;
    }
    if max - min < 0.1 {
        max = min + 0.1;
    }
    let one_over_delta = 1.0 / (max - min);

    // Plot the curve.
    for i in 0..=tl.mark as usize {
        let x = x0 + i as u16;
        if x >= area.x + area.width {
            break;
        }
        let mut pp = 0;
        while pp <= cow_core::consts::MAX_PLAYER {
            let p = if pp < cow_core::consts::MAX_PLAYER {
                ((pp + i) % cow_core::consts::MAX_PLAYER) as u8
            } else {
                state.controlled.0
            };
            if !non_zero[p as usize] && pp < cow_core::consts::MAX_PLAYER {
                pp += 1;
                continue;
            }
            let v = tl.data[p as usize][i];
            let normalized = ((v - min) * one_over_delta).clamp(0.0, 1.0);
            let dy = ((TIMELINE_HEIGHT - 1) as f32 * (1.0 - normalized)) as u16;
            let dy = dy.min(TIMELINE_HEIGHT - 1);
            let y = y0 + dy;
            let ch = if pp == cow_core::consts::MAX_PLAYER {
                '*'
            } else {
                '-'
            };
            let style = if pp == cow_core::consts::MAX_PLAYER {
                Style::default().fg(Color::White)
            } else {
                player_style(p)
            };
            if y < area.y + area.height {
                buf.set_string(x, y, &ch.to_string(), style);
            }
            pp += 1;
        }
    }

    // Min / max labels.
    buf.set_string(
        x0,
        y0,
        &format!("{:.0}", max),
        Style::default().fg(Color::White),
    );
    buf.set_string(
        x0,
        y0 + TIMELINE_HEIGHT - 1,
        &format!("{:.0}", min),
        Style::default().fg(Color::White),
    );

    // Year ticks (use the original `time_to_ymd` extraction).
    use cow_core::sim::time_to_ymd;
    for i in 1..=tl.mark as usize {
        let (mut y1, mut m1, mut d1) = (0u64, 0u32, 0u32);
        let (mut y2, mut m2, mut d2) = (0u64, 0u32, 0u32);
        time_to_ymd(tl.time[i - 1], &mut y1, &mut m1, &mut d1);
        time_to_ymd(tl.time[i], &mut y2, &mut m2, &mut d2);
        if y1 < y2 {
            let label = format!("{}", y2);
            let x = x0 + i as u16;
            if x + label.len() as u16 <= area.x + area.width {
                buf.set_string(
                    x,
                    y0 + TIMELINE_HEIGHT,
                    &label,
                    Style::default().fg(Color::White),
                );
            }
        }
    }

    // Suppress unused import warnings for Timeline/PlayerId.
    let _ = Timeline::default();
    let _ = PlayerId::default();
}
