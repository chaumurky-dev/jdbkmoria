// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::dice::Dice;
use crate::dungeon_tile::Tile;
use crate::globals::RacyCell;
use crate::ui::Panel;

// Dungeon size parameters
pub const RATIO: i32 = 3; // Size ratio of the Map screen
pub const MAX_HEIGHT: i32 = 66; // Multiple of 11; >= 22
pub const MAX_WIDTH: i32 = 198; // Multiple of 33; >= 66
pub const SCREEN_HEIGHT: i32 = 22;
pub const SCREEN_WIDTH: i32 = 66;
pub const QUART_HEIGHT: i32 = SCREEN_HEIGHT / 4;
pub const QUART_WIDTH: i32 = SCREEN_WIDTH / 4;

// DungeonObject is a base data object.
// This holds data for any non-living object in the game such as
// stairs, rubble, doors, gold, potions, weapons, wands, etc.
pub struct DungeonObject {
    pub name: &'static str,      // Object name
    pub flags: u32,              // Special flags
    pub category_id: u8,         // Category number (tval)
    pub sprite: u8,              // Character representation - ASCII symbol (tchar)
    pub misc_use: i16,           // Misc. use variable (p1)
    pub cost: i32,               // Cost of item
    pub sub_category_id: u8,     // Sub-category number (subval)
    pub items_count: u8,         // Number of items
    pub weight: u16,             // Weight
    pub to_hit: i16,             // Plusses to hit
    pub to_damage: i16,          // Plusses to damage
    pub ac: i16,                 // Normal AC
    pub to_ac: i16,              // Plusses to AC
    pub damage: Dice,            // Damage when hits
    pub depth_first_found: u8,   // Dungeon level item first found
}

pub struct Dungeon {
    // Dungeon size is either just big enough for town level, or the whole dungeon itself
    pub height: i16,
    pub width: i16,

    pub panel: Panel,

    // Current turn of the game
    pub game_turn: i32,

    // The current dungeon level
    pub current_level: i16,

    // A `true` value means a new level will be generated on next loop iteration
    pub generate_new_level: bool,

    // Floor definitions
    pub floor: [[Tile; MAX_WIDTH as usize]; MAX_HEIGHT as usize],
}

impl Dungeon {
    pub const fn empty() -> Self {
        Dungeon {
            height: 0,
            width: 0,
            panel: Panel::empty(),
            game_turn: -1,
            current_level: 0,
            generate_new_level: true,
            floor: [[Tile::empty(); MAX_WIDTH as usize]; MAX_HEIGHT as usize],
        }
    }
}

static DG: RacyCell<Dungeon> = RacyCell::new(Dungeon::empty());

// Access the global dungeon object (the C code's `dg` global).
pub fn dg() -> &'static mut Dungeon {
    DG.get()
}

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_treasure::GAME_OBJECTS;
use crate::dungeon_tile::*;
use crate::game::{game, random_number, sorted_objects};
use crate::game_objects::{item_get_random_object_id, popt, pusht};
use crate::inventory::inventory_item_copy_to;
use crate::monster::{monster_multiply_total, monsters, next_free_monster_id, BLANK_MONSTER};
use crate::player::py;
use crate::treasure::*;
use crate::types::Coord;
use crate::ui::coord_inside_panel;
use crate::ui_io::{
    add_char, clear_screen, get_key_input, move_cursor, panel_put_tile, print_message, put_string,
    terminal_restore_screen, terminal_save_screen,
};

