//! Curse of War (1.3.0) terminal front-end — Chinese by default, English
//! selectable. Re-uses the game logic from `cow-core`.
//!
//! Curse of War -- Real Time Strategy Game for Linux.
//! Copyright (C) 2013 Alexey Nikolaev (original C implementation).
//!
//! Licensed under the GNU General Public License v3.0 or later. See the
//! root `LICENSE` file for the full text.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

mod app;
mod cli;
mod event;
mod i18n;
mod render;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
