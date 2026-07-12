// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Bashing open doors and chests

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::{dice_roll, max_dice_roll};
use crate::dungeon::{dg, dungeon_lite_spot};
use crate::dungeon_tile::{MIN_CAVE_WALL, TILE_CORR_FLOOR};
use crate::game::{game, get_direction_with_memory, get_random_direction, random_number};
use crate::inventory::{inventory_item_copy_to, PlayerEquipment};
use crate::monster::{monster_take_hit, monsters};
use crate::player::{
    py, player_move_position, player_test_being_hit, player_weapon_critical_blow, A_DEX, A_STR,
    BTH_PER_PLUS_TO_HIT_ADJUST, CLASS_BTH,
};
use crate::player_move::player_move;
use crate::treasure::{TV_CHEST, TV_CLOSED_DOOR};
use crate::types::Coord;
use crate::ui::display_character_experience;
use crate::ui_io::{print_message, print_message_no_command_interrupt};

// Bash open a door or chest -RAK-
// Note: Affected by strength and weight of character
//
// For a closed door, `misc_use` is positive if locked; negative if stuck. A disarm spell
// unlocks and unjams doors!
//
// For an open door, `misc_use` is positive for a broken door.
//
// A closed door can be opened - harder if locked. Any door might be bashed open
// (and thereby broken). Bashing a door is (potentially) faster! You move into the
// door way. To open a stuck door, it must be bashed. A closed door can be jammed
// (which makes it stuck if previously locked).
//
// Creatures can also open doors. A creature with open door ability will (if not
// in the line of sight) move though a closed or secret door with no changes. If
// in the line of sight, closed door are opened, & secret door revealed. Whether
// in the line of sight or not, such a creature may unlock or unstick a door.
//
// A creature with no such ability will attempt to bash a non-secret door.
pub fn player_bash() {
    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    if py().flags.confused > 0 {
        print_message(Some("You are confused."));
        dir = get_random_direction();
    }

    let mut coord = py().pos;
    player_move_position(dir, &mut coord);

    let tile = dg().floor[coord.y as usize][coord.x as usize];

    if tile.creature_id > 1 {
        player_bash_position(coord);
        return;
    }

    if tile.treasure_id != 0 {
        let category_id = game().treasure.list[tile.treasure_id as usize].category_id;

        if category_id == TV_CLOSED_DOOR {
            player_bash_closed_door(coord, dir);
        } else if category_id == TV_CHEST {
            player_bash_closed_chest(tile.treasure_id as usize);
        } else {
            // Can't give free turn, or else player could try
            // directions until they find the invisible creature
            print_message(Some("You bash it, but nothing interesting happens."));
        }
        return;
    }

    if tile.feature_id < MIN_CAVE_WALL {
        print_message(Some("You bash at empty space."));
        return;
    }

    // same message for wall as for secret door
    print_message(Some("You bash it, but nothing interesting happens."));
}

