//! Localisation table — Chinese by default, English selectable.
//!
//! `Lang::t(TextKey)` is the single seam between UI code and text. Every
//! user-visible string in `curseofwar` should go through this table; no
//! raw Chinese or English should appear in `render::*` or `cli::*`.

use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    /// Detect language: CLI override → `$COW_LANG` → `$LC_ALL` / `$LANG` →
    /// default `Zh`.
    pub fn detect(cli: Option<Lang>) -> Lang {
        if let Some(l) = cli {
            return l;
        }
        if let Ok(v) = std::env::var("COW_LANG") {
            if let Some(l) = Self::from_code(&v) {
                return l;
            }
        }
        for var in ["LC_ALL", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if let Some(l) = Self::from_code(&v) {
                    return l;
                }
            }
        }
        Lang::Zh
    }

    pub(super) fn from_code(s: &str) -> Option<Lang> {
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("zh") {
            Some(Lang::Zh)
        } else if lower.starts_with("en") {
            Some(Lang::En)
        } else {
            None
        }
    }
}

/// All user-visible strings. `Lang::t` matches exhaustively; adding a new
/// variant will fail to compile until you add the language pair, which is
/// the point.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Some variants are only constructed from inside `Lang::t`.
pub enum TextKey {
    /// Sentinel — emitted by `Lang::t` callers that want an empty key slot.
    /// Matched as `"".into()` by both languages.
    Noop,

    AppTitle,
    AppSubtitle,
    WrittenBy,
    CmdLineHeading,
    MapGenWait,
    RandomSeedWas(u32),
    MultiplayerUnimplemented,

    // Status bar
    LabelGold,
    LabelPrices,
    LabelSpeed,
    LabelDate,
    LabelPopulationAtCursor,
    SpeedName(SpeedKind),

    // Help block
    HelpAddRemoveFlag,
    HelpBuild,
    HelpClearAllFlags,
    HelpClearHalfFlags,
    HelpSlowDown,
    HelpSpeedUp,
    HelpPause,
    HelpQuit,
    HelpFlagKey,
    HelpBuildKey,
    HelpClearAllKey,
    HelpClearHalfKey,
    HelpSlowDownKey,
    HelpSpeedUpKey,
    HelpPauseKey,
    HelpQuitKey,

    // Outcome banner
    YouWon,
    YouLost,

    // Quit dialog
    QuitPrompt,
    QuitHint,

    // Errors
    ErrorTui,

    // Version
    Version(&'static str),
}

/// Speed label — we carry a static tag so the UI table can match on it
/// without holding a `cow_core::Speed` (which would be a layering violation).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpeedKind {
    Pause,
    Slowest,
    Slower,
    Slow,
    Normal,
    Fast,
    Faster,
    Fastest,
}

impl From<cow_core::types::Speed> for SpeedKind {
    fn from(s: cow_core::types::Speed) -> Self {
        use cow_core::types::Speed as S;
        match s {
            S::Pause => SpeedKind::Pause,
            S::Slowest => SpeedKind::Slowest,
            S::Slower => SpeedKind::Slower,
            S::Slow => SpeedKind::Slow,
            S::Normal => SpeedKind::Normal,
            S::Fast => SpeedKind::Fast,
            S::Faster => SpeedKind::Faster,
            S::Fastest => SpeedKind::Fastest,
        }
    }
}

