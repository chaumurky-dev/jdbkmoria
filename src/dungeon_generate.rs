// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Initialize/create a dungeon or town level

use crate::config;
use crate::dungeon::*;
use crate::dungeon_tile::*;
use crate::game::{game, random_number, random_number_normal_distribution, seed_reset_to_old_seed, seed_set};
use crate::game_objects::popt;
use crate::globals::RacyCell;
use crate::inventory::inventory_item_copy_to;
use crate::monster::{monsters, next_free_monster_id, BLANK_MONSTER, MON_TOTAL_ALLOCATIONS};
use crate::player::py;
use crate::types::Coord;

static DOORS_TK: RacyCell<[Coord; 100]> = RacyCell::new([Coord::new(0, 0); 100]);
static DOOR_INDEX: RacyCell<usize> = RacyCell::new(0);

// Returns a Dark/Light floor tile based on dg.current_level, and random number
fn dungeon_floor_tile_for_level() -> u8 {
    if dg().current_level as i32 <= random_number(25) {
        TILE_LIGHT_FLOOR
    } else {
        TILE_DARK_FLOOR
    }
}

// Always picks a correct direction
fn pick_correct_direction(vertical: &mut i32, horizontal: &mut i32, start: Coord, end: Coord) {
    if start.y < end.y {
        *vertical = 1;
    } else if start.y == end.y {
        *vertical = 0;
    } else {
        *vertical = -1;
    }

    if start.x < end.x {
        *horizontal = 1;
    } else if start.x == end.x {
        *horizontal = 0;
    } else {
        *horizontal = -1;
    }

    if *vertical != 0 && *horizontal != 0 {
        if random_number(2) == 1 {
            *vertical = 0;
        } else {
            *horizontal = 0;
        }
    }
}

// Chance of wandering direction
fn chance_of_random_direction(vertical: &mut i32, horizontal: &mut i32) {
    let direction = random_number(4);

    if direction < 3 {
        *horizontal = 0;
        *vertical = -3 + (direction << 1); // direction=1 -> y=-1; direction=2 -> y=1
    } else {
        *vertical = 0;
        *horizontal = -7 + (direction << 1); // direction=3 -> x=-1; direction=4 -> x=1
    }
}

// Blanks out entire cave -RAK-
fn dungeon_blank_entire_cave() {
    for row in dg().floor.iter_mut() {
        for tile in row.iter_mut() {
            *tile = Tile::empty();
        }
    }
}

// Fills in empty spots with desired rock -RAK-
// Note: 9 is a temporary value.
fn dungeon_fill_empty_tiles_with(rock_type: u8) {
    // no need to check the border of the cave
    for y in (1..(dg().height as usize - 1)).rev() {
        for x in 1..(dg().width as usize - 1) {
            let feature_id = dg().floor[y][x].feature_id;
            if feature_id == TILE_NULL_WALL || feature_id == TMP1_WALL || feature_id == TMP2_WALL {
                dg().floor[y][x].feature_id = rock_type;
            }
        }
    }
}

// Places indestructible rock around edges of dungeon -RAK-
fn dungeon_place_boundary_walls() {
    // put permanent wall on leftmost row and rightmost row
    for i in 0..dg().height as usize {
        dg().tile_mut(Coord::new(i as i32, 0)).feature_id = TILE_BOUNDARY_WALL;
        dg().tile_mut(Coord::new(i as i32, dg().width as i32 - 1)).feature_id = TILE_BOUNDARY_WALL;
    }

    // put permanent wall on top row and bottom row
    for i in 0..dg().width as usize {
        dg().tile_mut(Coord::new(0, i as i32)).feature_id = TILE_BOUNDARY_WALL;
        dg().tile_mut(Coord::new(dg().height as i32 - 1, i as i32)).feature_id = TILE_BOUNDARY_WALL;
    }
}

// Places "streamers" of rock through dungeon -RAK-
fn dungeon_place_streamer_rock(rock_type: u8, chance_of_treasure: i32) {
    // Choose starting point and direction
    let mut coord = Coord::new(
        (dg().height as i32 / 2) + 11 - random_number(23),
        (dg().width as i32 / 2) + 16 - random_number(33),
    );

    // Get random direction. Numbers 1-4, 6-9
    let mut dir = random_number(8);
    if dir > 4 {
        dir += 1;
    }

    // Place streamer into dungeon
    let t1 = 2 * config::dungeon::DUN_STREAMER_WIDTH as i32 + 1; // Constants
    let t2 = config::dungeon::DUN_STREAMER_WIDTH as i32 + 1;

    loop {
        for _ in 0..config::dungeon::DUN_STREAMER_DENSITY {
            let spot = Coord::new(coord.y + random_number(t1) - t2, coord.x + random_number(t1) - t2);

            if coord_in_bounds(spot) {
                if dg().tile(spot).feature_id == TILE_GRANITE_WALL {
                    dg().tile_mut(spot).feature_id = rock_type;

                    if random_number(chance_of_treasure) == 1 {
                        dungeon_place_gold(spot);
                    }
                }
            }
        }

        if !crate::player::player_move_position(dir, &mut coord) {
            break;
        }
    }
}

fn dungeon_place_open_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_CORR_FLOOR;
}

fn dungeon_place_broken_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_CORR_FLOOR;
    game().treasure.list[cur_pos].misc_use = 1;
}

fn dungeon_place_closed_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_CLOSED_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_BLOCKED_FLOOR;
}

fn dungeon_place_locked_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_CLOSED_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_BLOCKED_FLOOR;
    game().treasure.list[cur_pos].misc_use = (random_number(10) + 10) as i16;
}

