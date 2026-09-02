# Curse of War — Rust 重写版

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](./LICENSE)
[![English](](https://img.shields.io/badge/lang-English-blue.svg)](./README.en.md)

> 一款终端即时策略游戏，本仓库是用 **Rust** 对 [Curse of War 1.3.0](https://github.com/a-nikolaev/curseofwar)（原作者 Alexey Nikolaev，2013）的忠实重写，并自带中文界面。

这是一款六边形网格上的快节奏策略游戏：你不是操控每个单位，而是做高层规划——建造基础设施、争夺资源、移动军队。游戏机制非常贴近一战/二战时期的战争形态，但并不指涉任何具体历史时期。

![demo](exp/image/demo.png)

## 玩法

游戏开始时，每个国家在地图的角落占据一个城堡和附近的两个金矿。地图由六边形地块组成：

| 符号 | 地形 | 含义 |
|---|---|---|
| `/\^` | 山 | 不可居住，阻挡军队 |
| `/$\` | 金矿 | 周围的格子都归你时，每步为你产出金币 |
| `-`  | 草地 | 普通可居住地块，人口密度用点阵密度表示 |
| `n`  | 村庄 | 人口增长 +10% |
| `i=i`| 城镇 | 人口增长 +20% |
| `W#W`| 城堡 | 人口增长 +30% |

人口是你的首要资源。每格最多容纳 499 人；当多国军队在同一格相遇时会战斗。人口**会自然迁徙**——在你的国家插旗的地方聚集。如果你控制一座城市，你会控制其周围领土；如果城市被严重攻击，它会逐级降级（城堡→城镇→村庄→草地）。

### 键位

| 键 | 作用 |
|---|---|
| `H` `J` `K` `L` / 方向键 | 移动光标（六边形网格） |
| `空格` | 在光标处插/拔旗（旗会吸引人口聚集） |
| `R` / `V` | 建造（草地→村庄 160 金 → 城镇 240 金 → 城堡 320 金） |
| `X` | 清除所有你方旗帜 |
| `C` | 清除一半你方旗帜 |
| `F` / `S` | 加速 / 减速 |
| `P` | 暂停 / 恢复 |
| `Q` | 退出（弹出 Y/N 确认） |

胜利条件：消灭所有敌方人口。如果你的国家人口降到 0 则战败。

## 命令行参数

忠实复刻原版参数集：

```
-W 宽度   -H 高度   -S 形状 (rhombus|rect|hex)
-l 国家数  -i 不等度 (0-4)  -q 出生质量
-r  -d 难度 (ee|e|n|h|hh)  -s 速度 (p|sss|ss|s|n|f|ff|fff)
-R 种子  -T 时间线  -v  -h
-E/-e/-C/-c   多人（本版本未实现，启动时友好提示）
--lang zh|en   语言（默认中文，可由 $COW_LANG 或 $LANG 覆盖）
```

几个常用示例：

```bash
curseofwar                          # 默认参数，中文界面开局
curseofwar --lang en                # 英文界面
curseofwar -W 18 -H 18 -R 7         # 小地图 + 固定种子（地图可复现）
curseofwar -i 4 -q 1 -d ee          # 不等度 4、玩家出生点最好、最易 AI（推荐新手）
```

## 与原版的差异

本仓库是**忠实重写**——所有游戏规则、命令行参数与原版保持一致。但作为独立发行版，下述差异是**有意为之**：

- **仅终端 TUI**：用 ratatui + crossterm 取代原版的 ncurses + SDL1 双前端
- **无多人联网**：原版 UDP client/server 仅解析并友好提示，未实现
- **不同的 RNG**：用 `StdRng`（ChaCha12）替代 `glibc rand()`。同种子在本实现内可复现，但**不会**与原版 C 程序产生同一张地图
- **默认中文**：所有界面文案已本地化；可随时切回英文
- **许可证**：GPLv3（衍生作品）

## 致谢与许可

本项目是 [Curse of War](https://github.com/a-nikolaev/curseofwar) 的衍生作品，根据 GPLv3 第 5 节发布。每个源文件头均保留原作者版权与衍生声明。

```
Curse of War -- Real Time Strategy Game for Linux.
Copyright (C) 2013 Alexey Nikolaev.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

完整许可证文本见仓库根目录的 [`LICENSE`](./LICENSE) 文件。