// dungeon_display_map shrinks the dungeon to a single screen
pub fn dungeon_display_map() {
    // Save the game screen
    terminal_save_screen();
    clear_screen();

    let mut priority: [i16; 256] = [0; 256];
    priority[60] = 5; // char '<'
    priority[62] = 5; // char '>'
    priority[64] = 10; // char '@'
    priority[35] = -5; // char '#'
    priority[46] = -10; // char '.'
    priority[92] = -3; // char '\'
    priority[32] = -15; // char ' '

    // Display highest priority object in the RATIO, by RATIO area
    let panel_width = (MAX_WIDTH / RATIO) as usize;
    let panel_height = MAX_HEIGHT / RATIO;

    let mut map: Vec<u8> = vec![b' '; panel_width];

    // Add screen border
    add_char('+', Coord::new(0, 0));
    add_char('+', Coord::new(0, panel_width as i32 + 1));
    for i in 0..panel_width as i32 {
        add_char('-', Coord::new(0, i + 1));
        add_char('-', Coord::new(panel_height + 1, i + 1));
    }
    for i in 0..panel_height {
        add_char('|', Coord::new(i + 1, 0));
        add_char('|', Coord::new(i + 1, panel_width as i32 + 1));
    }
    add_char('+', Coord::new(panel_height + 1, 0));
    add_char('+', Coord::new(panel_height + 1, panel_width as i32 + 1));
    put_string("Hit any key to continue", Coord::new(23, 23));

    let mut player_y = 0;
    let mut player_x = 0;
    let mut line: i32 = -1;

    // Shrink the dungeon!
    for y in 0..MAX_HEIGHT {
        let row = y / RATIO;
        if row != line {
            if line >= 0 {
                let line_buffer = format!("|{}|", String::from_utf8_lossy(&map));
                put_string(&line_buffer, Coord::new(line + 1, 0));
            }
            map.iter_mut().for_each(|c| *c = b' ');
            line = row;
        }

        for x in 0..MAX_WIDTH {
            let col = (x / RATIO) as usize;
            let cave_char = cave_get_tile_symbol(Coord::new(y, x));
            if priority[map[col] as usize] < priority[cave_char as u8 as usize] {
                map[col] = cave_char as u8;
            }
            if map[col] == b'@' {
                // +1 to account for border
                player_x = col as i32 + 1;
                player_y = row + 1;
            }
        }
    }

    if line >= 0 {
        let line_buffer = format!("|{}|", String::from_utf8_lossy(&map));
        put_string(&line_buffer, Coord::new(line + 1, 0));
    }

    // Move cursor onto player character
    move_cursor(Coord::new(player_y, player_x));

    // wait for any keypress
    get_key_input();

    // restore the game screen
    terminal_restore_screen();
}

// Checks a co-ordinate for in bounds status -RAK-
pub fn coord_in_bounds(coord: Coord) -> bool {
    let y = coord.y > 0 && coord.y < dg().height as i32 - 1;
    let x = coord.x > 0 && coord.x < dg().width as i32 - 1;

    y && x
}

// Distance between two points -RAK-
pub fn coord_distance_between(from: Coord, to: Coord) -> i32 {
    let dy = (from.y - to.y).abs();
    let dx = (from.x - to.x).abs();

    let a = (dy + dx) << 1;
    let b = if dy > dx { dx } else { dy };

    (a - b) >> 1
}

// Checks points north, south, east, and west for a wall -RAK-
// note that y,x is always coord_in_bounds(), i.e. 0 < y < dg.height-1,
// and 0 < x < dg.width-1
pub fn coord_walls_next_to(coord: Coord) -> i32 {
    let y = coord.y as usize;
    let x = coord.x as usize;

    let mut walls = 0;

    if dg().floor[y - 1][x].feature_id >= MIN_CAVE_WALL {
        walls += 1;
    }

    if dg().floor[y + 1][x].feature_id >= MIN_CAVE_WALL {
        walls += 1;
    }

    if dg().floor[y][x - 1].feature_id >= MIN_CAVE_WALL {
        walls += 1;
    }

    if dg().floor[y][x + 1].feature_id >= MIN_CAVE_WALL {
        walls += 1;
    }

    walls
}

// Checks all adjacent spots for corridors -RAK-
// note that y, x is always coord_in_bounds(), hence no need to check that
// j, k are coord_in_bounds(), even if they are 0 or cur_x-1 is still works
pub fn coord_corridor_walls_next_to(coord: Coord) -> i32 {
    let mut walls = 0;

    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            let tile_id = dg().floor[y as usize][x as usize].feature_id;
            let treasure_id = dg().floor[y as usize][x as usize].treasure_id;

            // should fail if there is already a door present
            if tile_id == TILE_CORR_FLOOR && (treasure_id == 0 || game().treasure.list[treasure_id as usize].category_id < TV_MIN_DOORS) {
                walls += 1;
            }
        }
    }

    walls
}