fn dungeon_place_stuck_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_CLOSED_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_BLOCKED_FLOOR;
    game().treasure.list[cur_pos].misc_use = (-random_number(10) - 10) as i16;
}

fn dungeon_place_secret_door(coord: Coord) {
    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_SECRET_DOOR as usize, &mut game().treasure.list[cur_pos]);
    dg().tile_mut(coord).feature_id = TILE_BLOCKED_FLOOR;
}

fn dungeon_place_door(coord: Coord) {
    let door_type = random_number(3);

    if door_type == 1 {
        if random_number(4) == 1 {
            dungeon_place_broken_door(coord);
        } else {
            dungeon_place_open_door(coord);
        }
    } else if door_type == 2 {
        let door_type = random_number(12);

        if door_type > 3 {
            dungeon_place_closed_door(coord);
        } else if door_type == 3 {
            dungeon_place_stuck_door(coord);
        } else {
            dungeon_place_locked_door(coord);
        }
    } else {
        dungeon_place_secret_door(coord);
    }
}

// Place an up staircase at given y, x -RAK-
fn dungeon_place_up_stairs(coord: Coord) {
    if dg().tile(coord).treasure_id != 0 {
        dungeon_delete_object(coord);
    }

    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_UP_STAIR as usize, &mut game().treasure.list[cur_pos]);
}

// Place a down staircase at given y, x -RAK-
fn dungeon_place_down_stairs(coord: Coord) {
    if dg().tile(coord).treasure_id != 0 {
        dungeon_delete_object(coord);
    }

    let cur_pos = popt() as usize;
    dg().tile_mut(coord).treasure_id = cur_pos as u16;
    inventory_item_copy_to(config::dungeon::objects::OBJ_DOWN_STAIR as usize, &mut game().treasure.list[cur_pos]);
}

// Places a staircase 1=up, 2=down -RAK-
fn dungeon_place_stairs(stair_type: i32, number: i32, walls: i32) {
    let mut walls = walls;

    for _ in 0..number {
        let mut placed = false;

        while !placed {
            let mut j = 0;

            loop {
                // Note:
                // don't let y1/x1 be zero,
                // don't let y2/x2 be equal to dg.height-1/dg.width-1,
                // these values are always BOUNDARY_ROCK.
                let mut coord1 = Coord::new(random_number(dg().height as i32 - 14), random_number(dg().width as i32 - 14));
                let coord2 = Coord::new(coord1.y + 12, coord1.x + 12);

                loop {
                    loop {
                        let tile = dg().tile(coord1);
                        if tile.feature_id <= MAX_OPEN_SPACE && tile.treasure_id == 0 && coord_walls_next_to(coord1) >= walls {
                            placed = true;
                            if stair_type == 1 {
                                dungeon_place_up_stairs(coord1);
                            } else {
                                dungeon_place_down_stairs(coord1);
                            }
                        }
                        coord1.x += 1;
                        if coord1.x == coord2.x || placed {
                            break;
                        }
                    }

                    coord1.x = coord2.x - 12;
                    coord1.y += 1;
                    if coord1.y == coord2.y || placed {
                        break;
                    }
                }

                j += 1;
                if placed || j > 30 {
                    break;
                }
            }

            walls -= 1;
        }
    }
}

// Place a trap with a given displacement of point -RAK-
fn dungeon_place_vault_trap(coord: Coord, displacement: Coord, number: i32) {
    for _ in 0..number {
        let mut placed = false;

        let mut count = 0;
        while !placed && count <= 5 {
            let spot = Coord::new(
                coord.y - displacement.y - 1 + random_number(2 * displacement.y + 1),
                coord.x - displacement.x - 1 + random_number(2 * displacement.x + 1),
            );

            let tile = dg().tile(spot);
            if tile.feature_id != TILE_NULL_WALL && tile.feature_id <= MAX_CAVE_FLOOR && tile.treasure_id == 0 {
                dungeon_set_trap(spot, random_number(config::dungeon::objects::MAX_TRAPS as i32) - 1);
                placed = true;
            }
            count += 1;
        }
    }
}

// Place a trap with a given displacement of point -RAK-
fn dungeon_place_vault_monster(coord: Coord, number: i32) {
    for _ in 0..number {
        let mut spot = coord;
        crate::monster_manager::monster_summon(&mut spot, true);
    }
}

