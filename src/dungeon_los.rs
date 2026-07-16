// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// LOS (line-of-sight) and Looking functions

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::dungeon::dg;
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CAVE_WALL, MIN_CLOSED_SPACE, TILE_BOUNDARY_WALL, TILE_GRANITE_WALL, TILE_MAGMA_WALL, TILE_QUARTZ_WALL};
use crate::game::{game, get_all_directions};
use crate::globals::RacyCell;
use crate::helpers::is_vowel;
use crate::identification::item_description;
use crate::monster::monsters;
use crate::player::py;
use crate::treasure::{TV_INVIS_TRAP, TV_SECRET_DOOR};
use crate::types::Coord;
use crate::ui::{coord_inside_panel, ESCAPE};
use crate::ui_io::{
    get_key_input, panel_move_cursor, print_message, put_string_clear_to_eol,
    terminal_restore_screen, terminal_save_screen,
};

// A simple, fast, integer-based line-of-sight algorithm.  By Joseph Hall,
// 4116 Brewster Drive, Raleigh NC 27606.  Email to jnh@ecemwl.ncsu.edu.
//
// Returns true if a line of sight can be traced from x0, y0 to x1, y1.
//
// The LOS begins at the center of the tile [x0, y0] and ends at the center of
// the tile [x1, y1].  If los() is to return true, all of the tiles this line
// passes through must be transparent, WITH THE EXCEPTIONS of the starting and
// ending tiles.
//
// We don't consider the line to be "passing through" a tile if it only passes
// across one corner of that tile.
pub fn los(from: Coord, to: Coord) -> bool {
    let mut from = from;
    let mut to = to;

    let delta_x = to.x - from.x;
    let delta_y = to.y - from.y;

    // Adjacent?
    if delta_x > -2 && delta_x < 2 && delta_y > -2 && delta_y < 2 {
        return true;
    }

    // Handle the cases where delta_x or delta_y == 0.
    if delta_x == 0 {
        if delta_y < 0 {
            std::mem::swap(&mut from.y, &mut to.y);
        }

        for yy in (from.y + 1)..to.y {
            if dg().tile(Coord::new(yy, from.x)).feature_id >= MIN_CLOSED_SPACE {
                return false;
            }
        }

        return true;
    }

    if delta_y == 0 {
        if delta_x < 0 {
            std::mem::swap(&mut from.x, &mut to.x);
        }

        for xx in (from.x + 1)..to.x {
            if dg().tile(Coord::new(from.y, xx)).feature_id >= MIN_CLOSED_SPACE {
                return false;
            }
        }

        return true;
    }

    // Now, we've eliminated all the degenerate cases.
    // In the computations below, dy (or dx) and m are multiplied by a scale factor,
    // scale = abs(delta_x * delta_y * 2), so that we can use integer arithmetic.

    let delta_multiply = delta_x * delta_y;
    let scale_half = delta_multiply.abs();
    let scale = scale_half << 1;
    let x_sign = if delta_x < 0 { -1 } else { 1 };
    let y_sign = if delta_y < 0 { -1 } else { 1 };

    // Travel from one end of the line to the other, oriented along the longer axis.

    let abs_delta_x = delta_x.abs();
    let abs_delta_y = delta_y.abs();

    if abs_delta_x >= abs_delta_y {
        // We start at the border between the first and second tiles, where
        // the y offset = .5 * slope.  Remember the scale factor.
        //
        // We have:     slope = delta_y / delta_x * 2 * (delta_y * delta_x)
        //                    = 2 * delta_y * delta_y.

        let mut dy = delta_y * delta_y; // "fractional" y position
        let slope = dy << 1;
        let mut xx = from.x + x_sign;

        // Consider the special case where slope == 1.
        let mut yy;
        if dy == scale_half {
            yy = from.y + y_sign;
            dy -= scale;
        } else {
            yy = from.y;
        }

        while to.x - xx != 0 {
            if dg().tile(Coord::new(yy, xx)).feature_id >= MIN_CLOSED_SPACE {
                return false;
            }

            dy += slope;

            if dy < scale_half {
                xx += x_sign;
            } else if dy > scale_half {
                yy += y_sign;
                if dg().tile(Coord::new(yy, xx)).feature_id >= MIN_CLOSED_SPACE {
                    return false;
                }
                xx += x_sign;
                dy -= scale;
            } else {
                // This is the case, dy == scale_half, where the LOS
                // exactly meets the corner of a tile.
                xx += x_sign;
                yy += y_sign;
                dy -= scale;
            }
        }
        return true;
    }

    let mut dx = delta_x * delta_x; // "fractional" x position
    let slope = dx << 1;

    let mut yy = from.y + y_sign;

    let mut xx;
    if dx == scale_half {
        xx = from.x + x_sign;
        dx -= scale;
    } else {
        xx = from.x;
    }

    while to.y - yy != 0 {
        if dg().tile(Coord::new(yy, xx)).feature_id >= MIN_CLOSED_SPACE {
            return false;
        }

        dx += slope;

        if dx < scale_half {
            yy += y_sign;
        } else if dx > scale_half {
            xx += x_sign;
            if dg().tile(Coord::new(yy, xx)).feature_id >= MIN_CLOSED_SPACE {
                return false;
            }
            yy += y_sign;
            dx -= scale;
        } else {
            xx += x_sign;
            yy += y_sign;
            dx -= scale;
        }
    }
    true
}

