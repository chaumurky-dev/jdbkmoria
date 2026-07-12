// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::globals::RacyCell;
use crate::monster::{MON_MAX_ATTACKS, MON_MAX_CREATURES};

// Recall holds the player's known knowledge for any given monster, aka memories
#[derive(Debug, Clone, Copy, Default)]
pub struct Recall {
    pub movement: u32,
    pub spells: u32,
    pub kills: u16,
    pub deaths: u16,
    pub defenses: u16,
    pub wake: u8,
    pub ignore: u8,
    pub attacks: [u8; MON_MAX_ATTACKS],
}

impl Recall {
    pub const fn empty() -> Self {
        Recall {
            movement: 0,
            spells: 0,
            kills: 0,
            deaths: 0,
            defenses: 0,
            wake: 0,
            ignore: 0,
            attacks: [0; MON_MAX_ATTACKS],
        }
    }
}

// Monster memories. -CJS-
static CREATURE_RECALL: RacyCell<[Recall; MON_MAX_CREATURES]> = RacyCell::new([Recall::empty(); MON_MAX_CREATURES]);

pub fn creature_recall() -> &'static mut [Recall; MON_MAX_CREATURES] {
    CREATURE_RECALL.get()
}
