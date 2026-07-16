// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player movement, and stepping on traps/objects

use crate::config;
use crate::dice::dice_roll;
use crate::dungeon::{
    dg, dungeon_delete_object, dungeon_light_room, dungeon_move_character_light,
    dungeon_move_creature_record, dungeon_place_random_object_at, dungeon_place_rubble,
    dungeon_set_trap, trap_change_visibility,
};
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CLOSED_SPACE, TILE_LIGHT_FLOOR};
use crate::game::{game, random_number};
use crate::identification::item_description;
use crate::inventory::{
    damage_acid, damage_corroding_gas, damage_fire, damage_poisoned_gas,
    inventory_can_carry_item, inventory_can_carry_item_count, inventory_carry_item, Inventory,
};
use crate::monster::monsters;
use crate::paintings::{painting_index_at, paintings, player_attack_painting, PaintingKind};
use crate::player::{
    py, player_attack_position, player_move_position, player_search, player_takes_hit,
    player_test_being_hit, A_CON, A_STR, CLASS_MISC_HIT,
};
use crate::player_run::{player_area_affect, player_end_running};
use crate::player_stats::player_stat_random_decrease;
use crate::treasure::{TV_CLOSED_DOOR, TV_GOLD, TV_INVIS_TRAP, TV_MAX_PICK_UP, TV_RUBBLE, TV_STORE_DOOR, TV_VIS_TRAP};
use crate::types::Coord;
use crate::ui::{coord_outside_panel, draw_dungeon_panel, print_character_gold_value};
use crate::ui_io::{get_input_confirmation, print_message};

fn trap_open_pit(item: &Inventory, dam: i32) {
    print_message(Some("You fell into a pit!"));

    if py().flags.free_fall {
        print_message(Some("You gently float down."));
        return;
    }

    let description = item_description(item, true);
    player_takes_hit(dam, &description);
}

fn trap_arrow(item: &Inventory, dam: i32) {
    if player_test_being_hit(125, 0, 0, py().misc.ac as i32 + py().misc.magical_ac as i32, CLASS_MISC_HIT) {
        let description = item_description(item, true);
        player_takes_hit(dam, &description);

        print_message(Some("An arrow hits you."));
        return;
    }

    print_message(Some("An arrow barely misses you."));
}

fn trap_covered_pit(item: &Inventory, dam: i32, coord: Coord) {
    print_message(Some("You fell into a covered pit."));

    if py().flags.free_fall {
        print_message(Some("You gently float down."));
    } else {
        let description = item_description(item, true);
        player_takes_hit(dam, &description);
    }

    dungeon_set_trap(coord, 0);
}

fn trap_door(item: &Inventory, dam: i32) {
    dg().generate_new_level = true;
    dg().current_level += 1;

    print_message(Some("You fell through a trap door!"));

    if py().flags.free_fall {
        print_message(Some("You gently float down."));
    } else {
        let description = item_description(item, true);
        player_takes_hit(dam, &description);
    }

    // Force the messages to display before starting to generate the next level.
    print_message(None);
}

fn trap_sleeping_gas() {
    if py().flags.paralysis != 0 {
        return;
    }

    print_message(Some("A strange white mist surrounds you!"));

    if py().flags.free_action {
        print_message(Some("You are unaffected."));
        return;
    }

    py().flags.paralysis += (random_number(10) + 4) as i16;
    print_message(Some("You fall asleep."));
}

fn trap_hidden_object(coord: Coord) {
    dungeon_delete_object(coord);

    dungeon_place_random_object_at(coord, false);

    print_message(Some("Hmmm, there was something under this rock."));
}

fn trap_strength_dart(item: &Inventory, dam: i32) {
    if player_test_being_hit(125, 0, 0, py().misc.ac as i32 + py().misc.magical_ac as i32, CLASS_MISC_HIT) {
        if !py().flags.sustain_str {
            player_stat_random_decrease(A_STR);

            let description = item_description(item, true);
            player_takes_hit(dam, &description);

            print_message(Some("A small dart weakens you!"));
        } else {
            print_message(Some("A small dart hits you."));
        }
    } else {
        print_message(Some("A small dart barely misses you."));
    }
}