// Builds a room at a row, column coordinate -RAK-
fn dungeon_build_room(coord: Coord) {
    let floor = dungeon_floor_tile_for_level();

    let height = coord.y - random_number(4);
    let depth = coord.y + random_number(3);
    let left = coord.x - random_number(11);
    let right = coord.x + random_number(11);

    // the x dim of rooms tends to be much larger than the y dim,
    // so don't bother rewriting the y loop.

    for y in height..=depth {
        for x in left..=right {
            let c = Coord::new(y, x);
            dg().tile_mut(c).feature_id = floor;
            dg().tile_mut(c).perma_lit_room = true;
        }
    }

    for y in (height - 1)..=(depth + 1) {
        let left_c = Coord::new(y, left - 1);
        dg().tile_mut(left_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(left_c).perma_lit_room = true;

        let right_c = Coord::new(y, right + 1);
        dg().tile_mut(right_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(right_c).perma_lit_room = true;
    }

    for x in left..=right {
        let top_c = Coord::new(height - 1, x);
        dg().tile_mut(top_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(top_c).perma_lit_room = true;

        let bottom_c = Coord::new(depth + 1, x);
        dg().tile_mut(bottom_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(bottom_c).perma_lit_room = true;
    }
}

// Builds a room at a row, column coordinate -RAK-
// Type 1 unusual rooms are several overlapping rectangular ones
fn dungeon_build_room_overlapping_rectangles(coord: Coord) {
    let floor = dungeon_floor_tile_for_level();

    let limit = 1 + random_number(2);

    for _ in 0..limit {
        let height = coord.y - random_number(4);
        let depth = coord.y + random_number(3);
        let left = coord.x - random_number(11);
        let right = coord.x + random_number(11);

        // the x dim of rooms tends to be much larger than the y dim,
        // so don't bother rewriting the y loop.

        for y in height..=depth {
            for x in left..=right {
                let c = Coord::new(y, x);
                dg().tile_mut(c).feature_id = floor;
                dg().tile_mut(c).perma_lit_room = true;
            }
        }
        for y in (height - 1)..=(depth + 1) {
            let left_c = Coord::new(y, left - 1);
            if dg().tile(left_c).feature_id != floor {
                dg().tile_mut(left_c).feature_id = TILE_GRANITE_WALL;
                dg().tile_mut(left_c).perma_lit_room = true;
            }

            let right_c = Coord::new(y, right + 1);
            if dg().tile(right_c).feature_id != floor {
                dg().tile_mut(right_c).feature_id = TILE_GRANITE_WALL;
                dg().tile_mut(right_c).perma_lit_room = true;
            }
        }

        for x in left..=right {
            let top_c = Coord::new(height - 1, x);
            if dg().tile(top_c).feature_id != floor {
                dg().tile_mut(top_c).feature_id = TILE_GRANITE_WALL;
                dg().tile_mut(top_c).perma_lit_room = true;
            }

            let bottom_c = Coord::new(depth + 1, x);
            if dg().tile(bottom_c).feature_id != floor {
                dg().tile_mut(bottom_c).feature_id = TILE_GRANITE_WALL;
                dg().tile_mut(bottom_c).perma_lit_room = true;
            }
        }
    }
}

fn dungeon_place_random_secret_door(coord: Coord, depth: i32, height: i32, left: i32, right: i32) {
    match random_number(4) {
        1 => dungeon_place_secret_door(Coord::new(height - 1, coord.x)),
        2 => dungeon_place_secret_door(Coord::new(depth + 1, coord.x)),
        3 => dungeon_place_secret_door(Coord::new(coord.y, left - 1)),
        _ => dungeon_place_secret_door(Coord::new(coord.y, right + 1)),
    }
}

fn dungeon_place_vault(coord: Coord) {
    for y in (coord.y - 1)..=(coord.y + 1) {
        dg().tile_mut(Coord::new(y, coord.x - 1)).feature_id = TMP1_WALL;
        dg().tile_mut(Coord::new(y, coord.x + 1)).feature_id = TMP1_WALL;
    }

    dg().tile_mut(Coord::new(coord.y - 1, coord.x)).feature_id = TMP1_WALL;
    dg().tile_mut(Coord::new(coord.y + 1, coord.x)).feature_id = TMP1_WALL;
}

fn dungeon_place_treasure_vault(coord: Coord, depth: i32, height: i32, left: i32, right: i32) {
    dungeon_place_random_secret_door(coord, depth, height, left, right);
    dungeon_place_vault(coord);

    // Place a locked door
    let offset = random_number(4);
    if offset < 3 {
        // 1 -> y-1; 2 -> y+1
        dungeon_place_locked_door(Coord::new(coord.y - 3 + (offset << 1), coord.x));
    } else {
        dungeon_place_locked_door(Coord::new(coord.y, coord.x - 7 + (offset << 1)));
    }
}

fn dungeon_place_inner_pillars(coord: Coord) {
    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            dg().tile_mut(Coord::new(y, x)).feature_id = TMP1_WALL;
        }
    }

    if random_number(2) != 1 {
        return;
    }

    let offset = random_number(2);

    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 5 - offset)..=(coord.x - 3 - offset) {
            dg().tile_mut(Coord::new(y, x)).feature_id = TMP1_WALL;
        }
    }

    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x + 3 + offset)..=(coord.x + 5 + offset) {
            dg().tile_mut(Coord::new(y, x)).feature_id = TMP1_WALL;
        }
    }
}

fn dungeon_place_maze_inside_room(depth: i32, height: i32, left: i32, right: i32) {
    for y in height..=depth {
        for x in left..=right {
            if (0x1 & (x + y)) != 0 {
                dg().tile_mut(Coord::new(y, x)).feature_id = TMP1_WALL;
            }
        }
    }
}

fn dungeon_place_four_small_rooms(coord: Coord, depth: i32, height: i32, left: i32, right: i32) {
    for y in height..=depth {
        dg().tile_mut(Coord::new(y, coord.x)).feature_id = TMP1_WALL;
    }

    for x in left..=right {
        dg().tile_mut(Coord::new(coord.y, x)).feature_id = TMP1_WALL;
    }

    // place random secret door
    if random_number(2) == 1 {
        let offset = random_number(10);
        dungeon_place_secret_door(Coord::new(height - 1, coord.x - offset));
        dungeon_place_secret_door(Coord::new(height - 1, coord.x + offset));
        dungeon_place_secret_door(Coord::new(depth + 1, coord.x - offset));
        dungeon_place_secret_door(Coord::new(depth + 1, coord.x + offset));
    } else {
        let offset = random_number(3);
        dungeon_place_secret_door(Coord::new(coord.y + offset, left - 1));
        dungeon_place_secret_door(Coord::new(coord.y - offset, left - 1));
        dungeon_place_secret_door(Coord::new(coord.y + offset, right + 1));
        dungeon_place_secret_door(Coord::new(coord.y - offset, right + 1));
    }
}

