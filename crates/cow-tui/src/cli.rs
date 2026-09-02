//! Command-line parsing for the `curseofwar` binary.
//!
//! Faithful to the original `getopt`-based parsing in main-common.c:138
//! (getopt string `"hvrTW:H:i:l:q:d:s:R:S:E:e:C:c:"`), plus the new
//! `--lang <zh|en>` option introduced by the Rust re-implementation.
//!
//! On `-v` or `-h` (and on any invalid option) the program prints the help
//! text and returns 1 — matching the C version's behaviour exactly.

use std::process::ExitCode;

use getopts::Options;

use crate::i18n::{Lang, TextKey};

/// Static version string. Matches `VERSION` in the C source (currently 1.2.0
/// inside a 1.3.0 release).
pub const VERSION: &str = "0.1.0 (Rust re-implementation)";

/// Default map dimensions match the C defaults (main-common.c:143-144).
pub const DEFAULT_W: usize = 21;
pub const DEFAULT_H: usize = 21;
pub const MIN_DIM: usize = 14;
pub const MAX_DIM_W: usize = 40;
pub const MAX_DIM_H: usize = 29;

/// Aggregated options.
pub struct CliOptions {
    pub keep_random: bool, // -r
    pub timeline: bool,    // -T
    pub w: Option<usize>,
    pub h: Option<usize>,
    pub inequality: Option<i32>, // -i
    pub loc_num: Option<usize>,  // -l
    pub conditions: Option<i32>, // -q
    pub map_seed: Option<u32>,   // -R
    pub dif: Option<String>,     // -d
    pub speed: Option<String>,   // -s
    pub shape: Option<String>,   // -S
    pub multiplayer: MultiplayerOptions,
}

#[derive(Default)]
pub struct MultiplayerOptions {
    pub clients_num: Option<usize>, // -E
    pub server_port: Option<String>,
    pub server_addr: Option<String>,
    pub client_port: Option<String>,
}

pub fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    run_with_args(&args)
}