fn trap_teleport(coord: Coord) {
    game().teleport_player = true;

    print_message(Some("You hit a teleport trap!"));

    // Light up the teleport trap, before we teleport away.
    dungeon_move_character_light(coord, coord);
}

fn trap_rockfall(coord: Coord, dam: i32) {
    player_takes_hit(dam, "a falling rock");

    dungeon_delete_object(coord);
    dungeon_place_rubble(coord);

    print_message(Some("You are hit by falling rock."));
}

fn trap_corrode_gas() {
    print_message(Some("A strange red gas surrounds you."));

    damage_corroding_gas("corrosion gas");
}

fn trap_summon_monster(coord: Coord) {
    // Rune disappears.
    dungeon_delete_object(coord);

    let num = 2 + random_number(3);

    for _ in 0..num {
        let mut location = coord;
        crate::monster_manager::monster_summon(&mut location, false);
    }
}

fn trap_fire(dam: i32) {
    print_message(Some("You are enveloped in flames!"));

    damage_fire(dam, "a fire trap");
}

fn trap_acid(dam: i32) {
    print_message(Some("You are splashed with acid!"));

    damage_acid(dam, "an acid trap");
}

fn trap_poison_gas(dam: i32) {
    print_message(Some("A pungent green gas surrounds you!"));

    damage_poisoned_gas(dam, "a poison gas trap");
}

fn trap_blind_gas() {
    print_message(Some("A black gas surrounds you!"));

    py().flags.blind += (random_number(50) + 50) as i16;
}

fn trap_confuse_gas() {
    print_message(Some("A gas of scintillating colors surrounds you!"));

    py().flags.confused += (random_number(15) + 15) as i16;
}

fn trap_slow_dart(item: &Inventory, dam: i32) {
    if player_test_being_hit(125, 0, 0, py().misc.ac as i32 + py().misc.magical_ac as i32, CLASS_MISC_HIT) {
        let description = item_description(item, true);
        player_takes_hit(dam, &description);

        print_message(Some("A small dart hits you!"));

        if py().flags.free_action {
            print_message(Some("You are unaffected."));
        } else {
            py().flags.slow += (random_number(20) + 10) as i16;
        }
    } else {
        print_message(Some("A small dart barely misses you."));
    }
}

fn trap_constitution_dart(item: &Inventory, dam: i32) {
    if player_test_being_hit(125, 0, 0, py().misc.ac as i32 + py().misc.magical_ac as i32, CLASS_MISC_HIT) {
        if !py().flags.sustain_con {
            player_stat_random_decrease(A_CON);

            let description = item_description(item, true);
            player_takes_hit(dam, &description);

            print_message(Some("A small dart saps your health!"));
        } else {
            print_message(Some("A small dart hits you."));
        }
    } else {
        print_message(Some("A small dart barely misses you."));
    }
}

// Player hit a trap.  (Chuckle) -RAK-
fn player_steps_on_trap(coord: Coord) {
    player_end_running();
    trap_change_visibility(coord);

    let treasure_id = dg().tile(coord).treasure_id as usize;
    let item = game().treasure.list[treasure_id];

    let damage = dice_roll(item.damage);

    match item.sub_category_id {
        1 => trap_open_pit(&item, damage),
        2 => trap_arrow(&item, damage),
        3 => trap_covered_pit(&item, damage, coord),
        4 => trap_door(&item, damage),
        5 => trap_sleeping_gas(),
        6 => trap_hidden_object(coord),
        7 => trap_strength_dart(&item, damage),
        8 => trap_teleport(coord),
        9 => trap_rockfall(coord, damage),
        10 => trap_corrode_gas(),
        11 => trap_summon_monster(coord),
        12 => trap_fire(damage),
        13 => trap_acid(damage),
        14 => trap_poison_gas(damage),
        15 => trap_blind_gas(),
        16 => trap_confuse_gas(),
        17 => trap_slow_dart(&item, damage),
        18 => trap_constitution_dart(&item, damage),
        // Secret Door / Scare Monster
        19 | 99 => {}

        // Town level traps are special, the stores.
        101 => crate::store::store_enter(0),
        102 => crate::store::store_enter(1),
        103 => crate::store::store_enter(2),
        104 => crate::store::store_enter(3),
        105 => crate::store::store_enter(4),
        106 => crate::store::store_enter(5),

        _ => {
            // All cases are handled, so this should never be reached!
            print_message(Some("Unknown trap value."));
        }
    }
}