// Builds a type 2 unusual room at a row, column coordinate -RAK-
//
// Type 2 unusual rooms all have an inner room:
//   1 - Just an inner room with one door
//   2 - An inner room within an inner room
//   3 - An inner room with pillar(s)
//   4 - Inner room has a maze
//   5 - A set of four inner rooms
fn dungeon_build_room_with_inner_rooms(coord: Coord) {
    let floor = dungeon_floor_tile_for_level();

    let mut height = coord.y - 4;
    let mut depth = coord.y + 4;
    let mut left = coord.x - 11;
    let mut right = coord.x + 11;

    // the x dim of rooms tends to be much larger than the y dim,
    // so don't bother rewriting the y loop.

    for i in height..=depth {
        for j in left..=right {
            let c = Coord::new(i, j);
            dg().tile_mut(c).feature_id = floor;
            dg().tile_mut(c).perma_lit_room = true;
        }
    }

    for i in (height - 1)..=(depth + 1) {
        let left_c = Coord::new(i, left - 1);
        dg().tile_mut(left_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(left_c).perma_lit_room = true;

        let right_c = Coord::new(i, right + 1);
        dg().tile_mut(right_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(right_c).perma_lit_room = true;
    }

    for i in left..=right {
        let top_c = Coord::new(height - 1, i);
        dg().tile_mut(top_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(top_c).perma_lit_room = true;

        let bottom_c = Coord::new(depth + 1, i);
        dg().tile_mut(bottom_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(bottom_c).perma_lit_room = true;
    }

    // The inner room
    height += 2;
    depth -= 2;
    left += 2;
    right -= 2;

    for i in (height - 1)..=(depth + 1) {
        dg().tile_mut(Coord::new(i, left - 1)).feature_id = TMP1_WALL;
        dg().tile_mut(Coord::new(i, right + 1)).feature_id = TMP1_WALL;
    }

    for i in left..=right {
        dg().tile_mut(Coord::new(height - 1, i)).feature_id = TMP1_WALL;
        dg().tile_mut(Coord::new(depth + 1, i)).feature_id = TMP1_WALL;
    }

    // Inner room variations
    match random_number(5) {
        1 => {
            // Plain: just an inner room with one door
            dungeon_place_random_secret_door(coord, depth, height, left, right);
            dungeon_place_vault_monster(coord, 1);
        }
        2 => {
            // Treasure Vault: an inner room within an inner room
            dungeon_place_treasure_vault(coord, depth, height, left, right);

            // Guard the treasure well
            dungeon_place_vault_monster(coord, 2 + random_number(3));

            // If the monsters don't get 'em.
            dungeon_place_vault_trap(coord, Coord::new(4, 10), 2 + random_number(3));
        }
        3 => {
            // Pillars: an inner room with pillar(s)
            dungeon_place_random_secret_door(coord, depth, height, left, right);

            dungeon_place_inner_pillars(coord);

            if random_number(3) != 1 {
                return;
            }

            // Inner rooms
            for i in (coord.x - 5)..=(coord.x + 5) {
                dg().tile_mut(Coord::new(coord.y - 1, i)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 1, i)).feature_id = TMP1_WALL;
            }
            dg().tile_mut(Coord::new(coord.y, coord.x - 5)).feature_id = TMP1_WALL;
            dg().tile_mut(Coord::new(coord.y, coord.x + 5)).feature_id = TMP1_WALL;

            dungeon_place_secret_door(Coord::new(coord.y - 3 + (random_number(2) << 1), coord.x - 3));
            dungeon_place_secret_door(Coord::new(coord.y - 3 + (random_number(2) << 1), coord.x + 3));

            if random_number(3) == 1 {
                dungeon_place_random_object_at(Coord::new(coord.y, coord.x - 2), false);
            }

            if random_number(3) == 1 {
                dungeon_place_random_object_at(Coord::new(coord.y, coord.x + 2), false);
            }

            dungeon_place_vault_monster(Coord::new(coord.y, coord.x - 2), random_number(2));
            dungeon_place_vault_monster(Coord::new(coord.y, coord.x + 2), random_number(2));
        }
        4 => {
            // Maze: inner room has a maze
            dungeon_place_random_secret_door(coord, depth, height, left, right);

            dungeon_place_maze_inside_room(depth, height, left, right);

            // Monsters just love mazes.
            dungeon_place_vault_monster(Coord::new(coord.y, coord.x - 5), random_number(3));
            dungeon_place_vault_monster(Coord::new(coord.y, coord.x + 5), random_number(3));

            // Traps make them entertaining.
            dungeon_place_vault_trap(Coord::new(coord.y, coord.x - 3), Coord::new(2, 8), random_number(3));
            dungeon_place_vault_trap(Coord::new(coord.y, coord.x + 3), Coord::new(2, 8), random_number(3));

            // Mazes should have some treasure too..
            for _ in 0..3 {
                dungeon_place_random_object_near(coord, 1);
            }
        }
        5 => {
            // FourSmallRooms: a set of four inner rooms
            dungeon_place_four_small_rooms(coord, depth, height, left, right);

            // Treasure in each one.
            dungeon_place_random_object_near(coord, 2 + random_number(2));

            // Gotta have some monsters.
            dungeon_place_vault_monster(Coord::new(coord.y + 2, coord.x - 4), random_number(2));
            dungeon_place_vault_monster(Coord::new(coord.y + 2, coord.x + 4), random_number(2));
            dungeon_place_vault_monster(Coord::new(coord.y - 2, coord.x - 4), random_number(2));
            dungeon_place_vault_monster(Coord::new(coord.y - 2, coord.x + 4), random_number(2));
        }
        _ => {
            // All cases are handled, so this should never be reached!
        }
    }
}

fn dungeon_place_large_middle_pillar(coord: Coord) {
    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            dg().tile_mut(Coord::new(y, x)).feature_id = TMP1_WALL;
        }
    }
}

// Builds a room at a row, column coordinate -RAK-
// Type 3 unusual rooms are cross shaped
fn dungeon_build_room_cross_shaped(coord: Coord) {
    let floor = dungeon_floor_tile_for_level();

    let mut random_offset = 2 + random_number(2);

    let mut height = coord.y - random_offset;
    let mut depth = coord.y + random_offset;
    let mut left = coord.x - 1;
    let mut right = coord.x + 1;

    for i in height..=depth {
        for j in left..=right {
            let c = Coord::new(i, j);
            dg().tile_mut(c).feature_id = floor;
            dg().tile_mut(c).perma_lit_room = true;
        }
    }

    for i in (height - 1)..=(depth + 1) {
        let left_c = Coord::new(i, left - 1);
        dg().tile_mut(left_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(left_c).perma_lit_room = true;

        let right_c = Coord::new(i, right + 1);
        dg().tile_mut(right_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(right_c).perma_lit_room = true;
    }

    for i in left..=right {
        let top_c = Coord::new(height - 1, i);
        dg().tile_mut(top_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(top_c).perma_lit_room = true;

        let bottom_c = Coord::new(depth + 1, i);
        dg().tile_mut(bottom_c).feature_id = TILE_GRANITE_WALL;
        dg().tile_mut(bottom_c).perma_lit_room = true;
    }

    random_offset = 2 + random_number(9);

    height = coord.y - 1;
    depth = coord.y + 1;
    left = coord.x - random_offset;
    right = coord.x + random_offset;

    for i in height..=depth {
        for j in left..=right {
            let c = Coord::new(i, j);
            dg().tile_mut(c).feature_id = floor;
            dg().tile_mut(c).perma_lit_room = true;
        }
    }

    for i in (height - 1)..=(depth + 1) {
        let left_c = Coord::new(i, left - 1);
        if dg().tile(left_c).feature_id != floor {
            dg().tile_mut(left_c).feature_id = TILE_GRANITE_WALL;
            dg().tile_mut(left_c).perma_lit_room = true;
        }

        let right_c = Coord::new(i, right + 1);
        if dg().tile(right_c).feature_id != floor {
            dg().tile_mut(right_c).feature_id = TILE_GRANITE_WALL;
            dg().tile_mut(right_c).perma_lit_room = true;
        }
    }

    for i in left..=right {
        let top_c = Coord::new(height - 1, i);
        if dg().tile(top_c).feature_id != floor {
            dg().tile_mut(top_c).feature_id = TILE_GRANITE_WALL;
            dg().tile_mut(top_c).perma_lit_room = true;
        }

        let bottom_c = Coord::new(depth + 1, i);
        if dg().tile(bottom_c).feature_id != floor {
            dg().tile_mut(bottom_c).feature_id = TILE_GRANITE_WALL;
            dg().tile_mut(bottom_c).perma_lit_room = true;
        }
    }

    // Special features.
    match random_number(4) {
        1 => {
            // Large middle pillar
            dungeon_place_large_middle_pillar(coord);
        }
        2 => {
            // Inner treasure vault
            dungeon_place_vault(coord);

            // Place a secret door
            random_offset = random_number(4);
            if random_offset < 3 {
                dungeon_place_secret_door(Coord::new(coord.y - 3 + (random_offset << 1), coord.x));
            } else {
                dungeon_place_secret_door(Coord::new(coord.y, coord.x - 7 + (random_offset << 1)));
            }

            // Place a treasure in the vault
            dungeon_place_random_object_at(coord, false);

            // Let's guard the treasure well.
            dungeon_place_vault_monster(coord, 2 + random_number(2));

            // Traps naturally
            dungeon_place_vault_trap(coord, Coord::new(4, 4), 1 + random_number(3));
        }
        3 => {
            if random_number(3) == 1 {
                dg().tile_mut(Coord::new(coord.y - 1, coord.x - 2)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 1, coord.x - 2)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y - 1, coord.x + 2)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 1, coord.x + 2)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y - 2, coord.x - 1)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y - 2, coord.x + 1)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 2, coord.x - 1)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 2, coord.x + 1)).feature_id = TMP1_WALL;
                if random_number(3) == 1 {
                    dungeon_place_secret_door(Coord::new(coord.y, coord.x - 2));
                    dungeon_place_secret_door(Coord::new(coord.y, coord.x + 2));
                    dungeon_place_secret_door(Coord::new(coord.y - 2, coord.x));
                    dungeon_place_secret_door(Coord::new(coord.y + 2, coord.x));
                }
            } else if random_number(3) == 1 {
                dg().tile_mut(coord).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y - 1, coord.x)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y + 1, coord.x)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y, coord.x - 1)).feature_id = TMP1_WALL;
                dg().tile_mut(Coord::new(coord.y, coord.x + 1)).feature_id = TMP1_WALL;
            } else if random_number(3) == 1 {
                dg().tile_mut(coord).feature_id = TMP1_WALL;
            }
        }
        _ => {
            // no special feature
        }
    }
}

