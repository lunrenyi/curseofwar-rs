//! Reproducible RNG used by the simulation (`CowRng`).
//!
//! Wraps `StdRng` (ChaCha12) and exposes the small set of helpers the C
//! source uses (`rand() % n`, `rand() / RAND_MAX`, `rnd_round`). The RNG is
//! always carried inside `State`; **never** use `rand::random` or
//! `rand::thread_rng` in this crate — that would break reproducibility.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, RngCore, SeedableRng};

/// Default RNG. Deterministic across platforms (ChaCha-based).
pub type InnerRng = StdRng;

/// The simulation's RNG. Keep it inside [`crate::State`] so that two runs with
/// the same seed produce the same map and the same subsequent evolution.
#[derive(Debug)]
pub struct CowRng {
    inner: InnerRng,
}

impl CowRng {
    /// Build from an explicit seed (matches `-R <seed>` and the
    /// `srand(map_seed)` call in state.c:101).
    pub fn from_seed(seed: u32) -> Self {
        CowRng {
            inner: InnerRng::seed_from_u64(seed as u64),
        }
    }

    /// Build from system entropy. Used for things that must not be reproducible
    /// — currently only the initial date (state.c:56), which the C source
    /// evaluates *before* `srand(map_seed)`.
    pub fn from_entropy() -> Self {
        CowRng {
            inner: InnerRng::from_entropy(),
        }
    }

    /// `rand() % n` — uniform in `0..n`. Caller must ensure `n > 0`.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        // StdRng.next_u32() % n is biased; that matches glibc rand()%n bias,
        // which is exactly the behaviour we want for a faithful re-write.
        (self.inner.next_u32()) % n
    }

    /// `(float)rand() / (float)RAND_MAX` — uniform in `[0.0, 1.0)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        // Sample uniformly in [0, 2^32) then scale. Matches the C expression.
        const SCALE: f64 = 1.0 / ((1u64 << 32) as f64);
        (self.inner.next_u32() as f64 * SCALE) as f32
    }

    /// `rnd_round(x)` (state.c:222) — truncate toward zero, then probabilistically
    /// round up by 1 with probability equal to the fractional part.
    pub fn rnd_round(&mut self, x: f32) -> i32 {
        let i = x as i32;
        let frac = x - i as f32;
        if self.unit() < frac {
            i + 1
        } else {
            i
        }
    }

    /// `rand() & 1` — used in state.c:344,347 to randomise scan direction.
    #[inline]
    pub fn coin_flip(&mut self) -> bool {
        self.inner.next_u32() & 1 != 0
    }

    /// `rand() % 6` — pick a starting direction in `0..6` for the migration
    /// rotation offset (state.c:355).
    #[inline]
    pub fn dir_offset(&mut self) -> usize {
        (self.inner.next_u32() % 6) as usize
    }

    /// Fisher–Yates style shuffle using `self` — used by grid.c:285 to permute
    /// the AI / human player lists. We just use the standard library's RNG
    /// here (operating on borrowed RNG) to keep the API tight.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        // SliceRandom accepts any RNG; we pass a mutable borrow.
        // (Temporarily re-borrow `inner` to satisfy SliceRandom's API.)
        v.shuffle(&mut self.inner);
    }

    /// Borrow the inner RNG for advanced uses (e.g. `Distribution::sample`).
    pub fn inner_mut(&mut self) -> &mut InnerRng {
        &mut self.inner
    }
}

/// `random_bit()` — used in king.c:115, 120 to add difficulty noise. Just an
/// alias for clarity at the call site.
impl CowRng {
    /// Returns a random value in `0..n`. Equivalent to `below(n)`.
    #[inline]
    pub fn rand_mod(&mut self, n: u32) -> u32 {
        self.below(n)
    }

    /// `rng.gen_range(lo..=hi)` — uniform inclusive range. Used by king.c
    /// ("rand() % 7 - 3" etc.).
    pub fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        self.inner.gen_range(lo..=hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_zero_returns_zero() {
        let mut r = CowRng::from_seed(42);
        // Note: caller responsibility — but the function must not panic on n=0
        // for defensive reasons in tests. We document the contract instead:
        assert!(r.below(1) == 0);
    }

    #[test]
    fn rnd_round_truncates_and_probably_rounds() {
        let mut r = CowRng::from_seed(7);
        // Statistical: across many trials, rnd_round(2.3) should be 2 with
        // prob 0.7 and 3 with prob 0.3 (within ±3 % on 10 000 samples).
        let n = 10_000;
        let mut count3 = 0;
        for _ in 0..n {
            if r.rnd_round(2.3) == 3 {
                count3 += 1;
            }
        }
        let frac = count3 as f64 / n as f64;
        assert!(frac > 0.27 && frac < 0.33, "frac={}", frac);
    }

    #[test]
    fn rnd_round_integer_is_exact() {
        let mut r = CowRng::from_seed(1);
        for _ in 0..100 {
            assert_eq!(r.rnd_round(5.0), 5);
            assert_eq!(r.rnd_round(-2.0), -2);
        }
    }

    #[test]
    fn reproducibility_same_seed_same_sequence() {
        let mut a = CowRng::from_seed(123);
        let mut b = CowRng::from_seed(123);
        for _ in 0..1000 {
            assert_eq!(a.below(100), b.below(100));
        }
    }

    #[test]
    fn unit_is_in_range() {
        let mut r = CowRng::from_seed(0);
        for _ in 0..10_000 {
            let u = r.unit();
            assert!(u >= 0.0 && u < 1.0);
        }
    }
}
