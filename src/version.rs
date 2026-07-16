// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Umoria base/save-format version. These are written into save and score files
// for compatibility checks — do not change them when releasing jdbkmoria versions.
pub const CURRENT_VERSION_MAJOR: u8 = 5;
pub const CURRENT_VERSION_MINOR: u8 = 7;
pub const CURRENT_VERSION_PATCH: u8 = 15;

// User-facing jdbkmoria version
pub const JDBK_VERSION_MAJOR: u8 = 0;
pub const JDBK_VERSION_MINOR: u8 = 1;
pub const JDBK_VERSION_PATCH: u8 = 0;

// jdbkmoria extension: the on-disk save-file binary format version, tracked
// separately from the displayed game version above. The displayed version
// stays pinned to 5.7.15 (matching upstream Umoria and what `-v`/the splash
// screen show); this triple instead versions the save file's byte layout,
// which changed incompatibly when the dungeon dimensions were doubled
// (coordinates widened from a byte to a short - see game_save.rs). Bump the
// major component whenever the layout changes in a way old saves can't be
// read as; `game_save::valid_save_format_version` requires an exact major
// match. This is deliberately distinct from `valid_game_version` in game.rs,
// which still governs the (unrelated, still-5.x) high-score file format in
// scores.rs.
pub const SAVE_FORMAT_MAJOR: u8 = 6;
pub const SAVE_FORMAT_MINOR: u8 = 0;
pub const SAVE_FORMAT_PATCH: u8 = 0;