// Constructs a tunnel between two points
fn dungeon_build_tunnel(start: Coord, end: Coord) {
    let mut tunnels_tk = [Coord::new(0, 0); 1000];
    let mut walls_tk = [Coord::new(0, 0); 1000];

    let mut start = start;

    // Main procedure for Tunnel
    // Note: 9 is a temporary value
    let mut door_flag = false;
    let mut stop_flag = false;
    let mut main_loop_count = 0;
    let start_row = start.y;
    let start_col = start.x;
    let mut tunnel_index: usize = 0;
    let mut wall_index: usize = 0;

    let mut y_direction = 0;
    let mut x_direction = 0;
    pick_correct_direction(&mut y_direction, &mut x_direction, start, end);

    loop {
        // prevent infinite loops, just in case
        main_loop_count += 1;
        if main_loop_count > 2000 {
            stop_flag = true;
        }

        if random_number(100) > config::dungeon::DUN_DIR_CHANGE as i32 {
            if random_number(config::dungeon::DUN_RANDOM_DIR as i32) == 1 {
                chance_of_random_direction(&mut y_direction, &mut x_direction);
            } else {
                pick_correct_direction(&mut y_direction, &mut x_direction, start, end);
            }
        }

        let mut tmp_row = start.y + y_direction;
        let mut tmp_col = start.x + x_direction;

        while !coord_in_bounds(Coord::new(tmp_row, tmp_col)) {
            if random_number(config::dungeon::DUN_RANDOM_DIR as i32) == 1 {
                chance_of_random_direction(&mut y_direction, &mut x_direction);
            } else {
                pick_correct_direction(&mut y_direction, &mut x_direction, start, end);
            }
            tmp_row = start.y + y_direction;
            tmp_col = start.x + x_direction;
        }

        let feature_id = dg().tile(Coord::new(tmp_row, tmp_col)).feature_id;
        match feature_id {
            TILE_NULL_WALL => {
                start.y = tmp_row;
                start.x = tmp_col;
                if tunnel_index < 1000 {
                    tunnels_tk[tunnel_index] = start;
                    tunnel_index += 1;
                }
                door_flag = false;
            }
            TMP2_WALL => {
                // do nothing
            }
            TILE_GRANITE_WALL => {
                start.y = tmp_row;
                start.x = tmp_col;

                if wall_index < 1000 {
                    walls_tk[wall_index] = start;
                    wall_index += 1;
                }

                for y in (start.y - 1)..=(start.y + 1) {
                    for x in (start.x - 1)..=(start.x + 1) {
                        let c = Coord::new(y, x);
                        if coord_in_bounds(c) {
                            // values 11 and 12 are impossible here, dungeon_place_streamer_rock
                            // is never run before dungeon_build_tunnel
                            if dg().tile(c).feature_id == TILE_GRANITE_WALL {
                                dg().tile_mut(c).feature_id = TMP2_WALL;
                            }
                        }
                    }
                }
            }
            TILE_CORR_FLOOR | TILE_BLOCKED_FLOOR => {
                start.y = tmp_row;
                start.x = tmp_col;

                if !door_flag {
                    let door_index = DOOR_INDEX.get();
                    if *door_index < 100 {
                        DOORS_TK.get()[*door_index] = start;
                        *door_index += 1;
                    }
                    door_flag = true;
                }

                if random_number(100) > config::dungeon::DUN_TUNNELING as i32 {
                    // make sure that tunnel has gone a reasonable distance
                    // before stopping it, this helps prevent isolated rooms
                    let tmp_row = (start.y - start_row).abs();
                    let tmp_col = (start.x - start_col).abs();

                    if tmp_row > 10 || tmp_col > 10 {
                        stop_flag = true;
                    }
                }
            }
            _ => {
                // none of: NULL, TMP2, GRANITE, CORR
                start.y = tmp_row;
                start.x = tmp_col;
            }
        }

        if (start.y == end.y && start.x == end.x) || stop_flag {
            break;
        }
    }

    for tunnel in tunnels_tk.iter().take(tunnel_index) {
        dg().tile_mut(*tunnel).feature_id = TILE_CORR_FLOOR;
    }

    for wall in walls_tk.iter().take(wall_index) {
        if dg().tile(*wall).feature_id == TMP2_WALL {
            if random_number(100) < config::dungeon::DUN_ROOM_DOORS as i32 {
                dungeon_place_door(*wall);
            } else {
                // these have to be doorways to rooms
                dg().tile_mut(*wall).feature_id = TILE_CORR_FLOOR;
            }
        }
    }
}

