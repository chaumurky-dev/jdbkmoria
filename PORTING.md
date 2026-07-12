# Porting status: umoria (C++) → rmoria (Rust)

Source: `../umoria/src` at version 5.7.15. Goal: gameplay-identical port, GPL-3.0-or-later.

## Conventions

- One Rust module per C++ source file (roughly). C++ `camelCase` functions become `snake_case`.
- Global game state (`game`, `dg`, `py`, `monsters`, ...) lives in `RacyCell` statics
  (see `globals.rs`) with accessor functions: `game()`, `dg()`, `py()`, `monsters()`, etc.
  The game is single threaded; do not add threads.
- Static data tables were machine-converted from the C sources (scripts preserved in git
  history / scratchpad) and verified value-for-value against the originals.
- `Coord_t` → `types::Coord`, `Dice_t` → `dice::Dice`, `vtype_t`/`obj_desc_t` → `String`.
- Original copyright headers and SPDX `GPL-3.0-or-later` identifiers preserved per file.
- The crate is split into a library (`src/lib.rs`, `pub mod` for every module) and a thin
  binary (`src/main.rs`, just `main()`/`parse_game_seed()`), so integration tests and any
  future consumers can exercise game logic directly. `tests/save_roundtrip.rs` seeds the
  RNG, pokes known values into the global state (player misc/stats, a couple of inventory
  items, monster recall memory, `game().character_died_from`), then round-trips them
  through `game_save::save_game_state_to_file()` / `load_game_state_from_file()` - small
  `pub` non-interactive cores factored out of `save_char()`/`restore_from_file()` that
  contain no curses calls, so the test runs without a terminal. Everything lives in one
  `#[test] fn` since the game state is process-wide (see `globals.rs`).
- `cargo clippy` is kept at zero warnings; a handful of pervasive, intentionally
  C-mirroring lints (`needless_range_loop`, `manual_range_contains`, `collapsible_if`,
  `collapsible_match`) are allowed crate-wide in `lib.rs` rather than hand-fixed, to avoid
  any risk of reordering RNG/game-logic calls.

## Status

**PORT COMPLETE (2026-07-12).** All 40 C++ source files ported; builds clean;
RNG reference test passes; verified end-to-end in a PTY (character creation,
town generation, movement, quit/tombstone, save + restore roundtrip).
Save files use the umoria 5.2.2+ compatible xor-encrypted format.

| C++ file | Rust module | Status |
|---|---|---|
| rng.cpp | rng.rs | done (reference test passes) |
| dice.cpp | dice.rs | done |
| helpers.cpp | helpers.rs | done |
| config.cpp/h | config.rs | done |
| types.h | types.rs | done |
| version.h | version.rs | done |
| game.h/cpp | game.rs | done |
| dungeon_tile.h | dungeon_tile.rs | done |
| dungeon.h/cpp | dungeon.rs | done |
| player.h/cpp | player.rs | done |
| monster.h/cpp | monster.rs | done |
| inventory.h/cpp | inventory.rs | done |
| treasure.h | treasure.rs | done (treasure.cpp -> treasure_magic.rs) |
| character.h/cpp | character.rs | done |
| recall.h/cpp | recall_data.rs + recall.rs | done |
| store.h | store_data.rs | done + verified |
| scores.h | scores.rs | done (high score read/write via game_save.rs xor I/O, display screen, scoring) |
| spells.h | spells_data.rs | done + verified |
| identification.h/cpp | identification.rs | done |
| data_creatures.cpp | data_creatures.rs | done + verified |
| data_treasure.cpp | data_treasure.rs | done + verified |
| data_player.cpp | data_player.rs | done + verified |
| data_recall.cpp | data_recall.rs | done + verified |
| data_stores.cpp | data_stores.rs | done + verified |
| data_store_owners.cpp | data_store_owners.rs | done + verified |
| data_tables.cpp | data_tables.rs | done + verified |
| ui_io.cpp | ui_io.rs | done (pancurses) |
| ui.h + ui.cpp | ui.rs | done |
| main.cpp | main.rs | done |
| game_files.cpp | game_files.rs | done |
| game_save.cpp | game_save.rs | done (save/load game state, xor byte I/O, high score I/O; builds clean) |
| game_death.cpp | game_death.rs | done |
| game_run.cpp | game_run.rs | done |
| game_objects.cpp | game_objects.rs | done |
| ui_inventory.cpp | ui_inventory.rs | done |
| dungeon_los.cpp | dungeon_los.rs | done |
| treasure.cpp | treasure_magic.rs | done |
| dungeon_generate.cpp | dungeon_generate.rs | done |
| monster_manager.cpp | monster_manager.rs | done |
| player_*.cpp | player_*.rs | done |
| spells.cpp | spells.rs | done |
| mage_spells.cpp | mage_spells.rs | done |
| scrolls.cpp | scrolls.rs | done |
| staves.cpp | staves.rs | done |
| store.cpp | store.rs | done |
| store_inventory.cpp | store_inventory.rs | done |
| wizard.cpp | wizard.rs | done |
