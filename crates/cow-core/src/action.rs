//! Player-action type re-exported from `state::Action` for convenient use
//! outside the core. The TUI builds an `Action` from a keystroke and calls
//! `State::apply`; a future multiplayer server would deserialise an
//! `Action` straight off the wire.

pub use crate::state::Action;