pub fn run_with_args(args: &[String]) -> ExitCode {
    // First, look for --lang so we can localise help/errors.
    let mut cli_lang: Option<Lang> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--lang" && i + 1 < args.len() {
            cli_lang = parse_lang(&args[i + 1]);
            break;
        } else if args[i].starts_with("--lang=") {
            cli_lang = parse_lang(&args[i]["--lang=".len()..]);
            break;
        }
        i += 1;
    }
    let lang = Lang::detect(cli_lang);

    let mut opts = Options::new();
    // Short options — identical to the C version's getopt("hvrTW:H:i:l:q:d:s:R:S:E:e:C:c:").
    opts.optflag("h", "help", "show this help");
    opts.optflag("v", "version", "show version");
    opts.optflag("r", "random", "absolutely random initial conditions");
    opts.optflag("T", "timeline", "show the timeline");
    opts.optflag("", "headless", "headless smoke (no TUI)");
    opts.optopt(
        "",
        "headless-steps",
        "number of simulation steps (default 1000)",
        "N",
    );
    opts.optopt("W", "width", "map width (default 21)", "W");
    opts.optopt("H", "height", "map height (default 21)", "H");
    opts.optopt("i", "inequality", "inequality 0..4", "I");
    opts.optopt("l", "locations", "number of countries", "N");
    opts.optopt("q", "quality", "player location quality", "Q");
    opts.optopt("R", "seed", "map seed", "SEED");
    opts.optopt("d", "difficulty", "ee|e|n|h|hh", "D");
    opts.optopt("s", "speed", "p|sss|ss|s|n|f|ff|fff", "S");
    opts.optopt("S", "shape", "rhombus|rect|hex", "SHAPE");
    opts.optopt("E", "server-clients", "start a server for N clients", "N");
    opts.optopt("e", "server-port", "server's port (default 19140)", "PORT");
    opts.optopt("C", "server-ip", "start a client and connect to IP", "IP");
    opts.optopt("c", "client-port", "client's port (default 19150)", "PORT");
    // Long-only: language selector.
    opts.optopt("", "lang", "language: zh or en", "LANG");

    let parsed = match opts.parse(&args[1..]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            print_help(&lang);
            return ExitCode::from(1);
        }
    };

    if parsed.opt_present("v") {
        println!("{}", lang.t(TextKey::Version(VERSION)));
        return ExitCode::from(1);
    }
    if parsed.opt_present("h") || parsed.free.len() > 0 {
        print_help(&lang);
        return ExitCode::from(1);
    }

    // Unwrap each option with validation.
    let out = CliOptions {
        keep_random: parsed.opt_present("r"),
        timeline: parsed.opt_present("T"),
        w: parse_bounded_usize(parsed.opt_str("W"), MIN_DIM, MAX_DIM_W),
        h: parse_bounded_usize(parsed.opt_str("H"), MIN_DIM, MAX_DIM_H),
        inequality: parsed.opt_str("i").and_then(|s| s.parse().ok()),
        loc_num: parsed.opt_str("l").and_then(|s| s.parse().ok()),
        conditions: parsed.opt_str("q").and_then(|s| s.parse().ok()),
        map_seed: parsed.opt_str("R").and_then(|s| {
            let n: i64 = s.parse().ok()?;
            Some(n.unsigned_abs() as u32)
        }),
        dif: parsed.opt_str("d"),
        speed: parsed.opt_str("s"),
        shape: parsed.opt_str("S"),
        multiplayer: MultiplayerOptions {
            clients_num: parsed.opt_str("E").and_then(|s| s.parse().ok()),
            server_port: parsed.opt_str("e"),
            server_addr: parsed.opt_str("C"),
            client_port: parsed.opt_str("c"),
        },
    };

    if out.multiplayers_set() {
        // Friendly notice — the multiplayer surface is parsed but unimplemented.
        eprintln!("{}", lang.t(TextKey::MultiplayerUnimplemented));
        return ExitCode::from(1);
    }

    // Validate difficulty / speed / shape strings so the user gets an immediate
    // error for typos (mirroring the C version's `print_help()` on bad input).
    if let Some(d) = &out.dif {
        if !matches!(
            d.as_str(),
            "n" | "e" | "e1" | "ee" | "e2" | "h" | "h1" | "hh" | "h2"
        ) {
            print_help(&lang);
            return ExitCode::from(1);
        }
    }
    if let Some(s) = &out.speed {
        if !matches!(
            s.as_str(),
            "p" | "s"
                | "s1"
                | "ss"
                | "s2"
                | "sss"
                | "s3"
                | "n"
                | "f"
                | "f1"
                | "ff"
                | "f2"
                | "fff"
                | "f3"
        ) {
            print_help(&lang);
            return ExitCode::from(1);
        }
    }
    if let Some(s) = &out.shape {
        if !matches!(s.as_str(), "rhombus" | "rect" | "hex") {
            print_help(&lang);
            return ExitCode::from(1);
        }
    }
    // `-l` must be at least 2 and at most the maximum available locations
    // for the chosen shape (C: `op->loc_num < 2 || op->loc_num > avlbl_loc_num`).
    let avlbl_loc_num = out
        .shape
        .as_deref()
        .map(|s| match s {
            "rhombus" => 4,
            "rect" => 4,
            "hex" => 6,
            _ => 0,
        })
        .unwrap_or(4); // default shape = rect = 4
    if let Some(n) = out.loc_num {
        if n < 2 || n > avlbl_loc_num {
            print_help(&lang);
            return ExitCode::from(1);
        }
    }

    // If --headless was requested, run the headless smoke channel (M1
    // verification) and skip the TUI loop.
    if parsed.opt_present("headless") {
        return run_headless(&out, &parsed, &lang);
    }

    // M2: launch the TUI loop.
    println!("{}", lang.t(TextKey::MapGenWait));

    use cow_core::ai::Difficulty;
    use cow_core::state::{GameOptions, State};
    use cow_core::types::{Shape, Speed};

    let dif = match out.dif.as_deref() {
        Some("ee") | Some("e2") => Difficulty::Easiest,
        Some("e") | Some("e1") => Difficulty::Easy,
        Some("h") | Some("h1") => Difficulty::Hard,
        Some("hh") | Some("h2") => Difficulty::Hardest,
        _ => Difficulty::Normal,
    };
    let shape = match out.shape.as_deref() {
        Some("rhombus") => Shape::Rhombus,
        Some("hex") => Shape::Hex,
        _ => Shape::Rect,
    };
    let speed = match out.speed.as_deref() {
        Some("p") => Speed::Pause,
        Some("sss") | Some("s3") => Speed::Slowest,
        Some("ss") | Some("s2") => Speed::Slower,
        Some("s") | Some("s1") => Speed::Slow,
        Some("f") | Some("f1") => Speed::Fast,
        Some("ff") | Some("f2") => Speed::Faster,
        Some("fff") | Some("f3") => Speed::Fastest,
        _ => Speed::Normal,
    };

    let seed = out.map_seed.unwrap_or_else(|| {
        use rand::RngCore;
        let mut bytes = [0u8; 4];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        u32::from_ne_bytes(bytes)
    });

    let w = out.w.unwrap_or(DEFAULT_W);
    let h = out.h.unwrap_or(DEFAULT_H);

    let opts = GameOptions {
        keep_random: out.keep_random,
        dif,
        speed,
        w,
        h,
        loc_num: out.loc_num.unwrap_or(shape.avlbl_loc_num()),
        map_seed: seed,
        conditions: out.conditions.unwrap_or(0),
        timeline: out.timeline,
        inequality: out.inequality.unwrap_or(-1),
        shape,
    };

    let state = State::new(&opts);
    if let Err(e) = crate::app::run_tui(state, lang) {
        eprintln!("TUI error: {}", e);
        return ExitCode::FAILURE;
    }
    println!("{}", lang.t(TextKey::RandomSeedWas(seed)));
    ExitCode::SUCCESS
}

