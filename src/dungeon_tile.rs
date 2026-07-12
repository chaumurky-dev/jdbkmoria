// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Tile holds data about a specific tile in the dungeon.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub creature_id: u8, // ID for any creature occupying the tile
    pub treasure_id: u8, // ID for any treasure item occupying the tile
    pub feature_id: u8,  // ID of cave feature; walls, floors, open space, etc.

    pub perma_lit_room: bool,  // Room should be lit with perm light, walls with this set should be perm lit after tunneled out.
    pub field_mark: bool,      // Field mark, used for traps/doors/stairs, object is hidden if fm is false.
    pub permanent_light: bool, // Permanent light, used for walls and lighted rooms.
    pub temporary_light: bool, // Temporary light, used for player's lamp light,etc.
}

impl Tile {
    pub const fn empty() -> Self {
        Tile {
            creature_id: 0,
            treasure_id: 0,
            feature_id: 0,
            perma_lit_room: false,
            field_mark: false,
            permanent_light: false,
            temporary_light: false,
        }
    }
}

// `fval` definitions: these describe the various types of dungeon floors and
// walls, if numbers above 15 are ever used, then the test against MIN_CAVE_WALL
// will have to be changed, also the save routines will have to be changed.
pub const TILE_NULL_WALL: u8 = 0;
pub const TILE_DARK_FLOOR: u8 = 1;
pub const TILE_LIGHT_FLOOR: u8 = 2;
pub const MAX_CAVE_ROOM: u8 = 2;
pub const TILE_CORR_FLOOR: u8 = 3;
pub const TILE_BLOCKED_FLOOR: u8 = 4; // a corridor space with cl/st/se door or rubble
pub const MAX_CAVE_FLOOR: u8 = 4;

pub const MAX_OPEN_SPACE: u8 = 3;
pub const MIN_CLOSED_SPACE: u8 = 4;

pub const TMP1_WALL: u8 = 8;
pub const TMP2_WALL: u8 = 9;

pub const MIN_CAVE_WALL: u8 = 12;
pub const TILE_GRANITE_WALL: u8 = 12;
pub const TILE_MAGMA_WALL: u8 = 13;
pub const TILE_QUARTZ_WALL: u8 = 14;
pub const TILE_BOUNDARY_WALL: u8 = 15;