// Returns symbol for given row, column -RAK-
pub fn cave_get_tile_symbol(coord: Coord) -> char {
    let tile = dg().floor[coord.y as usize][coord.x as usize];

    if tile.creature_id == 1 && (py().running_tracker == 0 || config::options::options().run_print_self) {
        return '@';
    }

    if (py().flags.status & config::player::status::PY_BLIND) != 0 {
        return ' ';
    }

    if py().flags.image > 0 && random_number(12) == 1 {
        return (random_number(95) + 31) as u8 as char;
    }

    if tile.creature_id > 1 && monsters()[tile.creature_id as usize].lit {
        return CREATURES_LIST[monsters()[tile.creature_id as usize].creature_id as usize].sprite as char;
    }

    if !tile.permanent_light && !tile.temporary_light && !tile.field_mark {
        return ' ';
    }

    if tile.treasure_id != 0 && game().treasure.list[tile.treasure_id as usize].category_id != TV_INVIS_TRAP {
        return game().treasure.list[tile.treasure_id as usize].sprite as char;
    }

    if tile.feature_id <= MAX_CAVE_FLOOR {
        return '.';
    }

    if tile.feature_id == TILE_GRANITE_WALL || tile.feature_id == TILE_BOUNDARY_WALL || !config::options::options().highlight_seams {
        return '#';
    }

    // Originally set highlight bit, but that is not portable,
    // now use the percent sign instead.
    '%'
}

// Tests a spot for light or field mark status -RAK-
pub fn cave_tile_visible(coord: Coord) -> bool {
    let tile = &dg().floor[coord.y as usize][coord.x as usize];
    tile.permanent_light || tile.temporary_light || tile.field_mark
}

// Places a particular trap at location y, x -RAK-
pub fn dungeon_set_trap(coord: Coord, sub_type_id: i32) {
    let free_treasure_id = popt() as usize;
    dg().floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
    inventory_item_copy_to(config::dungeon::objects::OBJ_TRAP_LIST as usize + sub_type_id as usize, &mut game().treasure.list[free_treasure_id]);
}

// Change a trap from invisible to visible -RAK-
// Note: Secret doors are handled here
pub fn trap_change_visibility(coord: Coord) {
    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;

    let item = &mut game().treasure.list[treasure_id];

    if item.category_id == TV_INVIS_TRAP {
        item.category_id = TV_VIS_TRAP;
        dungeon_lite_spot(coord);
        return;
    }

    // change secret door to closed door
    if item.category_id == TV_SECRET_DOOR {
        item.id = config::dungeon::objects::OBJ_CLOSED_DOOR;
        item.category_id = GAME_OBJECTS[config::dungeon::objects::OBJ_CLOSED_DOOR as usize].category_id;
        item.sprite = GAME_OBJECTS[config::dungeon::objects::OBJ_CLOSED_DOOR as usize].sprite;
        dungeon_lite_spot(coord);
    }
}

// Places rubble at location y, x -RAK-
pub fn dungeon_place_rubble(coord: Coord) {
    let free_treasure_id = popt() as usize;
    dg().floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
    dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
    inventory_item_copy_to(config::dungeon::objects::OBJ_RUBBLE as usize, &mut game().treasure.list[free_treasure_id]);
}