fn player_random_movement(dir: i32) -> bool {
    // Never random if sitting
    if dir == 5 {
        return false;
    }

    // 75% random movement
    let player_random_move = random_number(4) > 1;

    let player_is_confused = py().flags.confused > 0;

    player_is_confused && player_random_move
}

// Player is on an object. Many things can happen based -RAK-
// on the TVAL of the object. Traps are set off, money and most objects
// are picked up. Some objects, such as open doors, just sit there.
fn carry(coord: Coord, pickup: bool) {
    let mut pickup = pickup;

    let treasure_id = dg().tile(coord).treasure_id as usize;
    let item = game().treasure.list[treasure_id];

    let tile_flags = item.category_id;

    if tile_flags > TV_MAX_PICK_UP {
        if tile_flags == TV_INVIS_TRAP || tile_flags == TV_VIS_TRAP || tile_flags == TV_STORE_DOOR {
            // OOPS!
            player_steps_on_trap(coord);
        }
        return;
    }

    player_end_running();

    // There's GOLD in them thar hills!
    if tile_flags == TV_GOLD {
        py().misc.au += item.cost;

        let description = item_description(&item, true);
        let msg = format!("You have found {} gold pieces worth of {}", item.cost, description);

        print_character_gold_value();
        dungeon_delete_object(coord);

        print_message(Some(&msg));

        return;
    }

    // Too many objects?
    if inventory_can_carry_item_count(&item) {
        // Okay,  pick it up
        if pickup && config::options::options().prompt_to_pickup {
            let mut description = item_description(&item, true);

            // change the period to a question mark
            description.pop();
            description.push('?');
            pickup = get_input_confirmation(&format!("Pick up {}", description));
        }

        // Check to see if it will change the players speed.
        if pickup && !inventory_can_carry_item(&item) {
            let mut description = item_description(&item, true);

            // change the period to a question mark
            description.pop();
            description.push('?');
            pickup = get_input_confirmation(&format!("Exceed your weight limit to pick up {}", description));
        }

        // Attempt to pick up an object.
        if pickup {
            let mut item_copy = game().treasure.list[treasure_id];
            let locn = inventory_carry_item(&mut item_copy) as usize;
            game().treasure.list[treasure_id] = item_copy;

            let description = item_description(&py().inventory[locn], true);
            let msg = format!("You have {} ({})", description, (b'a' + locn as u8) as char);
            print_message(Some(&msg));
            dungeon_delete_object(coord);
        }
    } else {
        let description = item_description(&item, true);
        let msg = format!("You can't carry {}", description);
        print_message(Some(&msg));
    }
}

