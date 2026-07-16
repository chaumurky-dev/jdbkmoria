// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// The running algorithm: -CJS-
//
// See the original umoria `player_run.cpp` for the full description of the
// running/find-mode algorithm; the behaviour is preserved as-is.

use crate::config;
use crate::dungeon::{cave_get_tile_symbol, dg, dungeon_move_character_light};
use crate::dungeon_tile::MAX_OPEN_SPACE;
use crate::game::game;
use crate::globals::RacyCell;
use crate::monster::monsters;
use crate::player::{py, player_move_position};
use crate::treasure::{TV_INVIS_TRAP, TV_OPEN_DOOR, TV_SECRET_DOOR};
use crate::types::Coord;
use crate::ui_io::{panel_put_tile, print_message};

// The cycle lists the directions in anticlockwise order, for over two complete
// cycles. The chome array maps a direction on to its position in the cycle. -CJS-
static CYCLE: [i32; 17] = [1, 2, 3, 6, 9, 8, 7, 4, 1, 2, 3, 6, 9, 8, 7, 4, 1];
static CHOME: [i32; 10] = [-1, 8, 9, 10, 7, -1, 11, 6, 5, 4];

static FIND_OPENAREA: RacyCell<bool> = RacyCell::new(false);
static FIND_BREAKRIGHT: RacyCell<bool> = RacyCell::new(false);
static FIND_BREAKLEFT: RacyCell<bool> = RacyCell::new(false);
static FIND_PREVDIR: RacyCell<i32> = RacyCell::new(0);
static FIND_DIRECTION: RacyCell<i32> = RacyCell::new(0); // Keep a record of which way we are going.

fn cycle(index: i32) -> i32 {
    CYCLE[index as usize]
}

// Do we see a wall? Used in running. -CJS-
fn player_can_see_dungeon_wall(dir: i32, coord: Coord) -> bool {
    let mut coord = coord;

    // check to see if movement there possible
    if !player_move_position(dir, &mut coord) {
        return true;
    }

    let c = cave_get_tile_symbol(coord);

    c == '#' || c == '%'
}

// Do we see anything? Used in running. -CJS-
fn player_see_nothing(dir: i32, coord: Coord) -> bool {
    let mut coord = coord;

    // check to see if movement there possible
    player_move_position(dir, &mut coord) && cave_get_tile_symbol(coord) == ' '
}

fn find_running_break(dir: i32, coord: Coord) {
    let mut deep_left = false;
    let mut deep_right = false;
    let mut short_left = false;
    let mut short_right = false;

    let cycle_index = CHOME[dir as usize];

    if player_can_see_dungeon_wall(cycle(cycle_index + 1), py().pos) {
        *FIND_BREAKLEFT.get() = true;
        short_left = true;
    } else if player_can_see_dungeon_wall(cycle(cycle_index + 1), coord) {
        *FIND_BREAKLEFT.get() = true;
        deep_left = true;
    }

    if player_can_see_dungeon_wall(cycle(cycle_index - 1), py().pos) {
        *FIND_BREAKRIGHT.get() = true;
        short_right = true;
    } else if player_can_see_dungeon_wall(cycle(cycle_index - 1), coord) {
        *FIND_BREAKRIGHT.get() = true;
        deep_right = true;
    }

    if *FIND_BREAKLEFT.get() && *FIND_BREAKRIGHT.get() {
        *FIND_OPENAREA.get() = false;

        // a hack to allow angled corridor entry
        if (dir & 1) != 0 {
            if deep_left && !deep_right {
                *FIND_PREVDIR.get() = cycle(cycle_index - 1);
            } else if deep_right && !deep_left {
                *FIND_PREVDIR.get() = cycle(cycle_index + 1);
            }
        } else if player_can_see_dungeon_wall(cycle(cycle_index), coord) {
            // else if there is a wall two spaces ahead and seem to be in a
            // corridor, then force a turn into the side corridor, must
            // be moving straight into a corridor here

            if short_left && !short_right {
                *FIND_PREVDIR.get() = cycle(cycle_index - 2);
            } else if short_right && !short_left {
                *FIND_PREVDIR.get() = cycle(cycle_index + 2);
            }
        }
    } else {
        *FIND_OPENAREA.get() = true;
    }
}

pub fn player_find_initialize(direction: i32) {
    let mut coord = py().pos;

    if !player_move_position(direction, &mut coord) {
        py().running_tracker = 0;
    } else {
        py().running_tracker = 1;

        *FIND_DIRECTION.get() = direction;
        *FIND_PREVDIR.get() = direction;

        *FIND_BREAKRIGHT.get() = false;
        *FIND_BREAKLEFT.get() = false;

        if py().flags.blind < 1 {
            find_running_break(direction, coord);
        }
    }

    // We must erase the player symbol '@' here, because sub3_move_light()
    // does not erase the previous location of the player when in find mode
    // and when `run_print_self` is false.  The player symbol is not draw at all
    // in this case while moving, so the only problem is on the first turn
    // of find mode, when the initial position of the character must be erased.
    // Hence we must do the erasure here.
    if !py().temporary_light_only && !config::options::options().run_print_self {
        panel_put_tile(cave_get_tile_symbol(py().pos), py().pos);
    }

    crate::player_move::player_move(direction, true);

    if py().running_tracker == 0 {
        game().command_count = 0;
    }
}

pub fn player_run_and_find() {
    let tracker = py().running_tracker;

    py().running_tracker += 1;

    // prevent infinite loops in find mode, will stop after moving 100 times
    if tracker > 100 {
        print_message(Some(crate::tr!("You stop running to catch your breath.")));
        player_end_running();
        return;
    }

    crate::player_move::player_move(*FIND_DIRECTION.get(), true);
}

