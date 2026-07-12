// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Game object management

use crate::config;
use crate::data_treasure::GAME_OBJECTS;
use crate::dungeon::{dg, DungeonObject};
use crate::game::{game, random_number, sorted_objects, treasure_levels, LEVEL_MAX_OBJECTS, TREASURE_MAX_LEVELS};
use crate::inventory::inventory_item_copy_to;
use crate::player::py;
use crate::treasure::*;
use crate::types::Coord;
use crate::ui::draw_dungeon_panel;
use crate::ui_io::print_message;

// If too many objects on floor level, delete some of them-RAK-
fn compact_objects() {
    print_message(Some("Compacting objects..."));

    let mut counter = 0;
    let mut current_distance = 66;

    while counter <= 0 {
        for y in 0..dg().height as i32 {
            for x in 0..dg().width as i32 {
                let coord = Coord::new(y, x);
                let treasure_id = dg().tile(coord).treasure_id;

                if treasure_id != 0 && crate::dungeon::coord_distance_between(coord, py().pos) > current_distance {
                    let chance = match game().treasure.list[treasure_id as usize].category_id {
                        TV_VIS_TRAP => 15,
                        TV_INVIS_TRAP | TV_RUBBLE | TV_OPEN_DOOR | TV_CLOSED_DOOR => 5,
                        // Stairs, don't delete them.
                        // Shop doors, don't delete them.
                        TV_UP_STAIR | TV_DOWN_STAIR | TV_STORE_DOOR => 0,
                        // secret doors
                        TV_SECRET_DOOR => 3,
                        _ => 10,
                    };

                    if random_number(100) <= chance {
                        crate::dungeon::dungeon_delete_object(coord);
                        counter += 1;
                    }
                }
            }
        }

        if counter == 0 {
            current_distance -= 6;
        }
    }

    if current_distance < 66 {
        draw_dungeon_panel();
    }
}

// Gives pointer to next free space -RAK-
pub fn popt() -> i32 {
    if game().treasure.current_id as usize == LEVEL_MAX_OBJECTS {
        compact_objects();
    }

    let id = game().treasure.current_id;
    game().treasure.current_id += 1;
    id as i32
}

// Pushes a record back onto free space list -RAK-
// `dungeon_delete_object()` should always be called instead, unless the object
// in question is not in the dungeon, e.g. in store1.c and files.c
pub fn pusht(treasure_id: u8) {
    if treasure_id as i16 != game().treasure.current_id - 1 {
        let last = (game().treasure.current_id - 1) as usize;
        game().treasure.list[treasure_id as usize] = game().treasure.list[last];

        // must change the treasure_id in the cave of the object just moved
        for y in 0..dg().height as usize {
            for x in 0..dg().width as usize {
                if dg().floor[y][x].treasure_id as i16 == game().treasure.current_id - 1 {
                    dg().floor[y][x].treasure_id = treasure_id;
                }
            }
        }
    }
    game().treasure.current_id -= 1;

    let current = game().treasure.current_id as usize;
    inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut game().treasure.list[current]);
}

// Item too large to fit in chest? -DJG-
// Use a DungeonObject since the item has not yet been created
fn item_bigger_than_chest(obj: &DungeonObject) -> bool {
    match obj.category_id {
        TV_CHEST | TV_BOW | TV_POLEARM | TV_HARD_ARMOR | TV_SOFT_ARMOR | TV_STAFF => true,
        TV_HAFTED | TV_SWORD | TV_DIGGING => obj.weight > 150,
        _ => false,
    }
}

// Returns the array number of a random object -RAK-
pub fn item_get_random_object_id(level: i32, must_be_small: bool) -> i32 {
    if level == 0 {
        return random_number(treasure_levels()[0] as i32) - 1;
    }

    let mut level = level;

    if level >= TREASURE_MAX_LEVELS as i32 {
        level = TREASURE_MAX_LEVELS as i32;
    } else if random_number(config::treasure::TREASURE_CHANCE_OF_GREAT_ITEM as i32) == 1 {
        level = level * TREASURE_MAX_LEVELS as i32 / random_number(TREASURE_MAX_LEVELS as i32) + 1;
        if level > TREASURE_MAX_LEVELS as i32 {
            level = TREASURE_MAX_LEVELS as i32;
        }
    }

    let mut object_id;

    // This code has been added to make it slightly more likely to get the
    // higher level objects.  Originally a uniform distribution over all
    // objects less than or equal to the dungeon level. This distribution
    // makes a level n objects occur approx 2/n% of the time on level n,
    // and 1/2n are 0th level.
    loop {
        if random_number(2) == 1 {
            object_id = random_number(treasure_levels()[level as usize] as i32) - 1;
        } else {
            // Choose three objects, pick the highest level.
            object_id = random_number(treasure_levels()[level as usize] as i32) - 1;

            let j = random_number(treasure_levels()[level as usize] as i32) - 1;

            if object_id < j {
                object_id = j;
            }

            let j = random_number(treasure_levels()[level as usize] as i32) - 1;

            if object_id < j {
                object_id = j;
            }

            let found_level = GAME_OBJECTS[sorted_objects()[object_id as usize] as usize].depth_first_found as usize;

            if found_level == 0 {
                object_id = random_number(treasure_levels()[0] as i32) - 1;
            } else {
                object_id = random_number(treasure_levels()[found_level] as i32 - treasure_levels()[found_level - 1] as i32) - 1
                    + treasure_levels()[found_level - 1] as i32;
            }
        }

        if !(must_be_small && item_bigger_than_chest(&GAME_OBJECTS[sorted_objects()[object_id as usize] as usize])) {
            break;
        }
    }

    object_id
}