// Moves player from one space to another. -RAK-
pub fn player_move(direction: i32, do_pickup: bool) {
    let mut direction = direction;

    if player_random_movement(direction) {
        direction = random_number(9);
        player_end_running();
    }

    let mut coord = py().pos;

    // Legal move?
    if !player_move_position(direction, &mut coord) {
        return;
    }

    let tile = *dg().tile(coord);
    let monster_lit = monsters()[tile.creature_id as usize].lit;

    // if there is no creature, or an unlit creature in the walls then...
    // disallow attacks against unlit creatures in walls because moving into
    // a wall is a free turn normally, hence don't give player free turns
    // attacking each wall in an attempt to locate the invisible creature,
    // instead force player to tunnel into walls which always takes a turn
    if tile.creature_id < 2 || (!monster_lit && tile.feature_id >= MIN_CLOSED_SPACE) {
        // Open floor spot
        if tile.feature_id <= MAX_OPEN_SPACE {
            // Make final assignments of char coords
            let old_coord = py().pos;

            py().pos = coord;

            // Move character record (-1)
            dungeon_move_creature_record(old_coord, py().pos);

            // Check for new panel
            if coord_outside_panel(py().pos, false) {
                draw_dungeon_panel();
            }

            // Check to see if they should stop
            if py().running_tracker != 0 {
                player_area_affect(direction, py().pos);
            }

            // Check to see if they've noticed something
            // fos may be negative if have good rings of searching
            if py().misc.fos <= 1 || random_number(py().misc.fos as i32) == 1 || (py().flags.status & config::player::status::PY_SEARCH) != 0 {
                let chance = py().misc.chance_in_search as i32;
                player_search(py().pos, chance);
            }

            if tile.feature_id == TILE_LIGHT_FLOOR {
                // A room of light should be lit.

                if !tile.permanent_light && py().flags.blind == 0 {
                    dungeon_light_room(py().pos);
                }
            } else if tile.perma_lit_room && py().flags.blind < 1 {
                // In doorway of light-room?

                for row in (py().pos.y - 1)..=(py().pos.y + 1) {
                    for col in (py().pos.x - 1)..=(py().pos.x + 1) {
                        let neighbour_coord = Coord::new(row, col);
                        let neighbour = *dg().tile(neighbour_coord);
                        if neighbour.feature_id == TILE_LIGHT_FLOOR && !neighbour.permanent_light {
                            dungeon_light_room(neighbour_coord);
                        }
                    }
                }
            }

            // Move the light source
            dungeon_move_character_light(old_coord, py().pos);

            // An object is beneath them.
            if tile.treasure_id != 0 {
                carry(py().pos, do_pickup);

                // if stepped on falling rock trap, and space contains
                // rubble, then step back into a clear area
                if game().treasure.list[tile.treasure_id as usize].category_id == TV_RUBBLE {
                    dungeon_move_creature_record(py().pos, old_coord);
                    dungeon_move_character_light(py().pos, old_coord);

                    py().pos = old_coord;

                    // check to see if we have stepped back onto another trap, if so, set it off
                    let id = dg().tile(py().pos).treasure_id;
                    if id != 0 {
                        let val = game().treasure.list[id as usize].category_id;
                        if val == TV_INVIS_TRAP || val == TV_VIS_TRAP || val == TV_STORE_DOOR {
                            player_steps_on_trap(py().pos);
                        }
                    }
                }
            }
        } else {
            // Can't move onto floor space

            // jdbkmoria extension: an awake monster/swarm painting fights
            // back when you walk into it, instead of just bumping the wall.
            // Dormant ones (the ordinary case) fall through to the plain
            // wall bump below -- rousing them takes a look, a reach, or a
            // failed bash.
            if let Some(index) = painting_index_at(coord) {
                let painting = paintings()[index];
                let roused = painting.awake
                    && matches!(painting.kind, PaintingKind::MeleeMonster | PaintingKind::RangedMonster | PaintingKind::Swarm);

                if roused {
                    player_attack_painting(coord);
                    game().player_free_turn = false;
                    return;
                }
            }

            if py().running_tracker == 0 && tile.treasure_id != 0 {
                if game().treasure.list[tile.treasure_id as usize].category_id == TV_RUBBLE {
                    print_message(Some("There is rubble blocking your way."));
                } else if game().treasure.list[tile.treasure_id as usize].category_id == TV_CLOSED_DOOR {
                    print_message(Some("There is a closed door blocking your way."));
                }
            } else {
                player_end_running();
            }
            game().player_free_turn = true;
        }
    } else {
        // Attacking a creature!

        let old_find_flag = py().running_tracker;

        player_end_running();

        // if player can see monster, and was in find mode, then nothing
        if monster_lit && old_find_flag != 0 {
            // did not do anything this turn
            game().player_free_turn = true;
        } else {
            player_attack_position(coord);
        }
    }
}
