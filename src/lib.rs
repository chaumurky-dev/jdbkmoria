// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Library crate root: re-exports every module so the binary crate (main.rs)
// and integration tests (tests/*.rs) can use them.

// This is a faithful, gameplay-identical port of the original C++ umoria
// sources (see PORTING.md). The following clippy style lints are allowed
// crate-wide because "fixing" them either risks changing evaluation order
// around game-logic/RNG calls, or would fight the deliberate C-mirroring
// style used throughout the port (index-based loops over the same tables
// the C code indexed, C-style range checks, nested ifs matching the
// original control flow):
//
// - `needless_range_loop`: many loops walk two or more parallel arrays by
//   index (mirroring the C `for (i = 0; i < N; i++)` idiom), so converting
//   to `.iter().enumerate()` would not simplify anything and risks subtle
//   off-by-one mistakes across dozens of call sites.
// - `manual_range_contains`: several of these comparisons sit directly
//   beside `random_number()`/game-logic calls; rewriting them is safe in
//   isolation, but is skipped uniformly here to avoid touching that code at
//   all, per the port's zero-behavior-change policy.
// - `collapsible_if` / `collapsible_match`: a few of the nested `if`s guard
//   `random_number()` or other side-effecting game-logic calls in their
//   condition or body; collapsing is left undone uniformly rather than
//   judgment-calling which specific instances are "safe" to merge.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

pub mod character;
pub mod config;
pub mod data_creatures;
pub mod data_paintings;
pub mod data_player;
pub mod data_recall;
pub mod data_store_owners;
pub mod data_stores;
pub mod data_tables;
pub mod data_treasure;
pub mod dice;
pub mod dungeon;
pub mod dungeon_generate;
pub mod dungeon_los;
pub mod dungeon_tile;
pub mod game;
pub mod game_death;
pub mod game_files;
pub mod game_objects;
pub mod game_run;
pub mod game_save;
pub mod globals;
pub mod helpers;
pub mod identification;
pub mod inventory;
pub mod locale;
pub mod locale_data_en_gb;
pub mod locale_data_fr_ca;
pub mod mage_spells;
pub mod monster;
pub mod monster_manager;
pub mod paintings;
pub mod player;
pub mod player_bash;
pub mod player_eat;
pub mod player_magic;
pub mod player_move;
pub mod player_pray;
pub mod player_quaff;
pub mod player_run;
pub mod player_stats;
pub mod player_throw;
pub mod player_traps;
pub mod player_tunnel;
pub mod recall;
pub mod recall_data;
pub mod rng;
pub mod scores;
pub mod scrolls;
pub mod spells;
pub mod spells_data;
pub mod staves;
pub mod store;
pub mod store_data;
pub mod store_inventory;
pub mod treasure;
pub mod treasure_magic;
pub mod types;
pub mod ui;
pub mod ui_inventory;
pub mod ui_io;
pub mod version;
pub mod wizard;
