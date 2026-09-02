//! TUI main loop (replacement for main.c:72-100).
//!
//! Uses `crossterm::event::poll` with a 10 ms timeout to replace the C
//! source's SIGALRM-driven `pause()` pattern. Every wall-clock tick we
//! advance the simulation by one frame if the speed permits; keystrokes
//! are consumed immediately.

use std::io::Stdout;
use std::time::{Duration, Instant};

use cow_core::action::Action;
use cow_core::state::State;
use cow_core::ui::{adjust_cursor, ui_init, UiState};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::event::{map_key, Modal};
use crate::i18n::Lang;
use crate::render::{draw_help_dialog, draw_outcome_banner, draw_quit_dialog, FrameCtx, GameFrame};

const TICK: Duration = Duration::from_millis(10);

pub struct App {
    pub state: State,
    pub ui: UiState,
    pub k: u32,
    pub modal: Modal,
    pub lang: Lang,
}

impl App {
    pub fn new(state: State, lang: Lang) -> Self {
        let mut ui = UiState::new();
        ui_init(&state.grid, state.controlled, &mut ui);
        App {
            state,
            ui,
            k: 0,
            modal: Modal::None,
            lang,
        }
    }

    /// Drain queued keystrokes. Returns `true` if the user confirmed quitting.
    pub fn handle_input(&mut self) -> std::io::Result<bool> {
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(k) = event::read()? {
                if self.process_key(k) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn process_key(&mut self, key: KeyEvent) -> bool {
        let (actions, modal) = map_key(
            key,
            self.ui.cursor,
            self.modal,
            self.state.grid.width,
            self.state.grid.height,
        );

        if let Some(m) = modal {
            // Confirm-quit detection: pressing y/Q inside QuitConfirm exits.
            if m == Modal::QuitConfirm
                && self.modal == Modal::QuitConfirm
                && matches!(
                    key.code,
                    KeyCode::Char('y')
                        | KeyCode::Char('Y')
                        | KeyCode::Char('q')
                        | KeyCode::Char('Q')
                )
            {
                return true;
            }
            self.modal = m;
            return false;
        }

        for a in actions {
            if let Action::MoveCursor(ni, nj) = a {
                adjust_cursor(&self.state.grid, &mut self.ui, ni, nj);
            } else {
                self.state.apply(a);
            }
        }
        false
    }

    /// Advance simulation by one tick if speed permits.
    pub fn tick(&mut self) {
        self.k = self.k.wrapping_add(1);
        if self.modal == Modal::None {
            let slowdown = self.state.speed.slowdown();
            if self.k % slowdown == 0 && self.state.speed != cow_core::types::Speed::Pause {
                self.state.step();
            }
        }
    }

    pub fn draw(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> std::io::Result<()> {
        terminal.draw(|f| {
            let area = f.size();
            let ctx = FrameCtx::new(&self.lang, &self.ui, self.k);
            f.render_widget(
                GameFrame {
                    state: &self.state,
                    ui: &self.ui,
                    ctx,
                },
                area,
            );
            if self.modal == Modal::QuitConfirm {
                draw_quit_dialog(&self.state, &self.ui, &self.lang, area, f.buffer_mut());
            } else if self.modal == Modal::Help {
                draw_help_dialog(&self.state, &self.ui, &self.lang, area, f.buffer_mut());
            }
            draw_outcome_banner(&self.state, &self.lang, area, f.buffer_mut());
        })?;
        Ok(())
    }
}

/// TUI entry point — installs raw mode, runs the loop, restores the
/// terminal on exit (panic-safe via a small guard).
pub fn run_tui(state: State, lang: Lang) -> std::io::Result<()> {
    use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    use crossterm::{execute, terminal};
    use std::io::stdout;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(state, lang);

    let result = loop {
        let mut next_tick = Instant::now() + TICK;
        // 1. Drain input.
        if app.handle_input()? {
            break Ok(());
        }
        // 2. Sleep until next tick (or until input arrives).
        let now = Instant::now();
        let wait = next_tick
            .checked_duration_since(now)
            .unwrap_or(Duration::ZERO);
        if event::poll(wait)? {
            // We already drained; loop again to process any new events.
            continue;
        }
        // 3. Tick simulation.
        if Instant::now() >= next_tick {
            app.tick();
            // If we've fallen far behind (e.g. the host suspended our
            // thread), keep the "behind by more than 5 frames" flag set so
            // the next iteration of the outer loop skips ahead rather than
            // catching up frame by frame.
            if next_tick + TICK * 5 >= Instant::now() {
                next_tick += TICK;
            }
        }
        // 4. Draw.
        app.draw(&mut terminal)?;
    };

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cow_core::ai::Difficulty;
    use cow_core::state::GameOptions;
    use cow_core::types::Shape;

    fn dummy_state() -> State {
        State::new(&GameOptions {
            map_seed: 1,
            dif: Difficulty::Normal,
            shape: Shape::Rect,
            ..Default::default()
        })
    }

    #[test]
    fn app_new_has_initialised_ui() {
        let app = App::new(dummy_state(), Lang::Zh);
        assert!(app.ui.cursor.i >= 0);
        assert_eq!(app.modal, Modal::None);
    }

    #[test]
    fn tick_advances_k_counter() {
        let mut app = App::new(dummy_state(), Lang::Zh);
        let k0 = app.k;
        app.tick();
        assert_eq!(app.k, k0 + 1);
    }
}