fn dungeon_is_next_to(coord: Coord) -> bool {
    if coord_corridor_walls_next_to(coord) > 2 {
        let vertical = dg().tile(Coord::new(coord.y - 1, coord.x)).feature_id >= MIN_CAVE_WALL
            && dg().tile(Coord::new(coord.y + 1, coord.x)).feature_id >= MIN_CAVE_WALL;
        let horizontal = dg().tile(Coord::new(coord.y, coord.x - 1)).feature_id >= MIN_CAVE_WALL
            && dg().tile(Coord::new(coord.y, coord.x + 1)).feature_id >= MIN_CAVE_WALL;

        return vertical || horizontal;
    }

    false
}

// Places door at y, x position if at least 2 walls found
fn dungeon_place_door_if_next_to_two_walls(coord: Coord) {
    if dg().tile(coord).feature_id == TILE_CORR_FLOOR
        && random_number(100) > config::dungeon::DUN_TUNNEL_DOORS as i32
        && dungeon_is_next_to(coord)
    {
        dungeon_place_door(coord);
    }
}

// Returns random co-ordinates -RAK-
fn dungeon_new_spot(coord: &mut Coord) {
    let mut position;

    loop {
        position = Coord::new(random_number(dg().height as i32 - 2), random_number(dg().width as i32 - 2));
        let tile = dg().tile(position);
        if tile.feature_id < MIN_CLOSED_SPACE && tile.creature_id == 0 && tile.treasure_id == 0 {
            break;
        }
    }

    *coord = position;
}

// Functions to emulate the original Pascal sets
fn set_rooms(tile_id: i32) -> bool {
    tile_id == TILE_DARK_FLOOR as i32 || tile_id == TILE_LIGHT_FLOOR as i32
}

