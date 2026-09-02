//! Core data types (newtypes and enums).
//!
//! Faithful re-implementation of the C structs from `grid.h`/`state.h`/`king.h`,
//! reshaped into Rust idioms.

use crate::consts::{DIRECTIONS, MAX_PLAYER};

/// A player id. Always 0 (`NEUTRAL`) or 1..=7. The newtype prevents accidental
/// usage of raw `i32` that could be -1, etc.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct PlayerId(pub u8);

/// Neutral player (common.h:25, grid.c / state.c references throughout).
pub const NEUTRAL: PlayerId = PlayerId(0);

impl PlayerId {
    /// Construct from a raw id without range checking (used internally where
    /// indices into `pop: [i32; MAX_PLAYER]` are needed). The `as usize`
    /// conversion is intentional and safe given how callers use it.
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// True for non-neutral players (1..=7).
    #[inline]
    pub fn is_country(self) -> bool {
        self.0 >= 1 && (self.0 as usize) < MAX_PLAYER
    }
}

/// Coordinate of a tile (grid.h:95). We use `i16` so that arithmetic with
/// neighbour offsets (some of which are -1) stays in signed arithmetic; the
/// caller must still bound-check before indexing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Loc {
    pub i: i16,
    pub j: i16,
}

impl Loc {
    #[inline]
    pub const fn new(i: i16, j: i16) -> Self {
        Loc { i, j }
    }
}

/// The six neighbour offsets of the hex grid (grid.c:47). Order is identical
/// to the original `dirs[]` so iteration-dependent code (mine ownership,
/// migration rotation, floodfill) behaves identically.
pub const DIRS: [Loc; DIRECTIONS] = [
    Loc { i: -1, j: 0 },
    Loc { i: 1, j: 0 },
    Loc { i: 0, j: -1 },
    Loc { i: 0, j: 1 },
    Loc { i: 1, j: -1 },
    Loc { i: -1, j: 1 },
];

/// Tile terrain class (grid.h:52). Order matches the original `enum tile_class`
/// so that C-code-comments and tests can talk about the same indices.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum TileClass {
    Abyss = 0,
    Mountain = 1,
    Mine = 2,
    Grassland = 3,
    Village = 4,
    Town = 5,
    Castle = 6,
}

impl TileClass {
    /// `is_a_city(t)` (grid.h:56) — true for village, town, castle.
    #[inline]
    pub fn is_city(self) -> bool {
        matches!(
            self,
            TileClass::Village | TileClass::Town | TileClass::Castle
        )
    }

    /// `is_inhabitable(t)` (grid.h:60) — true for grassland + cities.
    #[inline]
    pub fn is_inhabitable(self) -> bool {
        !matches!(
            self,
            TileClass::Abyss | TileClass::Mountain | TileClass::Mine
        )
    }

    /// `is_visible(t)` (grid.h:62) — true for everything but abyss.
    #[inline]
    pub fn is_visible(self) -> bool {
        !matches!(self, TileClass::Abyss)
    }
}

/// Game speed (common.h:46). Eight levels from pause to fastest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Speed {
    Pause,
    Slowest,
    Slower,
    Slow,
    Normal,
    Fast,
    Faster,
    Fastest,
}

impl Speed {
    /// `game_slowdown(speed)` (main-common.c:123) — how many render ticks
    /// between simulation steps.
    pub fn slowdown(self) -> u32 {
        match self {
            Speed::Pause => 1,
            Speed::Slowest => 160,
            Speed::Slower => 80,
            Speed::Slow => 40,
            Speed::Normal => 20,
            Speed::Fast => 10,
            Speed::Faster => 5,
            Speed::Fastest => 2,
        }
    }

    /// `faster(s)` (state.c:24) — move one step faster, capped at Fastest.
    pub fn faster(self) -> Speed {
        match self {
            Speed::Pause => Speed::Slowest,
            Speed::Slowest => Speed::Slower,
            Speed::Slower => Speed::Slow,
            Speed::Slow => Speed::Normal,
            Speed::Normal => Speed::Fast,
            Speed::Fast => Speed::Faster,
            Speed::Faster | Speed::Fastest => Speed::Fastest,
        }
    }

    /// `slower(s)` (state.c:36) — move one step slower, capped at Pause.
    pub fn slower(self) -> Speed {
        match self {
            Speed::Fastest => Speed::Faster,
            Speed::Faster => Speed::Fast,
            Speed::Fast => Speed::Normal,
            Speed::Normal => Speed::Slow,
            Speed::Slow => Speed::Slower,
            Speed::Slower => Speed::Slowest,
            Speed::Slowest | Speed::Pause => Speed::Pause,
        }
    }
}

/// Game difficulty (common.h:49). Five levels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Difficulty {
    Easiest,
    Easy,
    Normal,
    Hard,
    Hardest,
}

/// Map shape / stencil (grid.h:64). Controls how the play area is carved and
/// how many starting locations are available (4 / 4 / 6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shape {
    Rhombus,
    Rect,
    Hex,
}

impl Shape {
    /// `stencil_avlbl_loc_num` (grid.c:100).
    pub fn avlbl_loc_num(self) -> usize {
        match self {
            Shape::Rhombus => 4,
            Shape::Rect => 4,
            Shape::Hex => 6,
        }
    }
}

/// Outcome of the game after each periodic re-check (`win_or_lose`,
/// main-common.c:93).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    /// You are the only country with population left.
    Victory,
    /// Your population is zero.
    Defeat,
    /// Neither — keep playing.
    Undecided,
}
