// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Many of the character fields used to be fixed length, which
// greatly increased the size of the executable. Many fixed
// length fields have been replaced with variable length ones.
pub const MORIA_MESSAGE_SIZE: usize = 80;

// Note: since its output can easily exceed 80 characters,
// an object description must always be called with an
// obj_desc_t type as the first parameter.
pub const MORIA_OBJ_DESC_SIZE: usize = 160;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Coord {
    pub y: i32,
    pub x: i32,
}

impl Coord {
    pub const fn new(y: i32, x: i32) -> Self {
        Coord { y, x }
    }
}
