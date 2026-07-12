// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::globals::RacyCell;

// defines for treasure type values (tval)
pub const TV_NEVER: i8 = -1; // used by find_range() for non-search
pub const TV_NOTHING: u8 = 0;
pub const TV_MISC: u8 = 1;
pub const TV_CHEST: u8 = 2;

// min tval for wearable items, all items between TV_MIN_WEAR and
// TV_MAX_WEAR use the same flag bits, see the TR_* defines.
pub const TV_MIN_WEAR: u8 = 10;
// items tested for enchantments, i.e. the MAGIK inscription, see the enchanted() procedure.
pub const TV_MIN_ENCHANT: u8 = 10;
pub const TV_SLING_AMMO: u8 = 10;
pub const TV_BOLT: u8 = 11;
pub const TV_ARROW: u8 = 12;
pub const TV_SPIKE: u8 = 13;
pub const TV_LIGHT: u8 = 15;
pub const TV_BOW: u8 = 20;
pub const TV_HAFTED: u8 = 21;
pub const TV_POLEARM: u8 = 22;
pub const TV_SWORD: u8 = 23;
pub const TV_DIGGING: u8 = 25;
pub const TV_BOOTS: u8 = 30;
pub const TV_GLOVES: u8 = 31;
pub const TV_CLOAK: u8 = 32;
pub const TV_HELM: u8 = 33;
pub const TV_SHIELD: u8 = 34;
pub const TV_HARD_ARMOR: u8 = 35;
pub const TV_SOFT_ARMOR: u8 = 36;
// max tval that uses the TR_* flags
pub const TV_MAX_ENCHANT: u8 = 39;
pub const TV_AMULET: u8 = 40;
pub const TV_RING: u8 = 45;
pub const TV_MAX_WEAR: u8 = 50; // max tval for wearable items

pub const TV_STAFF: u8 = 55;
pub const TV_WAND: u8 = 65;
pub const TV_SCROLL1: u8 = 70;
pub const TV_SCROLL2: u8 = 71;
pub const TV_POTION1: u8 = 75;
pub const TV_POTION2: u8 = 76;
pub const TV_FLASK: u8 = 77;
pub const TV_FOOD: u8 = 80;
pub const TV_MAGIC_BOOK: u8 = 90;
pub const TV_PRAYER_BOOK: u8 = 91;
pub const TV_MAX_OBJECT: u8 = 99; // objects with tval above this are never picked up by monsters
pub const TV_GOLD: u8 = 100;
pub const TV_MAX_PICK_UP: u8 = 100; // objects with higher tvals can not be picked up
pub const TV_INVIS_TRAP: u8 = 101;

// objects between TV_MIN_VISIBLE and TV_MAX_VISIBLE are always visible,
// i.e. the cave fm flag is set when they are present
pub const TV_MIN_VISIBLE: u8 = 102;
pub const TV_VIS_TRAP: u8 = 102;
pub const TV_RUBBLE: u8 = 103;
// following objects are never deleted when trying to create another one during level generation
pub const TV_MIN_DOORS: u8 = 104;
pub const TV_OPEN_DOOR: u8 = 104;
pub const TV_CLOSED_DOOR: u8 = 105;
pub const TV_UP_STAIR: u8 = 107;
pub const TV_DOWN_STAIR: u8 = 108;
pub const TV_SECRET_DOOR: u8 = 109;
pub const TV_STORE_DOOR: u8 = 110;
pub const TV_MAX_VISIBLE: u8 = 110;

static MISSILES_COUNTER: RacyCell<i16> = RacyCell::new(0);

pub fn missiles_counter() -> &'static mut i16 {
    MISSILES_COUNTER.get()
}