// Places a treasure (Gold or Gems) at given row, column -RAK-
pub fn dungeon_place_gold(coord: Coord) {
    let free_treasure_id = popt() as usize;

    let mut gold_type_id = ((random_number(dg().current_level as i32 + 2) + 2) / 2) - 1;

    if random_number(config::treasure::TREASURE_CHANCE_OF_GREAT_ITEM as i32) == 1 {
        gold_type_id += random_number(dg().current_level as i32 + 1);
    }

    if gold_type_id >= config::dungeon::objects::MAX_GOLD_TYPES as i32 {
        gold_type_id = config::dungeon::objects::MAX_GOLD_TYPES as i32 - 1;
    }

    dg().floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;
    inventory_item_copy_to(config::dungeon::objects::OBJ_GOLD_LIST as usize + gold_type_id as usize, &mut game().treasure.list[free_treasure_id]);
    let cost = game().treasure.list[free_treasure_id].cost;
    game().treasure.list[free_treasure_id].cost += 8 * random_number(cost) + random_number(8);

    if dg().floor[coord.y as usize][coord.x as usize].creature_id == 1 {
        print_message(Some("You feel something roll beneath your feet."));
    }
}

// Places an object at given row, column co-ordinate -RAK-
pub fn dungeon_place_random_object_at(coord: Coord, must_be_small: bool) {
    let free_treasure_id = popt() as usize;

    dg().floor[coord.y as usize][coord.x as usize].treasure_id = free_treasure_id as u8;

    let object_id = item_get_random_object_id(dg().current_level as i32, must_be_small);
    inventory_item_copy_to(sorted_objects()[object_id as usize] as usize, &mut game().treasure.list[free_treasure_id]);

    crate::treasure_magic::magic_treasure_magical_ability(free_treasure_id as i32, dg().current_level as i32);

    if dg().floor[coord.y as usize][coord.x as usize].creature_id == 1 {
        print_message(Some("You feel something roll beneath your feet.")); // -CJS-
    }
}

// Allocates an object for tunnels and rooms -RAK-
pub fn dungeon_allocate_and_place_object(set_function: fn(i32) -> bool, object_type: i32, number: i32) {
    for _ in 0..number {
        let mut coord = Coord::new(0, 0);

        // don't put an object beneath the player, this could cause
        // problems if player is standing under rubble, or on a trap.
        loop {
            coord.y = random_number(dg().height as i32) - 1;
            coord.x = random_number(dg().width as i32) - 1;

            let tile = &dg().floor[coord.y as usize][coord.x as usize];
            if set_function(tile.feature_id as i32) && tile.treasure_id == 0 && !(coord.y == py().pos.y && coord.x == py().pos.x) {
                break;
            }
        }

        match object_type {
            1 => dungeon_set_trap(coord, random_number(config::dungeon::objects::MAX_TRAPS as i32) - 1),
            // NOTE: object_type == 2 is no longer used - it used to be visible traps.
            // The original C code had no `break`, so case 2 fell through to case 3.
            2 | 3 => dungeon_place_rubble(coord),
            4 => dungeon_place_gold(coord),
            5 => dungeon_place_random_object_at(coord, false),
            _ => {}
        }
    }
}

// Creates objects nearby the coordinates given -RAK-
pub fn dungeon_place_random_object_near(coord: Coord, tries: i32) {
    let mut tries = tries;

    loop {
        let mut i = 0;
        while i <= 10 {
            let at = Coord::new(coord.y - 3 + random_number(5), coord.x - 4 + random_number(7));

            if coord_in_bounds(at) && dg().floor[at.y as usize][at.x as usize].feature_id <= MAX_CAVE_FLOOR && dg().floor[at.y as usize][at.x as usize].treasure_id == 0 {
                if random_number(100) < 75 {
                    dungeon_place_random_object_at(at, false);
                } else {
                    dungeon_place_gold(at);
                }
                i = 9;
            }
            i += 1;
        }

        tries -= 1;
        if tries == 0 {
            break;
        }
    }
}

// Moves creature record from one space to another -RAK-
// this always works correctly, even if y1==y2 and x1==x2
pub fn dungeon_move_creature_record(from: Coord, to: Coord) {
    let id = dg().floor[from.y as usize][from.x as usize].creature_id;
    dg().floor[from.y as usize][from.x as usize].creature_id = 0;
    dg().floor[to.y as usize][to.x as usize].creature_id = id;
}