fn set_corridors(tile_id: i32) -> bool {
    tile_id == TILE_CORR_FLOOR as i32 || tile_id == TILE_BLOCKED_FLOOR as i32
}

fn set_floors(tile_id: i32) -> bool {
    tile_id <= MAX_CAVE_FLOOR as i32
}

// Cave logic flow for generation of new dungeon
fn dungeon_generate() {
    // Room initialization
    let row_rooms = (2 * (dg().height as i32 / SCREEN_HEIGHT)) as usize;
    let col_rooms = (2 * (dg().width as i32 / SCREEN_WIDTH)) as usize;

    let mut room_map = [[false; 20]; 20];

    // jdbkmoria extension: standard deviation scaled x4 (2 -> 8) alongside
    // DUN_ROOMS_MEAN's x4 scaling, to preserve the distribution's shape.
    let random_room_count = random_number_normal_distribution(config::dungeon::DUN_ROOMS_MEAN as i32, 8);
    for _ in 0..random_room_count {
        room_map[(random_number(row_rooms as i32) - 1) as usize][(random_number(col_rooms as i32) - 1) as usize] = true;
    }

    // Build rooms
    let mut location_id: usize = 0;
    let mut locations = [Coord::new(0, 0); 400];

    for (row, room_row) in room_map.iter().enumerate().take(row_rooms) {
        for (col, is_room) in room_row.iter().enumerate().take(col_rooms) {
            if *is_room {
                locations[location_id] = Coord::new(
                    (row as i32) * (SCREEN_HEIGHT >> 1) + QUART_HEIGHT,
                    (col as i32) * (SCREEN_WIDTH >> 1) + QUART_WIDTH,
                );
                if dg().current_level as i32 > random_number(config::dungeon::DUN_UNUSUAL_ROOMS as i32) {
                    let room_type = random_number(3);

                    if room_type == 1 {
                        dungeon_build_room_overlapping_rectangles(locations[location_id]);
                    } else if room_type == 2 {
                        dungeon_build_room_with_inner_rooms(locations[location_id]);
                    } else {
                        dungeon_build_room_cross_shaped(locations[location_id]);
                    }
                } else {
                    dungeon_build_room(locations[location_id]);
                }
                location_id += 1;
            }
        }
    }

    for _ in 0..location_id {
        let pick1 = (random_number(location_id as i32) - 1) as usize;
        let pick2 = (random_number(location_id as i32) - 1) as usize;

        locations.swap(pick1, pick2);
    }

    *DOOR_INDEX.get() = 0;

    // move zero entry to location_id, so that can call dungeon_build_tunnel all location_id times
    locations[location_id] = locations[0];

    for i in 0..location_id {
        dungeon_build_tunnel(locations[i + 1], locations[i]);
    }

    // Generate walls and streamers
    dungeon_fill_empty_tiles_with(TILE_GRANITE_WALL);
    for _ in 0..config::dungeon::DUN_MAGMA_STREAMER {
        dungeon_place_streamer_rock(TILE_MAGMA_WALL, config::dungeon::DUN_MAGMA_TREASURE as i32);
    }
    for _ in 0..config::dungeon::DUN_QUARTZ_STREAMER {
        dungeon_place_streamer_rock(TILE_QUARTZ_WALL, config::dungeon::DUN_QUARTZ_TREASURE as i32);
    }
    dungeon_place_boundary_walls();

    // Place intersection doors
    for i in 0..*DOOR_INDEX.get() {
        let door = DOORS_TK.get()[i];
        dungeon_place_door_if_next_to_two_walls(Coord::new(door.y, door.x - 1));
        dungeon_place_door_if_next_to_two_walls(Coord::new(door.y, door.x + 1));
        dungeon_place_door_if_next_to_two_walls(Coord::new(door.y - 1, door.x));
        dungeon_place_door_if_next_to_two_walls(Coord::new(door.y + 1, door.x));
    }

    let alloc_level = (dg().current_level as i32 / 3).clamp(2, 10);

    // jdbkmoria extension: stair counts scaled x4 to match the level's
    // quadrupled area.
    dungeon_place_stairs(2, (random_number(2) + 2) * 4, 3);
    dungeon_place_stairs(1, random_number(2) * 4, 3);

    // Set up the character coords, used by monster_place_new_within_distance, monster_place_winning
    let mut coord = Coord::new(0, 0);
    dungeon_new_spot(&mut coord);
    py().pos = coord;

    // jdbkmoria extension: monster count, corridor object/gold count, and
    // trap count all scaled x4 (see the comment above DUN_ROOMS_MEAN in
    // config.rs); the normal-distribution std-devs below are scaled x4 in
    // lockstep with their (already x4-scaled) means.
    crate::monster_manager::monster_place_new_within_distance(4 * (random_number(8) + config::monsters::MON_MIN_PER_LEVEL as i32 + alloc_level), 0, true);
    dungeon_allocate_and_place_object(set_corridors, 3, 4 * random_number(alloc_level));
    dungeon_allocate_and_place_object(set_rooms, 5, random_number_normal_distribution(config::dungeon::objects::LEVEL_OBJECTS_PER_ROOM as i32, 12));
    dungeon_allocate_and_place_object(set_floors, 5, random_number_normal_distribution(config::dungeon::objects::LEVEL_OBJECTS_PER_CORRIDOR as i32, 12));
    dungeon_allocate_and_place_object(set_floors, 4, random_number_normal_distribution(config::dungeon::objects::LEVEL_TOTAL_GOLD_AND_GEMS as i32, 12));
    dungeon_allocate_and_place_object(set_floors, 1, 4 * random_number(alloc_level));

    if dg().current_level >= config::monsters::MON_ENDGAME_LEVEL as i16 {
        crate::monster_manager::monster_place_winning();
    }

    // jdbkmoria extension: hang paintings on the walls. Must come after the
    // character's position is set (see place_paintings on level 1).
    crate::paintings::place_paintings();
}