fn run_headless(out: &CliOptions, parsed: &getopts::Matches, lang: &Lang) -> ExitCode {
    use cow_core::ai::Difficulty;
    use cow_core::state::{GameOptions, State};
    use cow_core::types::Shape;

    let n: usize = parsed
        .opt_str("headless-steps")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let seed = out.map_seed.unwrap_or_else(|| {
        use rand::RngCore;
        let mut bytes = [0u8; 4];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        u32::from_ne_bytes(bytes)
    });

    let dif = match out.dif.as_deref() {
        Some("ee") | Some("e2") => Difficulty::Easiest,
        Some("e") | Some("e1") => Difficulty::Easy,
        Some("h") | Some("h1") => Difficulty::Hard,
        Some("hh") | Some("h2") => Difficulty::Hardest,
        _ => Difficulty::Normal,
    };

    // M1 默认 shape = Rect, M2 实现 CLI 转换。
    let shape = match out.shape.as_deref() {
        Some("rhombus") => Shape::Rhombus,
        Some("hex") => Shape::Hex,
        _ => Shape::Rect,
    };

    let w = out.w.unwrap_or(DEFAULT_W);
    let h = out.h.unwrap_or(DEFAULT_H);

    let opts = GameOptions {
        keep_random: out.keep_random,
        dif,
        speed: cow_core::types::Speed::Normal,
        w,
        h,
        loc_num: out.loc_num.unwrap_or(shape.avlbl_loc_num()),
        map_seed: seed,
        conditions: out.conditions.unwrap_or(0),
        timeline: out.timeline,
        inequality: out.inequality.unwrap_or(-1),
        shape,
    };

    let mut state = State::new(&opts);
    println!("{}", lang.t(TextKey::MapGenWait));
    println!(
        "headless: seed={} dif={:?} shape={:?} {}x{} loc={} — running {} steps",
        seed, dif, shape, w, h, opts.loc_num, n
    );

    for i in 0..n {
        state.step();
        if i % 100 == 0 {
            // Print a compact progress line. We report `step` and per-player
            // state; the absolute `state.time` includes the entropy-sourced
            // starting year and therefore differs between two runs (see
            // PLAN.md §2.6 "Double RNG"), but everything derived from
            // `srand(map_seed)` is reproducible.
            let total: i64 = state.grid.tiles.iter().map(|t| t.pop[1] as i64).sum();
            let gold = state.gold[1];
            println!(
                "step={:6} pop[1]={:5} gold[1]={} outcome={:?}",
                i,
                total,
                gold,
                state.win_or_lose()
            );
        }
    }
    ExitCode::SUCCESS
}

impl CliOptions {
    fn multiplayers_set(&self) -> bool {
        self.multiplayer.clients_num.is_some()
            || self.multiplayer.server_port.is_some()
            || self.multiplayer.server_addr.is_some()
            || self.multiplayer.client_port.is_some()
    }
}

fn parse_bounded_usize(s: Option<String>, lo: usize, hi: usize) -> Option<usize> {
    let v: usize = s?.parse().ok()?;
    Some(v.clamp(lo, hi))
}

fn parse_lang(s: &str) -> Option<Lang> {
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("zh") {
        Some(Lang::Zh)
    } else if lower.starts_with("en") {
        Some(Lang::En)
    } else {
        None
    }
}