// Room is lit, make it appear -RAK-
pub fn dungeon_light_room(coord: Coord) {
    let height_middle = SCREEN_HEIGHT / 2;
    let width_middle = SCREEN_WIDTH / 2;

    let top = (coord.y / height_middle) * height_middle;
    let left = (coord.x / width_middle) * width_middle;
    let bottom = top + height_middle - 1;
    let right = left + width_middle - 1;

    for y in top..=bottom {
        for x in left..=right {
            let location = Coord::new(y, x);
            let tile = &mut dg().floor[y as usize][x as usize];

            if tile.perma_lit_room && !tile.permanent_light {
                tile.permanent_light = true;

                if tile.feature_id == TILE_DARK_FLOOR {
                    tile.feature_id = TILE_LIGHT_FLOOR;
                }
                if !tile.field_mark && tile.treasure_id != 0 {
                    let treasure_id = game().treasure.list[tile.treasure_id as usize].category_id;
                    if (TV_MIN_VISIBLE..=TV_MAX_VISIBLE).contains(&treasure_id) {
                        dg().floor[y as usize][x as usize].field_mark = true;
                    }
                }
                panel_put_tile(cave_get_tile_symbol(location), location);
            }
        }
    }
}

// Lights up given location -RAK-
pub fn dungeon_lite_spot(coord: Coord) {
    if !coord_inside_panel(coord) {
        return;
    }

    let symbol = cave_get_tile_symbol(coord);
    panel_put_tile(symbol, coord);
}

// Normal movement
// When FIND_FLAG,  light only permanent features
fn sub1_move_light(from: Coord, to: Coord) {
    if py().temporary_light_only {
        // Turn off lamp light
        for y in (from.y - 1)..=(from.y + 1) {
            for x in (from.x - 1)..=(from.x + 1) {
                dg().floor[y as usize][x as usize].temporary_light = false;
            }
        }
        if py().running_tracker != 0 && !config::options::options().run_print_self {
            py().temporary_light_only = false;
        }
    } else if py().running_tracker == 0 || config::options::options().run_print_self {
        py().temporary_light_only = true;
    }

    for y in (to.y - 1)..=(to.y + 1) {
        for x in (to.x - 1)..=(to.x + 1) {
            let tile = &mut dg().floor[y as usize][x as usize];

            // only light up if normal movement
            if py().temporary_light_only {
                tile.temporary_light = true;
            }

            if tile.feature_id >= MIN_CAVE_WALL {
                tile.permanent_light = true;
            } else if !tile.field_mark && tile.treasure_id != 0 {
                let tval = game().treasure.list[tile.treasure_id as usize].category_id;

                if (TV_MIN_VISIBLE..=TV_MAX_VISIBLE).contains(&tval) {
                    dg().floor[y as usize][x as usize].field_mark = true;
                }
            }
        }
    }

    // From uppermost to bottom most lines player was on.
    let (top, bottom) = if from.y < to.y {
        (from.y - 1, to.y + 1)
    } else {
        (to.y - 1, from.y + 1)
    };
    let (left, right) = if from.x < to.x {
        (from.x - 1, to.x + 1)
    } else {
        (to.x - 1, from.x + 1)
    };

    for y in top..=bottom {
        // Leftmost to rightmost do
        for x in left..=right {
            let coord = Coord::new(y, x);
            panel_put_tile(cave_get_tile_symbol(coord), coord);
        }
    }
}

// When blinded,  move only the player symbol.
// With no light,  movement becomes involved.
fn sub3_move_light(from: Coord, to: Coord) {
    if py().temporary_light_only {
        for y in (from.y - 1)..=(from.y + 1) {
            for x in (from.x - 1)..=(from.x + 1) {
                let coord = Coord::new(y, x);
                dg().floor[y as usize][x as usize].temporary_light = false;
                panel_put_tile(cave_get_tile_symbol(coord), coord);
            }
        }

        py().temporary_light_only = false;
    } else if py().running_tracker == 0 || config::options::options().run_print_self {
        panel_put_tile(cave_get_tile_symbol(from), from);
    }

    if py().running_tracker == 0 || config::options::options().run_print_self {
        panel_put_tile('@', to);
    }
}

