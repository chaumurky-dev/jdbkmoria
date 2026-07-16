# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rmoria` is a Rust port of **Umoria 5.7.15**, translated from the upstream C++ sources at
<https://github.com/dungeons-of-moria/umoria>. Those sources are read-only reference —
consult them to verify behavior, never a build target.

## Commands

```sh
cargo build --release        # build
cargo build                  # debug build (required before the smoke test)
cargo test                   # unit + integration tests (tests/save_roundtrip.rs)
cargo test --test save_roundtrip     # a single integration test target
cargo clippy                 # kept at ZERO warnings — treat any warning as a regression
python3 tools/smoke_test.py  # end-to-end PTY smoke test (needs a debug build first)
```

Run: `cargo run -- [OPTIONS] SAVEGAME` (e.g. `-n` new game, `-s NUMBER` fixed seed,
`-d` show high scores, `-v` version). Save default is `game.sav`.

## Architecture

This is a **faithful, gameplay-identical translation** of the C++ sources — this constraint
drives every structural decision. See `PORTING.md` for the full file-by-file mapping and
conventions. Deliberate gameplay additions (see "Extensions beyond upstream" in
`PORTING.md`, e.g. `paintings.rs`) live in their own modules, and every hook they need in
ported code is marked with an `rmoria extension` comment — keep new extensions to that
pattern.

- **One Rust module per C++ source file** (roughly), same names, C++ `camelCase`
  functions → `snake_case`. When changing logic, cross-check the corresponding
  upstream `src/*.cpp` file (see `PORTING.md` for the mapping) to preserve behavior.
- **Global mutable state via `RacyCell` statics.** The C++ code keeps all game state in
  file-scope globals; the port mirrors this with `RacyCell<T>` (an `UnsafeCell` wrapper in
  `globals.rs` that hands out `&mut` from a shared ref). State is reached through accessor
  functions: `game()` (game.rs), `dg()` (dungeon.rs), `py()` (player.rs),
  `monsters()` (monster.rs), and similar. **The game is strictly single-threaded — do not
  add threads.** The safety invariant is that no returned reference is held across a call
  that could touch the same global another way.
- **Crate split:** `src/lib.rs` (library, `pub mod` for every module) + thin `src/main.rs`
  (`main()` + `parse_game_seed()`). This lets integration tests exercise game logic without
  a terminal — e.g. `save_roundtrip.rs` drives `pub` non-curses cores factored out of
  save/restore. Because state is process-wide, such tests live in a single `#[test] fn`.
- **Static data tables** (`data_*.rs`, `*_data.rs`) were machine-converted from the C
  sources and verified value-for-value. Do not hand-edit them casually; a mismatch changes
  gameplay.
- **UI** goes through `pancurses` (`ui_io.rs`, `ui.rs`); the RNG (`rng.rs`) has a reference
  test asserting parity with the original generator.
- **Save format** is umoria 5.2.2+ compatible (xor-encrypted), implemented in `game_save.rs`.

## Working conventions

- Preserve the original copyright header and `SPDX-License-Identifier: GPL-3.0-or-later` on
  every file. The whole project is GPL-3.0-or-later.
- Keep `cargo clippy` clean. A handful of C-mirroring lints
  (`needless_range_loop`, `manual_range_contains`, `collapsible_if`, `collapsible_match`)
  are allowed crate-wide in `lib.rs` on purpose — don't "fix" them by rewriting loops or
  conditions near RNG/game-logic calls, as that risks reordering side effects.
- Once the repo has a remote, use feature branches + PRs rather than pushing to `main`.