fn print_help(lang: &Lang) {
    // ASCII logo (kept identical to the C source's hand-drawn banner).
    let logo = "                                 __                      \n\
                \x20    ____                        /  ]                     \n\
                \x20   / __ \\_ _ ___ ___ ___    __ _| |_  /\\      /\\___ ___  \n\
                \x20 _/ /  \\/ | |X _/ __/ __\\  /   \\   /  \\ \\ /\\ / /__ \\X _/ \n\
                \x20 \\ X    | | | | |__ | __X  | X || |    \\ V  V // _ | |   \n\
                \x20  \\ \\__/\\ __X_| \\___/___/  \\___/| |     \\ /\\ / \\___X_|   \n\
                \x20   \\____/                       |/       V  V            \n";
    println!("{}", logo);
    println!("  {}", lang.t(TextKey::WrittenBy));
    println!();
    println!("  curseofwar — version {}", VERSION);
    println!();
    println!("{}", lang.t(TextKey::CmdLineHeading));
    println!();

    match lang {
        Lang::Zh => print_help_zh(),
        Lang::En => print_help_en(),
    }
}

fn print_help_zh() {
    println!(
        "  -W 宽         地图宽度（默认 21，最小 {}，最大 {}）
  -H 高         地图高度（默认 21，最小 {}，最大 {}）
  -S 形状       rhombus|rect|hex（默认 rect；菱形/矩形最多 4 国，六边形最多 6 国）
  -l 国家数     [2..N]，N 为形状最大国家数
  -i 不等度     [0..4]，0 最低 4 最高
  -q 出生质量   [1..L]，1=最佳 L=最差（仅单机）
  -r            完全随机初始条件（覆盖 -l/-i/-q）
  -d 难度       ee|e|n|h|hh（默认 n 正常）
  -s 速度       p|sss|ss|s|n|f|ff|fff（默认 n）
  -R 种子       地图生成种子（无符号整数）
  -T            显示时间线
  -E [1..L]     启动服务器等待最多 L 个客户端（多人，暂未实现）
  -e 端口       服务器端口（默认 19140）
  -C IP         启动客户端连接到服务器（多人，暂未实现）
  -c 端口       客户端端口（默认 19150）
  -v            显示版本
  -h            显示本帮助
  --lang zh|en  语言（默认 zh；也可由 COW_LANG 或 LANG 环境变量指定）
",
        MIN_DIM, MAX_DIM_W, MIN_DIM, MAX_DIM_H
    );
}

fn print_help_en() {
    println!(
        "  -W width      map width (default 21, min {}, max {})
  -H height     map height (default 21, min {}, max {})
  -S shape      rhombus|rect|hex (rect is default; rhombus/rect max 4 countries, hex max 6)
  -l locations  number of countries [2..N]
  -i inequality [0..4], 0 = lowest, 4 = highest
  -q quality    [1..L], 1 = the best spawn, L = the worst (single-player only)
  -r            absolutely random initial conditions (overrides -l/-i/-q)
  -d difficulty ee|e|n|h|hh (default n)
  -s speed      p|sss|ss|s|n|f|ff|fff (default n)
  -R seed       unsigned integer map-generation seed
  -T            show the timeline
  -E [1..L]     start a server for up to L clients (multiplayer — not yet implemented)
  -e port       server port (default 19140)
  -C IP         start a client connecting to IP (multiplayer — not yet implemented)
  -c port       client port (default 19150)
  -v            show version
  -h            show this help
  --lang zh|en  language (default zh; also from $COW_LANG or $LANG)
",
        MIN_DIM, MAX_DIM_W, MIN_DIM, MAX_DIM_H
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<const N: usize>(a: [&str; N]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn version_returns_1() {
        let code = run_with_args(&args(["curseofwar", "-v"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn help_returns_1() {
        let code = run_with_args(&args(["curseofwar", "-h"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn invalid_loc_returns_1() {
        let code = run_with_args(&args(["curseofwar", "-l", "9"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn multiplayer_returns_1_with_notice() {
        let code = run_with_args(&args(["curseofwar", "-E", "2"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn unknown_short_returns_1() {
        let code = run_with_args(&args(["curseofwar", "-Z"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn lang_override_to_en() {
        // -h forces a 1 exit; we mainly check that the binary does not panic
        // when --lang is provided.
        let code = run_with_args(&args(["curseofwar", "--lang", "en", "-h"]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn parse_lang_handles_invalid() {
        assert_eq!(parse_lang("zh_CN"), Some(Lang::Zh));
        assert_eq!(parse_lang("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(parse_lang("fr"), None);
    }
}
