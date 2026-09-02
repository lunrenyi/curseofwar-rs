//! AI kings: map evaluation, builder, and the five flag-placing strategies
//! (king.c re-implementation).
//!
//! The behaviours here follow the C source exactly:
//!
//! * `evaluate_map` walks every tile and diffuses the value field around
//!   castles/towns/villages/mines with strength depending on the king's
//!   `Strategy`.
//! * `builder_default` is shared across all AIs — pick the highest-scoring
//!   owned tile whose 6 neighbours are also owned, then `build`.
//! * `place_flags` dispatches to the strategy-specific functions.

use crate::consts::{FLAG_POWER, MAX_PLAYER, MAX_POP, PRICE_CASTLE, PRICE_TOWN, PRICE_VILLAGE};
use crate::flags::{add_flag, remove_flag, FlagGrid};
use crate::grid::Grid;
use crate::rng::CowRng;
use crate::types::{PlayerId, TileClass, DIRS};

/// Per-king strategy (king.h:64).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Strategy {
    None,
    AggrGreedy,
    OneGreedy,
    PersistentGreedy,
    Opportunist,
    Noble,
    Midas,
}

/// A king controls a single non-human player. Each king has its own
/// per-tile value map that reflects its preferences for territory types
/// (king.h:79).
#[derive(Clone, Debug)]
pub struct King {
    pub pl: PlayerId,
    pub strategy: Strategy,
    pub value: Vec<i32>,
    width: usize,
    height: usize,
}

impl King {
    pub fn new(pl: PlayerId, strategy: Strategy, width: usize, height: usize) -> Self {
        Self::new_clamped(pl, strategy, width, height)
    }

    /// Convenience constructor that clamps the map size and allocates the
    /// value buffer in one call. Use this in the actual game code; the
    /// `new` constructor is mostly for tests that need to peek inside.
    pub fn new_clamped(pl: PlayerId, strategy: Strategy, w: usize, h: usize) -> Self {
        let w = w.min(40);
        let h = h.min(29);
        King {
            pl,
            strategy,
            value: vec![0; w * h],
            width: w,
            height: h,
        }
    }

