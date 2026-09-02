# Curse of War — Rust Re-implementation

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](./LICENSE)
[![中文文档](](https://img.shields.io/badge/中文-README-red.svg)](./README.cn.md))
[![English](](https://img.shields.io/badge/English-README-blue.svg)](./README.en.md))

![demo](exp/image/demo.png)

A faithful Rust port of [Curse of War 1.3.0](https://github.com/a-nikolaev/curseofwar) by Alexey Nikolaev (2013), with a **Chinese UI by default**.

Curse of War is a fast-paced strategy game on a hex grid: you don't command individual units — you plan at a high level (build infrastructure, secure resources, move armies).

## ⚠️ Disclaimer

> **This project is a personal learning exercise only.**
> It is a non-commercial, educational Rust re-implementation of the original [Curse of War](https://github.com/a-nikolaev/curseofwar) by Alexey Nikolaev, created for the purpose of studying game development, Rust programming, and terminal UI techniques. No part of this project is intended for commercial use, distribution, or any purpose other than learning and personal study.

## Documentation

- **[中文文档](./README.cn.md)** — 玩法、键位、命令行参数、与原版的差异
- **[English](./README.en.md)** — Gameplay, key bindings, command-line options, differences from the original

## Repository layout

```
curseofwar-rs/
├── README.md            this file (entry point)
├── README.cn.md         Chinese documentation
├── README.en.md         English documentation
├── LICENSE              GPLv3-or-later full text
├── Cargo.toml           Cargo workspace manifest
└── crates/
    ├── cow-core/        game-logic library (no UI/IO deps)
    └── cow-tui/         terminal front-end (ratatui + crossterm)
```

## License

GPL-3.0-or-later. Derived from [Curse of War](https://github.com/a-nikolaev/curseofwar) by Alexey Nikolaev (2013). See [`LICENSE`](./LICENSE).