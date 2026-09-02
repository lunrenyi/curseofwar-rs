//! Map crossterm keystrokes onto `cow_core::Action` values.
//!
//! Includes the y/n quit-confirmation modal: pressing `q` doesn't quit
//! immediately; instead it sets the modal which freezes the simulation
//! until the user confirms (y/Q/Esc) or cancels (n/N).

use cow_core::action::Action;
use cow_core::types::Loc;
use crossterm::event::{KeyCode, KeyEvent};

/// Application-level mode. QuitConfirm freezes simulation until the user
/// decides; otherwise the game runs normally.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modal {
    None,
    QuitConfirm,
}

/// Translate a single key event into a list of actions. Some keys emit
/// nothing (e.g. ESC in normal mode).
pub fn map_key(
    key: KeyEvent,
    cursor: Loc,
    modal: Modal,
    max_w: usize,
    max_h: usize,
) -> (Vec<Action>, Option<Modal>) {
    match modal {
        Modal::QuitConfirm => {
            match key.code {
                KeyCode::Char('y')
                | KeyCode::Char('Y')
                | KeyCode::Char('q')
                | KeyCode::Char('Q') => {
                    return (vec![], Some(Modal::QuitConfirm)); // signal "confirmed"
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    return (vec![], Some(Modal::None));
                }
                _ => return (vec![], Some(Modal::QuitConfirm)),
            }
        }
        Modal::None => {}
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            // Enter quit-confirmation modal.
            return (vec![], Some(Modal::QuitConfirm));
        }
        KeyCode::Char('h') | KeyCode::Left => {
            let ni = (cursor.i - 1).max(0);
            return (vec![Action::MoveCursor(ni as i16, cursor.j)], None);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let ni = (cursor.i + 1).min(max_w as i16 - 1);
            return (vec![Action::MoveCursor(ni as i16, cursor.j)], None);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let nj = (cursor.j - 1).max(0);
            let mut ni = cursor.i;
            // Hex offset (C: main-common.c:343-347): odd new j shifts +1.
            if nj % 2 == 1 {
                ni = (ni + 1).min(max_w as i16 - 1);
            }
            return (vec![Action::MoveCursor(ni, nj)], None);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let nj = (cursor.j + 1).min(max_h as i16 - 1);
            let mut ni = cursor.i;
            // Even new j shifts -1.
            if nj % 2 == 0 {
                ni = (ni - 1).max(0);
            }
            return (vec![Action::MoveCursor(ni, nj)], None);
        }
        KeyCode::Char(' ') => {
            return (vec![Action::ToggleFlagAtCursor(cursor)], None);
        }
        KeyCode::Char('x') => return (vec![Action::ClearAllFlags], None),
        KeyCode::Char('c') => return (vec![Action::ClearHalfFlags], None),
        KeyCode::Char('r') | KeyCode::Char('v') => {
            return (vec![Action::BuildAtCursor(cursor)], None);
        }
        KeyCode::Char('f') => return (vec![Action::SpeedUp], None),
        KeyCode::Char('s') => return (vec![Action::SlowDown], None),
        KeyCode::Char('p') => return (vec![Action::TogglePause], None),
        KeyCode::Esc => {
            // M2: ESC in normal mode is ignored, mirroring the C source
            // (main-common.c:370-371).
            return (vec![Action::Noop], None);
        }
        _ => return (vec![Action::Noop], None),
    }
}