// An enhanced look, with peripheral vision. Looking all 8 -CJS- directions will
// see everything which ought to be visible. Can specify direction 5, which looks
// in all directions.
//
// See the original umoria `dungeon_los.cpp` for the full description of the
// diamond-based visibility model used here.
//
// Globally accessed variables: los_num_places_seen counts the number of places
// where something is seen. los_rocks_and_objects indicates a look for rock or
// objects.
//
// The others map coords in the ray frame to dungeon coords.
//
// dungeon y = py.pos.y + los_fyx * (ray x) + los_fyy * (ray y)
// dungeon x = py.pos.x + los_fxx * (ray x) + los_fxy * (ray y)
static LOS_FXX: RacyCell<i32> = RacyCell::new(0);
static LOS_FXY: RacyCell<i32> = RacyCell::new(0);
static LOS_FYX: RacyCell<i32> = RacyCell::new(0);
static LOS_FYY: RacyCell<i32> = RacyCell::new(0);
static LOS_NUM_PLACES_SEEN: RacyCell<i32> = RacyCell::new(0);
static LOS_HACK_NO_QUERY: RacyCell<bool> = RacyCell::new(false);
static LOS_ROCKS_AND_OBJECTS: RacyCell<i32> = RacyCell::new(0);

// Intended to be indexed by dir/2, since is only
// relevant to horizontal or vertical directions.
static LOS_DIR_SET_FXY: [i32; 5] = [0, 1, 0, 0, -1];
static LOS_DIR_SET_FXX: [i32; 5] = [0, 0, -1, 1, 0];
static LOS_DIR_SET_FYY: [i32; 5] = [0, 0, 1, -1, 0];
static LOS_DIR_SET_FYX: [i32; 5] = [0, 1, 0, 0, -1];

// Map diagonal-dir/2 to a normal-dir/2.
static LOS_MAP_DIAGONALS1: [usize; 5] = [1, 3, 0, 2, 4];
static LOS_MAP_DIAGONALS2: [usize; 5] = [2, 1, 0, 4, 3];

const GRADF: i32 = 10000; // Any sufficiently big number will do

// Look at what we can see. This is a free move.
//
// Prompts for a direction, and then looks at every object in turn within a cone of
// vision in that direction. For each object, the cursor is moved over the object,
// a description is given, and we wait for the user to type something. Typing
// ESCAPE will abort the entire look.
//
// Looks first at real objects and monsters, and looks at rock types only after all
// other things have been seen.  Only looks at rock types if the highlight_seams
// option is set.
pub fn look() {
    if py().flags.blind > 0 {
        print_message(Some("You can't see a damn thing!"));
        return;
    }

    if py().flags.image > 0 {
        print_message(Some("You can't believe what you are seeing! It's like a dream!"));
        return;
    }

    let mut dir = 0;
    if !get_all_directions("Look which direction?", &mut dir) {
        return;
    }

    *LOS_NUM_PLACES_SEEN.get() = 0;
    *LOS_ROCKS_AND_OBJECTS.get() = 0;

    // Have to set this up for the look_see
    *LOS_HACK_NO_QUERY.get() = false;

    let mut dummy = false;
    if look_see(Coord::new(0, 0), &mut dummy) {
        return;
    }

    let mut abort;
    loop {
        abort = false;
        if dir == 5 {
            for i in 1..=4 {
                *LOS_FXX.get() = LOS_DIR_SET_FXX[i];
                *LOS_FYX.get() = LOS_DIR_SET_FYX[i];
                *LOS_FXY.get() = LOS_DIR_SET_FXY[i];
                *LOS_FYY.get() = LOS_DIR_SET_FYY[i];
                if look_ray(0, 2 * GRADF - 1, 1) {
                    abort = true;
                    break;
                }
                *LOS_FXY.get() = -*LOS_FXY.get();
                *LOS_FYY.get() = -*LOS_FYY.get();
                if look_ray(0, 2 * GRADF, 2) {
                    abort = true;
                    break;
                }
            }
        } else if (dir & 1) == 0 {
            // Straight directions

            let i = (dir >> 1) as usize;
            *LOS_FXX.get() = LOS_DIR_SET_FXX[i];
            *LOS_FYX.get() = LOS_DIR_SET_FYX[i];
            *LOS_FXY.get() = LOS_DIR_SET_FXY[i];
            *LOS_FYY.get() = LOS_DIR_SET_FYY[i];
            if look_ray(0, GRADF, 1) {
                abort = true;
            } else {
                *LOS_FXY.get() = -*LOS_FXY.get();
                *LOS_FYY.get() = -*LOS_FYY.get();
                abort = look_ray(0, GRADF, 2);
            }
        } else {
            let i = LOS_MAP_DIAGONALS1[(dir >> 1) as usize];
            *LOS_FXX.get() = LOS_DIR_SET_FXX[i];
            *LOS_FYX.get() = LOS_DIR_SET_FYX[i];
            *LOS_FXY.get() = -LOS_DIR_SET_FXY[i];
            *LOS_FYY.get() = -LOS_DIR_SET_FYY[i];
            if look_ray(1, 2 * GRADF, GRADF) {
                abort = true;
            } else {
                let i = LOS_MAP_DIAGONALS2[(dir >> 1) as usize];
                *LOS_FXX.get() = LOS_DIR_SET_FXX[i];
                *LOS_FYX.get() = LOS_DIR_SET_FYX[i];
                *LOS_FXY.get() = LOS_DIR_SET_FXY[i];
                *LOS_FYY.get() = LOS_DIR_SET_FYY[i];
                abort = look_ray(1, 2 * GRADF - 1, GRADF);
            }
        }

        *LOS_ROCKS_AND_OBJECTS.get() += 1;

        if abort || !config::options::options().highlight_seams || *LOS_ROCKS_AND_OBJECTS.get() >= 2 {
            break;
        }
    }

    if abort {
        print_message(Some("--Aborting look--"));
        return;
    }

    if *LOS_NUM_PLACES_SEEN.get() != 0 {
        if dir == 5 {
            print_message(Some("That's all you see."));
        } else {
            print_message(Some("That's all you see in that direction."));
        }
    } else if dir == 5 {
        print_message(Some("You see nothing of interest."));
    } else {
        print_message(Some("You see nothing of interest in that direction."));
    }
}