// Package for moving the character's light about the screen
// Four cases : Normal, Finding, Blind, and No light -RAK-
pub fn dungeon_move_character_light(from: Coord, to: Coord) {
    if py().flags.blind > 0 || !py().carrying_light {
        sub3_move_light(from, to);
    } else {
        sub1_move_light(from, to);
    }
}

// Deletes a monster entry from the level -RAK-
//
// If used within update_monsters(), deleting a monster while scanning the
// monsters causes two problems;
//   1. monsters might get two turns
//   2. m_ptr/monptr might be invalid after running this function
// Hence a two step process is provided for update_monsters().
pub fn dungeon_delete_monster(id: i32) {
    dungeon_remove_monster_from_level(id);
    dungeon_delete_monster_record(id);
}

// dungeon_remove_monster_from_level ensures the monster has no HP before removing
// its ID from the dungeon level.
// This is called in breath(), and a couple of places in creatures.c.
pub fn dungeon_remove_monster_from_level(id: i32) {
    let monster = &mut monsters()[id as usize];

    // Force the HP negative to ensure that the monster is dead. For example, if the
    // monster was just eaten by another, it will still have positive hit points.
    monster.hp = -1;

    let pos = monster.pos;
    let lit = monster.lit;

    dg().floor[pos.y as usize][pos.x as usize].creature_id = 0;

    if lit {
        dungeon_lite_spot(pos);
    }

    if *monster_multiply_total() > 0 {
        *monster_multiply_total() -= 1;
    }
}

// dungeon_delete_monster_record delete the monster record from the monsters list.
// Called by update_monsters() and dungeon_delete_monster() only.
pub fn dungeon_delete_monster_record(id: i32) {
    let last_id = (*next_free_monster_id() - 1) as usize;
    let monster = monsters()[last_id];

    if id as usize != last_id {
        dg().floor[monster.pos.y as usize][monster.pos.x as usize].creature_id = id as u8;
        monsters()[id as usize] = monsters()[last_id];
    }

    monsters()[last_id] = BLANK_MONSTER;
    *next_free_monster_id() -= 1;
}

// Creates objects nearby the coordinates given -RAK-
pub fn dungeon_summon_object(coord: Coord, amount: i32, object_type: i32) -> i32 {
    let mut real_type = if object_type == 1 || object_type == 5 {
        1 // object_type == 1 -> objects
    } else {
        256 // object_type == 2 -> gold
    };

    let mut result = 0;
    let mut amount = amount;

    loop {
        let mut tries = 0;
        while tries <= 20 {
            let at = Coord::new(coord.y - 3 + random_number(5), coord.x - 3 + random_number(5));

            if coord_in_bounds(at) && crate::dungeon_los::los(coord, at) {
                let tile = &dg().floor[at.y as usize][at.x as usize];
                if tile.feature_id <= MAX_OPEN_SPACE && tile.treasure_id == 0 {
                    // object_type == 3 -> 50% objects, 50% gold
                    if object_type == 3 || object_type == 7 {
                        if random_number(100) < 50 {
                            real_type = 1;
                        } else {
                            real_type = 256;
                        }
                    }

                    if real_type == 1 {
                        dungeon_place_random_object_at(at, object_type >= 4);
                    } else {
                        dungeon_place_gold(at);
                    }

                    dungeon_lite_spot(at);

                    if cave_tile_visible(at) {
                        result += real_type;
                    }

                    tries = 20;
                }
            }
            tries += 1;
        }

        amount -= 1;
        if amount == 0 {
            break;
        }
    }

    result
}

// Deletes object from given location -RAK-
pub fn dungeon_delete_object(coord: Coord) -> bool {
    let tile = &mut dg().floor[coord.y as usize][coord.x as usize];

    if tile.feature_id == TILE_BLOCKED_FLOOR {
        tile.feature_id = TILE_CORR_FLOOR;
    }

    let treasure_id = tile.treasure_id;

    tile.treasure_id = 0;
    tile.field_mark = false;

    pusht(treasure_id);

    dungeon_lite_spot(coord);

    cave_tile_visible(coord)
}