// Make a bash attack on someone. -CJS-
// Used to be part of bash above.
fn player_bash_attack(coord: Coord) {
    let monster_id = dg().floor[coord.y as usize][coord.x as usize].creature_id as usize;

    monsters()[monster_id].sleep_count = 0;

    let creature_id = monsters()[monster_id].creature_id as usize;
    let monster_lit = monsters()[monster_id].lit;

    // Does the player know what they're fighting?
    let name = if !monster_lit {
        "it".to_string()
    } else {
        format!("the {}", CREATURES_LIST[creature_id].name)
    };

    let mut base_to_hit = py().stats.used[A_STR] as i32;
    base_to_hit += py().inventory[PlayerEquipment::Arm as usize].weight as i32 / 2;
    base_to_hit += py().misc.weight as i32 / 10;

    if !monster_lit {
        base_to_hit /= 2;
        base_to_hit -= py().stats.used[A_DEX] as i32 * (BTH_PER_PLUS_TO_HIT_ADJUST - 1);
        base_to_hit -= py().misc.level as i32 * CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_BTH] as i32 / 2;
    }

    if player_test_being_hit(base_to_hit, py().misc.level as i32, py().stats.used[A_DEX] as i32, CREATURES_LIST[creature_id].ac as i32, CLASS_BTH) {
        let msg = format!("You hit {}.", name);
        print_message(Some(&msg));

        let arm_item = py().inventory[PlayerEquipment::Arm as usize];
        let mut damage = dice_roll(arm_item.damage);
        damage = player_weapon_critical_blow(arm_item.weight as i32 / 4 + py().stats.used[A_STR] as i32, 0, damage, CLASS_BTH);
        damage += py().misc.weight as i32 / 60;
        damage += 3;

        if damage < 0 {
            damage = 0;
        }

        // See if we done it in.
        if monster_take_hit(monster_id as i32, damage) >= 0 {
            let msg = format!("You have slain {}.", name);
            print_message(Some(&msg));
            display_character_experience();
        } else {
            // Capitalize
            let mut name = name;
            if let Some(first) = name.get_mut(0..1) {
                let capitalized = first.to_uppercase();
                name.replace_range(0..1, &capitalized);
            }

            // Can not stun Balrog
            let creature = &CREATURES_LIST[creature_id];
            let avg_max_hp = if (creature.defenses & config::monsters::defense::CD_MAX_HP) != 0 {
                max_dice_roll(creature.hit_die)
            } else {
                (creature.hit_die.dice as i32 * (creature.hit_die.sides as i32 + 1)) >> 1
            };

            let msg;
            if 100 + random_number(400) + random_number(400) > monsters()[monster_id].hp as i32 + avg_max_hp {
                monsters()[monster_id].stunned_amount += (random_number(3) + 1) as u8;
                if monsters()[monster_id].stunned_amount > 24 {
                    monsters()[monster_id].stunned_amount = 24;
                }

                msg = format!("{} appears stunned!", name);
            } else {
                msg = format!("{} ignores your bash!", name);
            }
            print_message(Some(&msg));
        }
    } else {
        let msg = format!("You miss {}.", name);
        print_message(Some(&msg));
    }

    if random_number(150) > py().stats.used[A_DEX] as i32 {
        print_message(Some("You are off balance."));
        py().flags.paralysis = (1 + random_number(2)) as i16;
    }
}

fn player_bash_position(coord: Coord) {
    // Is a Coward?
    if py().flags.afraid > 0 {
        print_message(Some("You are afraid!"));
        return;
    }

    player_bash_attack(coord);
}

fn player_bash_closed_door(coord: Coord, dir: i32) {
    print_message_no_command_interrupt("You smash into the door!");

    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;

    let chance = py().stats.used[A_STR] as i32 + py().misc.weight as i32 / 2;

    // Use (roughly) similar method as for monsters.
    let abs_misc_use = game().treasure.list[treasure_id].misc_use.abs() as i32;
    if random_number(chance * (20 + abs_misc_use)) < 10 * (chance - abs_misc_use) {
        print_message(Some("The door crashes open!"));

        inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[treasure_id]);

        // 50% chance of breaking door
        game().treasure.list[treasure_id].misc_use = (1 - random_number(2)) as i16;

        dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;

        if py().flags.confused == 0 {
            player_move(dir, false);
        } else {
            dungeon_lite_spot(coord);
        }

        return;
    }

    if random_number(150) > py().stats.used[A_DEX] as i32 {
        print_message(Some("You are off-balance."));
        py().flags.paralysis = (1 + random_number(2)) as i16;
        return;
    }

    if game().command_count == 0 {
        print_message(Some("The door holds firm."));
    }
}

fn player_bash_closed_chest(treasure_id: usize) {
    if random_number(10) == 1 {
        print_message(Some("You have destroyed the chest."));
        print_message(Some("and its contents!"));

        game().treasure.list[treasure_id].id = config::dungeon::objects::OBJ_RUINED_CHEST;
        game().treasure.list[treasure_id].flags = 0;

        return;
    }

    if (game().treasure.list[treasure_id].flags & config::treasure::chests::CH_LOCKED) != 0 && random_number(10) == 1 {
        print_message(Some("The lock breaks open!"));

        game().treasure.list[treasure_id].flags &= !config::treasure::chests::CH_LOCKED;

        return;
    }

    print_message_no_command_interrupt("The chest holds firm.");
}