// Look at everything within a cone of vision between two ray lines emanating
// from  the player, and y or more places away from the direct line of view.
// This is recursive.
//
// Rays are specified by gradients, y over x, multiplied by 2*GRADF. This is ONLY
// called with gradients between 2*GRADF (45 degrees) and 1 (almost horizontal).
fn look_ray(y: i32, from: i32, to: i32) -> bool {
    let mut from = from;

    // from is the larger angle of the ray, since we scan towards the
    // center line. If from is smaller, then the ray does not exist.
    if from <= to || y > config::monsters::MON_MAX_SIGHT as i32 {
        return false;
    }

    // Find first visible location along this line. Minimum x such
    // that (2x-1)/x < from/GRADF <=> x > GRADF(2x-1)/from. This may
    // be called with y=0 whence x will be set to 0. Thus we need a
    // special fix.
    let mut x = GRADF * (2 * y - 1) / from + 1;
    if x <= 0 {
        x = 1;
    }

    // Find last visible location along this line.
    // Maximum x such that (2x+1)/x > to/GRADF <=> x < GRADF(2x+1)/to
    let mut max_x = (GRADF * (2 * y + 1) - 1) / to;
    if max_x > config::monsters::MON_MAX_SIGHT as i32 {
        max_x = config::monsters::MON_MAX_SIGHT as i32;
    }
    if max_x < x {
        return false;
    }

    // LOS_HACK_NO_QUERY is a HACK to prevent doubling up on direct lines of
    // sight. If 'to' is  greater than 1, we do not really look at
    // stuff along the direct line of sight, but we do have to see
    // what is opaque for the purposes of obscuring other objects.
    *LOS_HACK_NO_QUERY.get() = (y == 0 && to > 1) || (y == x && from < GRADF * 2);

    let mut transparent = false;

    if look_see(Coord::new(y, x), &mut transparent) {
        return true;
    }

    if y == x {
        *LOS_HACK_NO_QUERY.get() = false;
    }

    // The original C used a `goto init_transparent` to enter the second
    // inner loop directly when the first location was transparent.
    let mut enter_transparent = transparent;

    loop {
        if !enter_transparent {
            // Look down the window we've found.
            if look_ray(y + 1, from, (2 * y + 1) * GRADF / x) {
                return true;
            }

            // Find the start of next window.
            loop {
                if x == max_x {
                    return false;
                }

                // See if this seals off the scan. (If y is zero, then it will.)
                from = (2 * y - 1) * GRADF / x;

                if from <= to {
                    return false;
                }

                x += 1;

                if look_see(Coord::new(y, x), &mut transparent) {
                    return true;
                }

                if transparent {
                    break;
                }
            }
        }
        enter_transparent = false;

        // init_transparent:
        // Find the end of this window of visibility.
        loop {
            if x == max_x {
                // The window is trimmed by an earlier limit.
                return look_ray(y + 1, from, to);
            }

            x += 1;

            if look_see(Coord::new(y, x), &mut transparent) {
                return true;
            }

            if !transparent {
                break;
            }
        }
    }
}