impl Lang {
    pub fn t(&self, key: TextKey) -> Cow<'static, str> {
        match (self, key) {
            (_, TextKey::Noop) => Cow::Borrowed(""),
            (Lang::Zh, TextKey::AppTitle) => Cow::Borrowed("Curse of War"),
            (Lang::En, TextKey::AppTitle) => Cow::Borrowed("Curse of War"),

            (Lang::Zh, TextKey::AppSubtitle) => {
                Cow::Borrowed("一款 Linux 终端上的快节奏即时策略游戏。")
            }
            (Lang::En, TextKey::AppSubtitle) => {
                Cow::Borrowed("A fast-paced real-time strategy game for the Linux terminal.")
            }

            (Lang::Zh, TextKey::WrittenBy) => {
                Cow::Borrowed("原作者：Alexey Nikolaev，2013。本版本为 Rust 重写。")
            }
            (Lang::En, TextKey::WrittenBy) => Cow::Borrowed(
                "Originally written by Alexey Nikolaev in 2013. Rust re-implementation.",
            ),

            (Lang::Zh, TextKey::CmdLineHeading) => Cow::Borrowed("  命令行参数："),
            (Lang::En, TextKey::CmdLineHeading) => Cow::Borrowed("  Command line arguments:"),

            (Lang::Zh, TextKey::MapGenWait) => Cow::Borrowed("正在生成地图，请稍候……"),
            (Lang::En, TextKey::MapGenWait) => Cow::Borrowed("Map is generated. Please wait."),

            (Lang::Zh, TextKey::RandomSeedWas(s)) => Cow::Owned(format!("随机种子为 {}", s)),
            (Lang::En, TextKey::RandomSeedWas(s)) => Cow::Owned(format!("Random seed was {}", s)),

            (Lang::Zh, TextKey::MultiplayerUnimplemented) => {
                Cow::Borrowed("多人联网模式尚未在本版本实现，请使用单机模式。")
            }
            (Lang::En, TextKey::MultiplayerUnimplemented) => Cow::Borrowed(
                "Multiplayer is not implemented in this version yet. Please play single-player.",
            ),

            // Status labels
            (Lang::Zh, TextKey::LabelGold) => Cow::Borrowed("金币:"),
            (Lang::En, TextKey::LabelGold) => Cow::Borrowed("Gold:"),
            (Lang::Zh, TextKey::LabelPrices) => Cow::Borrowed("价格: 160, 240, 320."),
            (Lang::En, TextKey::LabelPrices) => Cow::Borrowed("Prices: 160, 240, 320."),
            (Lang::Zh, TextKey::LabelSpeed) => Cow::Borrowed("速度:"),
            (Lang::En, TextKey::LabelSpeed) => Cow::Borrowed("Speed:"),
            (Lang::Zh, TextKey::LabelDate) => Cow::Borrowed("日期:"),
            (Lang::En, TextKey::LabelDate) => Cow::Borrowed("Date:"),
            (Lang::Zh, TextKey::LabelPopulationAtCursor) => Cow::Borrowed("光标处人口:"),
            (Lang::En, TextKey::LabelPopulationAtCursor) => {
                Cow::Borrowed("Population at the cursor:")
            }

            (l, TextKey::SpeedName(sk)) => {
                let s: &'static str = match sk {
                    SpeedKind::Pause => "暂停",
                    SpeedKind::Slowest => "最慢",
                    SpeedKind::Slower => "慢",
                    SpeedKind::Slow => "较慢",
                    SpeedKind::Normal => "正常",
                    SpeedKind::Fast => "快",
                    SpeedKind::Faster => "较快",
                    SpeedKind::Fastest => "最快",
                };
                if matches!(l, Lang::En) {
                    Cow::Borrowed(match sk {
                        SpeedKind::Pause => "Pause",
                        SpeedKind::Slowest => "Slowest",
                        SpeedKind::Slower => "Slower",
                        SpeedKind::Slow => "Slow",
                        SpeedKind::Normal => "Normal",
                        SpeedKind::Fast => "Fast",
                        SpeedKind::Faster => "Faster",
                        SpeedKind::Fastest => "Fastest",
                    })
                } else {
                    Cow::Borrowed(s)
                }
            }