// Switch off the run flag - and get the light correct. -CJS-
pub fn player_end_running() {
    if py().running_tracker == 0 {
        return;
    }

    py().running_tracker = 0;

    dungeon_move_character_light(py().pos, py().pos);
}

fn area_affect_stop_looking_at_squares(i: i32, dir: i32, new_dir: i32, coord: Coord, check_dir: &mut i32, dir_a: &mut i32, dir_b: &mut i32) -> bool {
    let tile = *dg().tile(coord);

    // Default: Square unseen. Treat as open.
    let mut invisible = true;

    if py().carrying_light || tile.temporary_light || tile.permanent_light || tile.field_mark {
        if tile.treasure_id != 0 {
            let tile_id = game().treasure.list[tile.treasure_id as usize].category_id;

            if tile_id != TV_INVIS_TRAP && tile_id != TV_SECRET_DOOR && (tile_id != TV_OPEN_DOOR || !config::options::options().run_ignore_doors) {
                player_end_running();
                return true;
            }
        }

        // jdbkmoria extension: a painting hanging on this wall causes a stop,
        // just like a visible door -- it's a landmark, not part of the corridor.
        if crate::paintings::painting_index_at(coord).is_some() {
            player_end_running();
            return true;
        }

        // Also Creatures
        // The monster should be visible since monster_update_visibility() checks
        // for the special case of being in find mode
        if tile.creature_id > 1 && monsters()[tile.creature_id as usize].lit {
            player_end_running();
            return true;
        }

        invisible = false;
    }

    if tile.feature_id <= MAX_OPEN_SPACE || invisible {
        if *FIND_OPENAREA.get() {
            // Have we found a break?
            if i < 0 {
                if *FIND_BREAKRIGHT.get() {
                    player_end_running();
                    return true;
                }
            } else if i > 0 {
                if *FIND_BREAKLEFT.get() {
                    player_end_running();
                    return true;
                }
            }
        } else if *dir_a == 0 {
            // The first new direction.
            *dir_a = new_dir;
        } else if *dir_b != 0 {
            // Three new directions. STOP.
            player_end_running();
            return true;
        } else if *dir_a != cycle(CHOME[dir as usize] + i - 1) {
            // If not adjacent to prev, STOP
            player_end_running();
            return true;
        } else {
            // Two adjacent choices. Make dir_b the diagonal, and
            // remember the other diagonal adjacent to the first option.
            if (new_dir & 1) == 1 {
                *check_dir = cycle(CHOME[dir as usize] + i - 2);
                *dir_b = new_dir;
            } else {
                *check_dir = cycle(CHOME[dir as usize] + i + 1);
                *dir_b = *dir_a;
                *dir_a = new_dir;
            }
        }
    } else if *FIND_OPENAREA.get() {
        // We see an obstacle. In open area, STOP if on a side previously open.
        if i < 0 {
            if *FIND_BREAKLEFT.get() {
                player_end_running();
                return true;
            }
            *FIND_BREAKRIGHT.get() = true;
        } else if i > 0 {
            if *FIND_BREAKRIGHT.get() {
                player_end_running();
                return true;
            }
            *FIND_BREAKLEFT.get() = true;
        }
    }

    false
}

// Determine the next direction for a run, or if we should stop. -CJS-
pub fn player_area_affect(_direction: i32, coord: Coord) {
    if py().flags.blind >= 1 {
        return;
    }

    let mut check_dir = 0;
    let mut dir_a = 0;
    let mut dir_b = 0;

    let direction = *FIND_PREVDIR.get();

    let max = (direction & 1) + 1;

    // Look at every newly adjacent square.
    for i in -max..=max {
        let new_dir = cycle(CHOME[direction as usize] + i);

        let mut spot = coord;

        // Objects player can see (Including doors?) cause a stop.
        if player_move_position(new_dir, &mut spot) {
            area_affect_stop_looking_at_squares(i, direction, new_dir, spot, &mut check_dir, &mut dir_a, &mut dir_b);
        }
    }

    if *FIND_OPENAREA.get() {
        return;
    }

    // choose a direction.

    if dir_b == 0 || (config::options::options().run_examine_corners && !config::options::options().run_cut_corners) {
        // There is only one option, or if two, then we always examine
        // potential corners and never cur known corners, so you step
        // into the straight option.
        if dir_a != 0 {
            *FIND_DIRECTION.get() = dir_a;
        }

        if dir_b == 0 {
            *FIND_PREVDIR.get() = dir_a;
        } else {
            *FIND_PREVDIR.get() = dir_b;
        }

        return;
    }

    // Two options!

    let mut location = coord;
    player_move_position(dir_a, &mut location);

    if !player_can_see_dungeon_wall(dir_a, location) || !player_can_see_dungeon_wall(check_dir, location) {
        // Don't see that it is closed off.  This could be a
        // potential corner or an intersection.
        if config::options::options().run_examine_corners && player_see_nothing(dir_a, location) && player_see_nothing(dir_b, location) {
            // Can not see anything ahead and in the direction we are
            // turning, assume that it is a potential corner.
            *FIND_DIRECTION.get() = dir_a;
            *FIND_PREVDIR.get() = dir_b;
        } else {
            // STOP: we are next to an intersection or a room
            player_end_running();
        }
    } else if config::options::options().run_cut_corners {
        // This corner is seen to be enclosed; we cut the corner.
        *FIND_DIRECTION.get() = dir_b;
        *FIND_PREVDIR.get() = dir_b;
    } else {
        // This corner is seen to be enclosed, and we deliberately
        // go the long way.
        *FIND_DIRECTION.get() = dir_a;
        *FIND_PREVDIR.get() = dir_b;
    }
}