fn look_see(coord: Coord, transparent: &mut bool) -> bool {
    let mut coord = coord;

    if coord.x < 0 || coord.y < 0 || coord.y > coord.x {
        let error_message = format!("Illegal call to look_see({}, {})", coord.y, coord.x);
        print_message(Some(&error_message));
    }

    let mut description: &str = if coord.x == 0 && coord.y == 0 {
        "You are on"
    } else {
        "You see"
    };

    let j = py().pos.x + *LOS_FXX.get() * coord.x + *LOS_FXY.get() * coord.y;
    coord.y = py().pos.y + *LOS_FYX.get() * coord.x + *LOS_FYY.get() * coord.y;
    coord.x = j;

    if !coord_inside_panel(coord) {
        *transparent = false;
        return false;
    }

    let tile = *dg().tile(coord);
    *transparent = tile.feature_id <= MAX_OPEN_SPACE;

    if *LOS_HACK_NO_QUERY.get() {
        return false; // Don't look at a direct line of sight. A hack.
    }

    let mut key = ESCAPE;
    let mut msg = String::new();

    if *LOS_ROCKS_AND_OBJECTS.get() == 0 && tile.creature_id > 1 && monsters()[tile.creature_id as usize].lit {
        let creature_id = monsters()[tile.creature_id as usize].creature_id as usize;
        let name = CREATURES_LIST[creature_id].name;
        msg = format!(
            "{} {} {}. [(r)ecall]",
            description,
            if is_vowel(name.chars().next().unwrap_or(' ')) { "an" } else { "a" },
            name
        );
        description = "It is on";
        put_string_clear_to_eol(&msg, Coord::new(0, 0));

        panel_move_cursor(coord);
        key = get_key_input();

        if key == 'r' || key == 'R' {
            terminal_save_screen();
            key = crate::recall::memory_recall(creature_id as i32);
            terminal_restore_screen();
        }
    }

    if tile.temporary_light || tile.permanent_light || tile.field_mark {
        // The original C jumped straight into the granite case of the wall
        // switch when the tile held a secret door.
        let mut goto_granite = false;

        if tile.treasure_id != 0 {
            if game().treasure.list[tile.treasure_id as usize].category_id == TV_SECRET_DOOR {
                goto_granite = true;
            } else if *LOS_ROCKS_AND_OBJECTS.get() == 0 && game().treasure.list[tile.treasure_id as usize].category_id != TV_INVIS_TRAP {
                let obj_string = item_description(&game().treasure.list[tile.treasure_id as usize], true);

                msg = format!("{} {} ---pause---", description, obj_string);
                description = "It is in";
                put_string_clear_to_eol(&msg, Coord::new(0, 0));

                panel_move_cursor(coord);
                key = get_key_input();
            }
        }

        // rmoria extension: a painting hanging on this wall tile. Described
        // on the first (objects) pass, like monsters and items.
        let mut painting_described = false;
        if *LOS_ROCKS_AND_OBJECTS.get() == 0 && tile.feature_id >= MIN_CAVE_WALL && crate::paintings::painting_index_at(coord).is_some() {
            let (painting_msg, painting_key) = crate::paintings::look_at_painting(coord, description);
            if !painting_msg.is_empty() {
                msg = painting_msg;
                key = painting_key;
                painting_described = true;
            }
        }

        if !painting_described && (goto_granite || ((*LOS_ROCKS_AND_OBJECTS.get() != 0 || !msg.is_empty()) && tile.feature_id >= MIN_CLOSED_SPACE)) {
            let wall_description: Option<&str> = if goto_granite || tile.feature_id == TILE_BOUNDARY_WALL || tile.feature_id == TILE_GRANITE_WALL {
                // Granite is only interesting if it contains something.
                if !msg.is_empty() {
                    Some("a granite wall")
                } else {
                    None
                }
            } else if tile.feature_id == TILE_MAGMA_WALL {
                Some("some dark rock")
            } else if tile.feature_id == TILE_QUARTZ_WALL {
                Some("a quartz vein")
            } else {
                None
            };

            if let Some(wall_description) = wall_description {
                msg = format!("{} {} ---pause---", description, wall_description);
                put_string_clear_to_eol(&msg, Coord::new(0, 0));
                panel_move_cursor(coord);
                key = get_key_input();
            }
        }
    }

    if !msg.is_empty() {
        *LOS_NUM_PLACES_SEEN.get() += 1;
        if key == ESCAPE {
            return true;
        }
    }

    false
}
