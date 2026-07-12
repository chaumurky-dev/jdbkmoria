// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::game::random_number;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dice {
    pub dice: u8,
    pub sides: u8,
}

impl Dice {
    pub const fn new(dice: u8, sides: u8) -> Self {
        Dice { dice, sides }
    }
}

// generates damage for 2d6 style dice rolls
pub fn dice_roll(dice: Dice) -> i32 {
    let mut sum = 0;
    for _ in 0..dice.dice {
        sum += random_number(dice.sides as i32);
    }
    sum
}

// Returns max dice roll value -RAK-
pub fn max_dice_roll(dice: Dice) -> i32 {
    dice.dice as i32 * dice.sides as i32
}