// Builds a store at a row, column coordinate
fn dungeon_build_store(store_id: i32, coord: Coord) {
    let yval = coord.y * 10 + 5;
    let xval = coord.x * 16 + 16;
    let height = yval - random_number(3);
    let depth = yval + random_number(4);
    let left = xval - random_number(6);
    let right = xval + random_number(6);

    for y in height..=depth {
        for x in left..=right {
            dg().tile_mut(Coord::new(y, x)).feature_id = TILE_BOUNDARY_WALL;
        }
    }

    let tmp = random_number(4);
    let (y, x) = if tmp < 3 {
        let y = random_number(depth - height) + height - 1;
        let x = if tmp == 1 { left } else { right };
        (y, x)
    } else {
        let x = random_number(right - left) + left - 1;
        let y = if tmp == 3 { depth } else { height };
        (y, x)
    };

    let door_pos = Coord::new(y, x);
    dg().tile_mut(door_pos).feature_id = TILE_CORR_FLOOR;

    let cur_pos = popt() as usize;
    dg().tile_mut(door_pos).treasure_id = cur_pos as u16;

    inventory_item_copy_to(config::dungeon::objects::OBJ_STORE_DOOR as usize + store_id as usize, &mut game().treasure.list[cur_pos]);
}

// Link all free space in treasure list together
fn treasure_linker() {
    for i in 0..game().treasure.list.len() {
        inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut game().treasure.list[i]);
    }
    game().treasure.current_id = config::treasure::MIN_TREASURE_LIST_ID as i16;
}

// Link all free space in monster list together
fn monster_linker() {
    for i in 0..MON_TOTAL_ALLOCATIONS {
        monsters()[i] = BLANK_MONSTER;
    }
    *next_free_monster_id() = config::monsters::MON_MIN_INDEX_ID as i16;
}

fn dungeon_place_town_stores() {
    let mut rooms: [i32; 6] = [0, 1, 2, 3, 4, 5];

    let mut rooms_count = 6;

    for y in 0..2 {
        for x in 0..3 {
            let room_id = (random_number(rooms_count) - 1) as usize;
            dungeon_build_store(rooms[room_id], Coord::new(y, x));

            for i in room_id..(rooms_count as usize - 1) {
                rooms[i] = rooms[i + 1];
            }

            rooms_count -= 1;
        }
    }
}

fn is_nigh_time() -> bool {
    (0x1 & (dg().game_turn / 5000)) != 0
}

// Light town based on whether it is Night time, or day time.
fn light_town() {
    if is_nigh_time() {
        for y in 0..dg().height as usize {
            for x in 0..dg().width as usize {
                if dg().floor[y][x].feature_id != TILE_DARK_FLOOR {
                    dg().floor[y][x].permanent_light = true;
                }
            }
        }
        crate::monster_manager::monster_place_new_within_distance(config::monsters::MON_MIN_TOWNSFOLK_NIGHT as i32, 3, true);
    } else {
        // ...it is day time
        for y in 0..dg().height as usize {
            for x in 0..dg().width as usize {
                dg().floor[y][x].permanent_light = true;
            }
        }
        crate::monster_manager::monster_place_new_within_distance(config::monsters::MON_MIN_TOWNSFOLK_DAY as i32, 3, true);
    }
}

// I may have written the town level code, but I'm not exactly
// proud of it.   Adding the stores required some real slucky
// hooks which I have not had time to re-think. -RAK-

// Town logic flow for generation of new town
fn town_generation() {
    seed_set(game().town_seed);

    dungeon_place_town_stores();

    dungeon_fill_empty_tiles_with(TILE_DARK_FLOOR);

    // make stairs before seed_reset_to_old_seed, so that they don't move around
    dungeon_place_boundary_walls();
    dungeon_place_stairs(2, 1, 0);

    seed_reset_to_old_seed();

    // Set up the character coords, used by monster_place_new_within_distance below
    let mut coord = Coord::new(0, 0);
    dungeon_new_spot(&mut coord);
    py().pos = coord;

    light_town();

    crate::store_inventory::store_maintenance();
}

// Generates a random dungeon level -RAK-
pub fn generate_cave() {
    dg().panel.top = 0;
    dg().panel.bottom = 0;
    dg().panel.left = 0;
    dg().panel.right = 0;

    py().pos.y = -1;
    py().pos.x = -1;

    treasure_linker();
    monster_linker();
    dungeon_blank_entire_cave();

    // jdbkmoria extension: paintings are per-level; dungeon_generate() hangs
    // new ones, the town has none.
    crate::paintings::paintings().clear();

    // We're in the dungeon more than the town, so let's default to that -MRC-
    dg().height = MAX_HEIGHT as i16;
    dg().width = MAX_WIDTH as i16;

    if dg().current_level == 0 {
        dg().height = SCREEN_HEIGHT as i16;
        dg().width = SCREEN_WIDTH as i16;
    }

    dg().panel.max_rows = ((dg().height as i32 / SCREEN_HEIGHT) * 2 - 2) as i16;
    dg().panel.max_cols = ((dg().width as i32 / SCREEN_WIDTH) * 2 - 2) as i16;

    dg().panel.row = dg().panel.max_rows as i32;
    dg().panel.col = dg().panel.max_cols as i32;

    if dg().current_level == 0 {
        town_generation();
    } else {
        dungeon_generate();
    }
}