    /// Re-evaluate the value map (king.c:53). The values are spread around
    /// castles/towns/villages/mines, then optionally degraded by the
    /// difficulty setting.
    pub fn evaluate_map(&mut self, g: &Grid, difficulty: Difficulty, rng: &mut CowRng) {
        let w = self.width;
        let h = self.height;
        // Allocate scratch buffers used by `spread`.
        let mut u: Vec<i32> = vec![0; w * h];

        for i in 0..w {
            for j in 0..h {
                self.value[i * h + j] = 0;
            }
        }

        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                if g.tiles[idx].cl.is_inhabitable() {
                    self.value[idx] += 1;
                    if self.strategy == Strategy::PersistentGreedy {
                        self.value[idx] += 1;
                    }
                }
            }
        }

        // Pass 2: spread values from cities and mines.
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                let l = crate::types::Loc::new(i as i16, j as i16);
                match g.tiles[idx].cl {
                    TileClass::Castle => {
                        let v = if self.strategy == Strategy::Noble {
                            32
                        } else {
                            16
                        };
                        spread_value(g, &mut u, &mut self.value, l, v, 1);
                        even_value(g, &mut u, l, 0);
                    }
                    TileClass::Town => {
                        spread_value(g, &mut u, &mut self.value, l, 8, 1);
                        even_value(g, &mut u, l, 0);
                    }
                    TileClass::Village => {
                        let v = if self.strategy == Strategy::Noble {
                            2
                        } else {
                            4
                        };
                        spread_value(g, &mut u, &mut self.value, l, v, 1);
                        even_value(g, &mut u, l, 0);
                    }
                    TileClass::Mine => {
                        let v = if self.strategy == Strategy::Midas {
                            8
                        } else {
                            4
                        };
                        for d in DIRS.iter() {
                            let nl = crate::types::Loc::new(i as i16 + d.i, j as i16 + d.j);
                            spread_value(g, &mut u, &mut self.value, nl, v, 1);
                            even_value(g, &mut u, nl, 0);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Difficulty dampening.
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                match difficulty {
                    Difficulty::Easiest => {
                        let mut x = self.value[idx] / 4;
                        x += rng.gen_range(-3, 3);
                        self.value[idx] = x.max(0);
                    }
                    Difficulty::Easy => {
                        let mut x = self.value[idx] / 2;
                        x += rng.gen_range(-1, 1);
                        self.value[idx] = x.max(0);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Place a flag grid update according to the king's strategy (king.c:363).
    pub fn place_flags(&self, g: &Grid, fg: &mut FlagGrid) {
        match self.strategy {
            Strategy::AggrGreedy => self.action_aggr_greedy(g, fg),
            Strategy::OneGreedy => self.action_one_greedy(g, fg),
            Strategy::PersistentGreedy => self.action_persistent_greedy(g, fg),
            Strategy::Opportunist => self.action_opportunist(g, fg),
            Strategy::Noble => self.action_noble(g, fg),
            Strategy::Midas | Strategy::None => {
                // midas / none do not place flags.
            }
        }
    }

    fn action_aggr_greedy(&self, g: &Grid, fg: &mut FlagGrid) {
        let w = self.width;
        let h = self.height;
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                if fg.flag[idx] {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    remove_flag(g, fg, l, FLAG_POWER);
                }
                let (army, enemy) = army_enemy(g, idx, self.pl);
                let val = self.value[idx] as f32;
                let v = val * (2.0 * enemy as f32 - army as f32) * (army as f32).sqrt();
                if v > 5000.0 {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    add_flag(g, fg, l, FLAG_POWER);
                }
            }
        }
    }

    fn action_one_greedy(&self, g: &Grid, fg: &mut FlagGrid) {
        let w = self.width;
        let h = self.height;
        let mut best_v = -1.0f32;
        let mut best_l: Option<crate::types::Loc> = None;
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                if fg.flag[idx] {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    remove_flag(g, fg, l, FLAG_POWER);
                }
                let (army, enemy) = army_enemy(g, idx, self.pl);
                let val = self.value[idx] as f32;
                let v = val * (5.0 * enemy as f32 - army as f32) * (army as f32).sqrt();
                if v > 5000.0 && v > best_v {
                    best_v = v;
                    best_l = Some(crate::types::Loc::new(i as i16, j as i16));
                }
            }
        }
        if let Some(l) = best_l {
            add_flag(g, fg, l, FLAG_POWER);
        }
    }

    fn action_persistent_greedy(&self, g: &Grid, fg: &mut FlagGrid) {
        let w = self.width;
        let h = self.height;
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                let (army, enemy) = army_enemy(g, idx, self.pl);
                let val = self.value[idx] as f32;
                let v1 = val * (2.5 * enemy as f32 - army as f32) * (army as f32).powf(0.7);
                let v2_raw = val
                    * (MAX_POP as f32 - (enemy as f32 - army as f32))
                    * (army as f32).powf(0.7)
                    * 0.5;
                let v2 = if enemy <= army { -10000.0 } else { v2_raw };
                let v = v1.max(v2);
                let l = crate::types::Loc::new(i as i16, j as i16);
                if fg.flag[idx] {
                    if v < 1000.0 {
                        remove_flag(g, fg, l, FLAG_POWER);
                    }
                } else if v > 9000.0 {
                    add_flag(g, fg, l, FLAG_POWER);
                }
            }
        }
    }

    fn action_opportunist(&self, g: &Grid, fg: &mut FlagGrid) {
        let w = self.width;
        let h = self.height;
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                if fg.flag[idx] {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    remove_flag(g, fg, l, FLAG_POWER);
                }
                let (army, enemy) = army_enemy(g, idx, self.pl);
                let val = self.value[idx] as f32;
                let v =
                    val * (MAX_POP as f32 - (enemy as f32 - army as f32)) * (army as f32).sqrt();
                if enemy > army && v > 7000.0 {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    add_flag(g, fg, l, FLAG_POWER);
                }
            }
        }
    }

    fn action_noble(&self, g: &Grid, fg: &mut FlagGrid) {
        const MAX_PRIORITY: usize = 32;
        const NO_LOC: crate::types::Loc = crate::types::Loc { i: -1, j: -1 };
        let mut locs: [crate::types::Loc; MAX_PRIORITY] = [NO_LOC; MAX_PRIORITY];
        let mut vals: [i32; MAX_PRIORITY] = [-1; MAX_PRIORITY];
        let locval_len = 5;

        let w = self.width;
        let h = self.height;
        for i in 0..w {
            for j in 0..h {
                let idx = i * h + j;
                if fg.flag[idx] {
                    let l = crate::types::Loc::new(i as i16, j as i16);
                    remove_flag(g, fg, l, FLAG_POWER);
                }
                let (army, enemy) = army_enemy(g, idx, self.pl);
                let val = self.value[idx] as f32;
                let v =
                    val * (MAX_POP as f32 - (enemy as f32 - army as f32)) * (army as f32).sqrt();
                if enemy > army && v > 7000.0 {
                    insert_locval(
                        &mut locs,
                        &mut vals,
                        locval_len,
                        crate::types::Loc::new(i as i16, j as i16),
                        v as i32,
                    );
                }
            }
        }
        for k in 0..locval_len.min(MAX_PRIORITY) {
            if vals[k] > 0 {
                let l = locs[k];
                if l.i >= 0 && l.j >= 0 {
                    add_flag(g, fg, l, FLAG_POWER);
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Difficulty {
    Easiest,
    Easy,
    Normal,
    Hard,
    Hardest,
}

impl Difficulty {
    pub fn ai_gold_bonus(self) -> i32 {
        match self {
            Difficulty::Hard => 1,
            Difficulty::Hardest => 2,
            _ => 0,
        }
    }
}

/// Default strategy for an AI player (state.c:77-96):
///   2 → Opportunist
///   3 → OneGreedy
///   4 → None
///   5 → AggrGreedy
///   6 → Noble
///   7 → PersistentGreedy
///   1 is the human and never gets a strategy.
pub fn default_strategy(pl: PlayerId) -> Option<Strategy> {
    match pl.0 {
        2 => Some(Strategy::Opportunist),
        3 => Some(Strategy::OneGreedy),
        4 => Some(Strategy::None),
        5 => Some(Strategy::AggrGreedy),
        6 => Some(Strategy::Noble),
        7 => Some(Strategy::PersistentGreedy),
        _ => None,
    }
}

fn army_enemy(g: &Grid, idx: usize, pl: PlayerId) -> (i32, i32) {
    let army = g.tiles[idx].pop[pl.index()];
    let mut enemy = 0;
    for p in 0..MAX_PLAYER {
        if p != pl.index() {
            enemy += g.tiles[idx].pop[p];
        }
    }
    (army, enemy)
}

/// `builder_default` (king.c:135). The greedy "build the best city I can
/// afford" routine shared across all kings. Returns `true` if a build occurred
/// (and the caller is then expected to re-evaluate the value map).
pub fn builder_default(king: &King, g: &mut Grid, gold: &mut [i64; MAX_PLAYER]) -> bool {
    let w = king.width;
    let h = king.height;
    let mut best_l: Option<crate::types::Loc> = None;
    let mut best_v = 0.0f32;
    for i in 0..w {
        for j in 0..h {
            let idx = i * h + j;
            // 1. Tile must be owned by this king and inhabitable.
            if g.tiles[idx].pl != king.pl || !g.tiles[idx].cl.is_inhabitable() {
                continue;
            }
            // 2. All 6 neighbours must also be owned by this king.
            let mut ok = true;
            for d in DIRS.iter() {
                let ni = i as i16 + d.i;
                let nj = j as i16 + d.j;
                if ni < 0 || nj < 0 {
                    continue;
                }
                let (ni, nj) = (ni as usize, nj as usize);
                if ni >= g.width || nj >= g.height {
                    continue;
                }
                let n_idx = ni * g.height + nj;
                if g.tiles[n_idx].cl.is_inhabitable() && g.tiles[n_idx].pl != king.pl {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let army = g.tiles[idx].pop[king.pl.index()];
            if army < MAX_POP / 10 {
                continue;
            }
            let base = match g.tiles[idx].cl {
                TileClass::Grassland => 1.0,
                TileClass::Village => 8.0,
                TileClass::Town => 32.0,
                _ => continue,
            };
            let base = if king.strategy == Strategy::Midas {
                base * (king.value[idx] as f32 + 10.0)
            } else {
                base
            };
            let v = base * (MAX_POP - army) as f32;
            if v > best_v {
                best_v = v;
                best_l = Some(crate::types::Loc::new(i as i16, j as i16));
            }
        }
    }
    if let Some(l) = best_l {
        build(g, gold, king.pl, l)
    } else {
        false
    }
}

/// `build` (king.c:23). Spends the appropriate amount of gold and upgrades
/// the tile. Returns `true` on success.
pub fn build(
    g: &mut Grid,
    gold: &mut [i64; MAX_PLAYER],
    pl: PlayerId,
    l: crate::types::Loc,
) -> bool {
    let Some(t) = g.get(l) else {
        return false;
    };
    let (price, new_cl) = match t.cl {
        TileClass::Grassland => (PRICE_VILLAGE, TileClass::Village),
        TileClass::Village => (PRICE_TOWN, TileClass::Town),
        TileClass::Town => (PRICE_CASTLE, TileClass::Castle),
        _ => return false,
    };
    if gold[pl.index()] < price {
        return false;
    }
    let idx = i_h(g, l);
    if g.tiles[idx].pl != pl {
        return false;
    }
    g.tiles[idx].cl = new_cl;
    gold[pl.index()] -= price;
    true
}

/// `degrade` (king.c:38). Downgrades a city by one level.
pub fn degrade(g: &mut Grid, l: crate::types::Loc) -> bool {
    let Some(t) = g.get(l) else {
        return false;
    };
    let new_cl = match t.cl {
        TileClass::Castle => TileClass::Town,
        TileClass::Town => TileClass::Village,
        TileClass::Village => TileClass::Grassland,
        _ => return false,
    };
    let idx = i_h(g, l);
    g.tiles[idx].cl = new_cl;
    true
}

#[inline]
fn i_h(g: &Grid, l: crate::types::Loc) -> usize {
    (l.i as usize) * g.height + (l.j as usize)
}

/// Insert `(l, v)` into the descending-sorted `vals` array, shifting later
/// entries down (king.c:314).
fn insert_locval(
    locs: &mut [crate::types::Loc],
    vals: &mut [i32],
    len: usize,
    l: crate::types::Loc,
    v: i32,
) {
    let max = locs.len().min(vals.len()).min(len);
    let mut i = 0;
    while i < max && vals[i] >= v {
        i += 1;
    }
    if i >= max {
        return;
    }
    let mut j = max - 1;
    while j > i {
        locs[j] = locs[j - 1];
        vals[j] = vals[j - 1];
        j -= 1;
    }
    locs[i] = l;
    vals[i] = v;
}

/// Local re-implementation of `spread` that targets the king's value field.
/// Identical to `crate::flags::spread` but kept local to avoid leaking
/// the scratch `u` buffer.
fn spread_value(
    g: &Grid,
    u: &mut [i32],
    v: &mut [i32],
    l: crate::types::Loc,
    val: i32,
    factor: i32,
) {
    if !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false) {
        return;
    }
    let h = g.height;
    let idx = (l.i as usize) * h + (l.j as usize);
    let d = val - u[idx];
    if d > 0 {
        v[idx] = (v[idx] + d * factor).max(0);
        u[idx] += d;
        let next_val = val / 2;
        if next_val <= 0 {
            return;
        }
        for dir in DIRS.iter() {
            let n = crate::types::Loc::new(l.i + dir.i, l.j + dir.j);
            spread_value(g, u, v, n, next_val, factor);
        }
    }
}

fn even_value(g: &Grid, u: &mut [i32], l: crate::types::Loc, val: i32) {
    if !g.get(l).map(|t| t.cl.is_inhabitable()).unwrap_or(false) {
        return;
    }
    let h = g.height;
    let idx = (l.i as usize) * h + (l.j as usize);
    if u[idx] == val {
        return;
    }
    u[idx] = val;
    for dir in DIRS.iter() {
        let n = crate::types::Loc::new(l.i + dir.i, l.j + dir.j);
        even_value(g, u, n, val);
    }
}

// ---- Tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::DIRECTIONS;
    use crate::grid::Grid;
    use crate::rng::CowRng;
    use crate::types::{Loc, PlayerId, TileClass, NEUTRAL};

    fn small_owned_grassland(w: usize, h: usize, pl: PlayerId) -> (Grid, [i64; MAX_PLAYER]) {
        let mut g = Grid::empty(w, h);
        for i in 0..w {
            for j in 0..h {
                if let Some(t) = g.get_mut(Loc::new(i as i16, j as i16)) {
                    t.cl = TileClass::Grassland;
                    t.pl = pl;
                }
            }
        }
        (g, [0; MAX_PLAYER])
    }

    #[test]
    fn default_strategy_matches_state_c() {
        assert_eq!(default_strategy(PlayerId(2)), Some(Strategy::Opportunist));
        assert_eq!(default_strategy(PlayerId(3)), Some(Strategy::OneGreedy));
        assert_eq!(default_strategy(PlayerId(4)), Some(Strategy::None));
        assert_eq!(default_strategy(PlayerId(5)), Some(Strategy::AggrGreedy));
        assert_eq!(default_strategy(PlayerId(6)), Some(Strategy::Noble));
        assert_eq!(
            default_strategy(PlayerId(7)),
            Some(Strategy::PersistentGreedy)
        );
        assert_eq!(default_strategy(PlayerId(1)), None);
    }

    #[test]
    fn build_spends_gold_and_upgrades_tile() {
        let (mut g, mut gold) = small_owned_grassland(5, 5, PlayerId(1));
        gold[1] = 1000;
        let l = Loc::new(2, 2);
        assert!(build(&mut g, &mut gold, PlayerId(1), l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Village);
        assert_eq!(gold[1], 1000 - PRICE_VILLAGE);
        assert!(build(&mut g, &mut gold, PlayerId(1), l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Town);
        assert_eq!(gold[1], 1000 - PRICE_VILLAGE - PRICE_TOWN);
        assert!(build(&mut g, &mut gold, PlayerId(1), l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Castle);
    }

    #[test]
    fn build_fails_when_not_owner() {
        let mut g = Grid::empty(5, 5);
        let mut gold = [0; MAX_PLAYER];
        gold[1] = 1000;
        let l = Loc::new(2, 2);
        if let Some(t) = g.get_mut(l) {
            t.cl = TileClass::Grassland;
            t.pl = NEUTRAL;
        }
        assert!(!build(&mut g, &mut gold, PlayerId(1), l));
    }

    #[test]
    fn build_fails_on_insufficient_gold() {
        let (mut g, mut gold) = small_owned_grassland(5, 5, PlayerId(1));
        gold[1] = 10;
        let l = Loc::new(2, 2);
        assert!(!build(&mut g, &mut gold, PlayerId(1), l));
    }

    #[test]
    fn degrade_steps_down() {
        let (mut g, _) = small_owned_grassland(5, 5, PlayerId(1));
        let l = Loc::new(2, 2);
        if let Some(t) = g.get_mut(l) {
            t.cl = TileClass::Castle;
        }
        assert!(degrade(&mut g, l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Town);
        assert!(degrade(&mut g, l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Village);
        assert!(degrade(&mut g, l));
        assert_eq!(g.get(l).unwrap().cl, TileClass::Grassland);
        assert!(!degrade(&mut g, l));
    }

    #[test]
    fn difficulty_gold_bonus_matches_state_c() {
        assert_eq!(Difficulty::Normal.ai_gold_bonus(), 0);
        assert_eq!(Difficulty::Hard.ai_gold_bonus(), 1);
        assert_eq!(Difficulty::Hardest.ai_gold_bonus(), 2);
    }

    #[test]
    fn builder_default_returns_false_when_army_too_small() {
        let (mut g, mut gold) = small_owned_grassland(5, 5, PlayerId(1));
        gold[1] = 10_000;
        // Put 10 pop on every tile (below the 50-min threshold).
        for i in 0..5 {
            for j in 0..5 {
                let idx = i * g.height + j;
                g.tiles[idx].pop[1] = 10;
            }
        }
        let king = King::new_clamped(PlayerId(1), Strategy::None, 5, 5);
        assert!(!builder_default(&king, &mut g, &mut gold));
    }

    #[test]
    fn evaluate_map_populates_value_field() {
        let mut g = Grid::empty(5, 5);
        // One castle in the middle.
        let idx = 2 * g.height + 2;
        g.tiles[idx].cl = TileClass::Castle;
        g.tiles[idx].pl = PlayerId(1);
        g.tiles[idx].pop[1] = 10;
        let mut king = King::new_clamped(PlayerId(1), Strategy::None, 5, 5);
        let mut rng = CowRng::from_seed(0);
        king.evaluate_map(&g, Difficulty::Normal, &mut rng);
        // Some tiles must have a positive value; the field is initialised
        // to all-zero otherwise.
        assert!(king.value.iter().any(|&v| v > 0));
    }

    #[test]
    fn place_flags_does_nothing_for_midas() {
        let (g, _gold) = small_owned_grassland(5, 5, PlayerId(1));
        let mut fg = FlagGrid::new(5, 5);
        let king = King::new_clamped(PlayerId(1), Strategy::Midas, 5, 5);
        king.place_flags(&g, &mut fg);
        assert!(!fg.flag.iter().any(|&b| b));
    }

    #[allow(dead_code)]
    fn _force_use_of_DIRECTIONS() -> usize {
        DIRECTIONS
    }
}