            // Help block
            (Lang::Zh, TextKey::HelpFlagKey) => Cow::Borrowed("[空格]"),
            (Lang::En, TextKey::HelpFlagKey) => Cow::Borrowed("[Space]"),
            (Lang::Zh, TextKey::HelpAddRemoveFlag) => Cow::Borrowed("增/删旗"),
            (Lang::En, TextKey::HelpAddRemoveFlag) => Cow::Borrowed("add/remove a flag"),
            (Lang::Zh, TextKey::HelpBuildKey) => Cow::Borrowed("[R/V]"),
            (Lang::En, TextKey::HelpBuildKey) => Cow::Borrowed("[R or V]"),
            (Lang::Zh, TextKey::HelpBuild) => Cow::Borrowed("建造"),
            (Lang::En, TextKey::HelpBuild) => Cow::Borrowed("build"),
            (Lang::Zh, TextKey::HelpClearAllKey) => Cow::Borrowed("[X]"),
            (Lang::En, TextKey::HelpClearAllKey) => Cow::Borrowed("[X]"),
            (Lang::Zh, TextKey::HelpClearAllFlags) => Cow::Borrowed("清除所有旗"),
            (Lang::En, TextKey::HelpClearAllFlags) => Cow::Borrowed("remove all flags"),
            (Lang::Zh, TextKey::HelpClearHalfKey) => Cow::Borrowed("[C]"),
            (Lang::En, TextKey::HelpClearHalfKey) => Cow::Borrowed("[C]"),
            (Lang::Zh, TextKey::HelpClearHalfFlags) => Cow::Borrowed("清除一半旗"),
            (Lang::En, TextKey::HelpClearHalfFlags) => Cow::Borrowed("remove 50% of flags"),
            (Lang::Zh, TextKey::HelpSlowDownKey) => Cow::Borrowed("[S]"),
            (Lang::En, TextKey::HelpSlowDownKey) => Cow::Borrowed("[S]"),
            (Lang::Zh, TextKey::HelpSlowDown) => Cow::Borrowed("减速"),
            (Lang::En, TextKey::HelpSlowDown) => Cow::Borrowed("slow down"),
            (Lang::Zh, TextKey::HelpSpeedUpKey) => Cow::Borrowed("[F]"),
            (Lang::En, TextKey::HelpSpeedUpKey) => Cow::Borrowed("[F]"),
            (Lang::Zh, TextKey::HelpSpeedUp) => Cow::Borrowed("加速"),
            (Lang::En, TextKey::HelpSpeedUp) => Cow::Borrowed("speed up"),
            (Lang::Zh, TextKey::HelpPauseKey) => Cow::Borrowed("[P]"),
            (Lang::En, TextKey::HelpPauseKey) => Cow::Borrowed("[P]"),
            (Lang::Zh, TextKey::HelpPause) => Cow::Borrowed("暂停"),
            (Lang::En, TextKey::HelpPause) => Cow::Borrowed("pause"),
            (Lang::Zh, TextKey::HelpQuitKey) => Cow::Borrowed("[Q]"),
            (Lang::En, TextKey::HelpQuitKey) => Cow::Borrowed("[Q]"),
            (Lang::Zh, TextKey::HelpQuit) => Cow::Borrowed("退出"),
            (Lang::En, TextKey::HelpQuit) => Cow::Borrowed("quit"),

            // Outcome
            (Lang::Zh, TextKey::YouWon) => Cow::Borrowed("你胜利了！"),
            (Lang::En, TextKey::YouWon) => Cow::Borrowed("You are victorious!"),
            (Lang::Zh, TextKey::YouLost) => Cow::Borrowed("你失败了！"),
            (Lang::En, TextKey::YouLost) => Cow::Borrowed("You are defeated!"),

            // Dialog
            (Lang::Zh, TextKey::QuitPrompt) => Cow::Borrowed("   退出? [Y/N]   "),
            (Lang::En, TextKey::QuitPrompt) => Cow::Borrowed("   Quit? [Y/N]   "),
            (Lang::Zh, TextKey::QuitHint) => Cow::Borrowed("        [Q/Esc]  "),
            (Lang::En, TextKey::QuitHint) => Cow::Borrowed("        [Q/Esc]  "),

            // Errors
            (Lang::Zh, TextKey::ErrorTui) => Cow::Borrowed("终端界面错误:"),
            (Lang::En, TextKey::ErrorTui) => Cow::Borrowed("TUI error:"),

            (_l, TextKey::Version(s)) => Cow::Owned(format!("curseofwar {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_zh_default() {
        // We can't safely mutate env vars from a `#![forbid(unsafe_code)]`
        // crate, so we only verify the parse helper directly.
        assert_eq!(Lang::from_code("zh"), Some(Lang::Zh));
        assert_eq!(Lang::from_code("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_code("fr"), None);
    }

    #[test]
    fn status_labels_translate() {
        assert_eq!(Lang::Zh.t(TextKey::LabelGold), "金币:");
        assert_eq!(Lang::En.t(TextKey::LabelGold), "Gold:");
    }

    #[test]
    fn speed_name_chinese() {
        let s = Lang::Zh.t(TextKey::SpeedName(SpeedKind::Normal));
        assert_eq!(s, "正常");
    }

    #[test]
    fn help_block_chinese_contains_key_brackets() {
        let key = Lang::Zh.t(TextKey::HelpFlagKey);
        assert!(key.starts_with('['));
    }

    #[test]
    fn outcome_translates() {
        assert_eq!(Lang::Zh.t(TextKey::YouWon), "你胜利了！");
        assert_eq!(Lang::En.t(TextKey::YouWon), "You are victorious!");
    }

    #[test]
    fn quit_prompt_translates() {
        assert!(Lang::Zh.t(TextKey::QuitPrompt).contains("退出"));
        assert!(Lang::En.t(TextKey::QuitPrompt).contains("Quit"));
    }
}
