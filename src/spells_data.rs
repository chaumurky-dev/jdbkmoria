// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// spell types used by get_flags(), breathe(), fire_bolt() and fire_ball()
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicSpellFlags {
    MagicMissile = 0,
    Lightning,
    PoisonGas,
    Acid,
    Frost,
    Fire,
    HolyOrb,
}

// Spell is a base data object.
// Holds the base game data for a spell
// Note: the names for the spells are stored in spell_names[] array at index i, +31 if priest
#[derive(Debug, Clone, Copy)]
pub struct Spell {
    pub level_required: u8,
    pub mana_required: u8,
    pub failure_chance: u8,
    pub exp_gain_for_learning: u8, // 1/4 of exp gained for learning spell
}
