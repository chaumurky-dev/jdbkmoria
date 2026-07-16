// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player tunneling functions

use crate::dice::max_dice_roll;
use crate::dungeon::{
    cave_tile_visible, dg, dungeon_delete_object, dungeon_lite_spot, dungeon_place_random_object_at,
};
use crate::dungeon_tile::{MIN_CAVE_WALL, TILE_BOUNDARY_WALL, TILE_GRANITE_WALL, TILE_MAGMA_WALL, TILE_QUARTZ_WALL};
use crate::game::{game, random_number};
use crate::identification::object_blocked_by_monster;
use crate::inventory::{Inventory, PlayerEquipment};
use crate::player::{py, player_attack_position, player_move_position, player_search, player_tunnel_wall, A_STR};
use crate::treasure::{TV_NOTHING, TV_RUBBLE, TV_SECRET_DOOR};
use crate::types::Coord;
use crate::ui_io::{print_message, print_message_no_command_interrupt};

// Don't let the player tunnel somewhere illegal, this is necessary to
// prevent the player from getting a free attack by trying to tunnel
// somewhere where it has no effect.
fn player_can_tunnel(treasure_id: u8, tile_id: u8) -> bool {
    if tile_id < MIN_CAVE_WALL
        && (treasure_id == 0
            || (game().treasure.list[treasure_id as usize].category_id != TV_RUBBLE
                && game().treasure.list[treasure_id as usize].category_id != TV_SECRET_DOOR))
    {
        game().player_free_turn = true;

        if treasure_id == 0 {
            print_message(Some(crate::tr!("Tunnel through what?  Empty air?!?")));
        } else {
            print_message(Some(crate::tr!("You can't tunnel through that.")));
        }

        return false;
    }

    true
}

// Compute the digging ability of player; based on strength, and type of tool used
fn player_digging_ability(weapon: &Inventory) -> i32 {
    let mut digging_ability = py().stats.used[A_STR] as i32;

    if (weapon.flags & crate::config::treasure::flags::TR_TUNNEL) != 0 {
        digging_ability += 25 + weapon.misc_use as i32 * 50;
    } else {
        digging_ability += max_dice_roll(weapon.damage) + weapon.to_hit as i32 + weapon.to_damage as i32;

        // divide by two so that digging without shovel isn't too easy
        digging_ability >>= 1;
    }

    // If this weapon is too heavy for the player to wield properly,
    // then also make it harder to dig with it.
    if py().weapon_is_heavy {
        digging_ability += (py().stats.used[A_STR] as i32 * 15) - weapon.weight as i32;

        if digging_ability < 0 {
            digging_ability = 0;
        }
    }

    digging_ability
}

fn dungeon_dig_granite_wall(coord: Coord, digging_ability: i32) {
    let i = random_number(1200) + 80;

    if player_tunnel_wall(coord, digging_ability, i) {
        print_message(Some(crate::tr!("You have finished the tunnel.")));
    } else {
        print_message_no_command_interrupt(crate::tr!("You tunnel into the granite wall."));
    }
}

fn dungeon_dig_magma_wall(coord: Coord, digging_ability: i32) {
    let i = random_number(600) + 10;

    if player_tunnel_wall(coord, digging_ability, i) {
        print_message(Some(crate::tr!("You have finished the tunnel.")));
    } else {
        print_message_no_command_interrupt(crate::tr!("You tunnel into the magma intrusion."));
    }
}

fn dungeon_dig_quartz_wall(coord: Coord, digging_ability: i32) {
    let i = random_number(400) + 10;

    if player_tunnel_wall(coord, digging_ability, i) {
        print_message(Some(crate::tr!("You have finished the tunnel.")));
    } else {
        print_message_no_command_interrupt(crate::tr!("You tunnel into the quartz vein."));
    }
}

fn dungeon_dig_rubble(coord: Coord, digging_ability: i32) {
    if digging_ability > random_number(180) {
        dungeon_delete_object(coord);
        print_message(Some(crate::tr!("You have removed the rubble.")));

        if random_number(10) == 1 {
            dungeon_place_random_object_at(coord, false);

            if cave_tile_visible(coord) {
                print_message(Some(crate::tr!("You have found something!")));
            }
        }

        dungeon_lite_spot(coord);
    } else {
        print_message_no_command_interrupt(crate::tr!("You dig in the rubble."));
    }
}

// Dig regular walls; Granite, magma intrusion, quartz vein
// Don't forget the boundary walls, made of titanium (255)
// Return `true` if a wall was dug at
fn dungeon_dig_at_location(coord: Coord, wall_type: u8, digging_ability: i32) -> bool {
    match wall_type {
        TILE_GRANITE_WALL => dungeon_dig_granite_wall(coord, digging_ability),
        TILE_MAGMA_WALL => dungeon_dig_magma_wall(coord, digging_ability),
        TILE_QUARTZ_WALL => dungeon_dig_quartz_wall(coord, digging_ability),
        TILE_BOUNDARY_WALL => print_message(Some(crate::tr!("This seems to be permanent rock."))),
        _ => return false,
    }
    true
}

// Tunnels through rubble and walls -RAK-
// Must take into account: secret doors, special tools
pub fn player_tunnel(direction: i32) {
    let mut direction = direction;

    // Confused?                    75% random movement
    if py().flags.confused > 0 && random_number(4) > 1 {
        direction = random_number(9);
    }

    let mut coord = py().pos;
    player_move_position(direction, &mut coord);

    let tile = *dg().tile(coord);
    let item = py().inventory[PlayerEquipment::Wield as usize];

    if !player_can_tunnel(tile.treasure_id, tile.feature_id) {
        return;
    }

    if tile.creature_id > 1 {
        object_blocked_by_monster(tile.creature_id as usize);
        player_attack_position(coord);
        return;
    }

    if item.category_id != TV_NOTHING {
        let digging_ability = player_digging_ability(&item);

        if !dungeon_dig_at_location(coord, tile.feature_id, digging_ability) {
            // Is there an object in the way?  (Rubble and secret doors)
            if tile.treasure_id != 0 {
                if game().treasure.list[tile.treasure_id as usize].category_id == TV_RUBBLE {
                    dungeon_dig_rubble(coord, digging_ability);
                } else if game().treasure.list[tile.treasure_id as usize].category_id == TV_SECRET_DOOR {
                    // Found secret door!
                    print_message_no_command_interrupt(crate::tr!("You tunnel into the granite wall."));
                    let chance = py().misc.chance_in_search as i32;
                    player_search(py().pos, chance);
                } else {
                    std::process::abort();
                }
            } else {
                std::process::abort();
            }
        }

        return;
    }

    print_message(Some(crate::tr!("You dig with your hands, making no progress.")));
}
