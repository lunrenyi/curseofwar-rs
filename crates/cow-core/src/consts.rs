//! Global constants from the original C source (common.h).
//!
//! Every constant here mirrors `common.h` lines 24–37 of the original
//! curseofwar-1.3.0 source. Do not change values without reading the
//! "Faithful re-implementation" section of PLAN.md.

#![allow(dead_code)]

/// `MAX_PLAYER` (common.h:24) — number of players (countries). Indices 1..=7 are
/// real countries; index 0 is the `NEUTRAL` placeholder used throughout the
/// simulation (mines start neutral, uninhabited tiles have no owner, etc.).
pub const MAX_PLAYER: usize = 8;

/// Neutral player id (common.h:25). Exported as `types::NEUTRAL`.
/// (Re-exported for convenience here too.)
pub const NEUTRAL_ID: u8 = 0;

/// `MAX_CLASS` (common.h:26) — unit classes. Only citizens exist.
pub const MAX_CLASS: usize = 1;

/// `MAX_WIDTH` (common.h:27) — maximum map width.
pub const MAX_WIDTH: usize = 40;

/// `MAX_HEIGHT` (common.h:28) — maximum map height.
pub const MAX_HEIGHT: usize = 29;

/// `DIRECTIONS` (common.h:29) — number of neighbours per hex tile.
pub const DIRECTIONS: usize = 6;

/// `MAX_POP` (common.h:31) — population cap per (tile, player).
pub const MAX_POP: i32 = 499;

/// `MAX_TIMELINE_MARK` (common.h:33) — ring-buffer size of timeline samples.
pub const MAX_TIMELINE_MARK: usize = 72;

/// Power of an individual flag when placed (grid.h:30 in original).
pub const FLAG_POWER: i32 = 8;

/// Building costs (king.h:33-35).
pub const PRICE_VILLAGE: i64 = 160;
pub const PRICE_TOWN: i64 = 240;
pub const PRICE_CASTLE: i64 = 320;

/// Sentinel meaning "no inequality constraint" (grid.h:32).
pub const RANDOM_INEQUALITY: i32 = -1;

/// Maximum number of starting locations supported by any stencil (grid.h:33).
pub const MAX_AVLBL_LOC: usize = 7;

pub mod keys {
    //! Keycodes used by the original game (common.h:39-43). Only consumed by
    //! the TUI layer; here for reference.

    pub const ESCAPE: u8 = 0x1B;
    pub const K_UP: u8 = 65;
    pub const K_DOWN: u8 = 66;
    pub const K_RIGHT: u8 = 67;
    pub const K_LEFT: u8 = 68;
}
