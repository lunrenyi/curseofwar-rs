//! Curse of War (1.3.0) Rust re-implementation — core (zero UI/IO).
//!
//! Curse of War -- Real Time Strategy Game for Linux.
//! Copyright (C) 2013 Alexey Nikolaev (original C implementation).
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR.  See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program.  If not, see <http://www.gnu.org/licenses/>.
//!
//! ---
//!
//! See `PLAN.md` in repo root for the faithful-rewrite strategy and the
//! quirk-preservation checklist.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod action;
pub mod ai;
pub mod consts;
pub mod flags;
pub mod grid;
pub mod rng;
pub mod sim;
pub mod state;
pub mod types;
pub mod ui;

pub use consts::*;
pub use rng::CowRng;
pub use types::{Difficulty, Loc, Outcome, PlayerId, Shape, Speed, TileClass, DIRS, NEUTRAL};
