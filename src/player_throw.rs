// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player throw functions

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::dice_roll;
use crate::dungeon::{coord_in_bounds, dg, dungeon_lite_spot};
use crate::dungeon_tile::MAX_OPEN_SPACE;
use crate::game::{game, get_direction_with_memory, get_random_direction, random_number};
use crate::game_objects::popt;
use crate::identification::{item_description, item_type_remaining_count_description};
use crate::inventory::{inventory_destroy_item, Inventory, PlayerEquipment};
use crate::monster::{monster_take_hit, monsters};
use crate::player::{
    player_move_position, player_test_being_hit, player_weapon_critical_blow, py, A_STR,
    BTH_PER_PLUS_TO_HIT_ADJUST, CLASS_BTHB,
};
use crate::player_magic::item_magic_ability_damage;
use crate::treasure::{TV_ARROW, TV_BOLT, TV_BOW, TV_NOTHING, TV_SLING_AMMO};
use crate::ui::{coord_inside_panel, display_character_experience};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::{panel_put_tile, print_message, put_qio};

fn inventory_throw(item_id: usize, treasure: &mut Inventory) {
    let item = py().inventory[item_id];

    *treasure = item;

    if item.items_count > 1 {
        treasure.items_count = 1;
        py().inventory[item_id].items_count -= 1;
        py().pack.weight -= item.weight as i16;
        py().flags.status |= config::player::status::PY_STR_WGT;
    } else {
        inventory_destroy_item(item_id);
    }
}

// Obtain the hit and damage bonuses and the maximum distance for a thrown missile.
fn weapon_missile_facts(item: &Inventory, base_to_hit: &mut i32, plus_to_hit: &mut i32, damage: &mut i32, distance: &mut i32) {
    let mut weight = item.weight as i32;
    if weight < 1 {
        weight = 1;
    }

    // Throwing objects
    *damage = dice_roll(item.damage) + item.to_damage as i32;
    *base_to_hit = py().misc.bth_with_bows as i32 * 75 / 100;
    *plus_to_hit = py().misc.plusses_to_hit as i32 + item.to_hit as i32;

    // Add this back later if the correct throwing device. -CJS-
    if py().inventory[PlayerEquipment::Wield as usize].category_id != TV_NOTHING {
        *plus_to_hit -= py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
    }

    *distance = ((py().stats.used[A_STR] as i32 + 20) * 10) / weight;
    if *distance > 10 {
        *distance = 10;
    }

    // multiply damage bonuses instead of adding, when have proper
    // missile/weapon combo, this makes them much more useful

    // Using Bows, slings, or crossbows?
    if py().inventory[PlayerEquipment::Wield as usize].category_id != TV_BOW {
        return;
    }

    match py().inventory[PlayerEquipment::Wield as usize].misc_use {
        1 => {
            if item.category_id == TV_SLING_AMMO {
                // Sling and ammo
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 2;
                *distance = 20;
            }
        }
        2 => {
            if item.category_id == TV_ARROW {
                // Short Bow and Arrow
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 2;
                *distance = 25;
            }
        }
        3 => {
            if item.category_id == TV_ARROW {
                // Long Bow and Arrow
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 3;
                *distance = 30;
            }
        }
        4 => {
            if item.category_id == TV_ARROW {
                // Composite Bow and Arrow
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 4;
                *distance = 35;
            }
        }
        5 => {
            if item.category_id == TV_BOLT {
                // Light Crossbow and Bolt
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 3;
                *distance = 25;
            }
        }
        6 => {
            if item.category_id == TV_BOLT {
                // Heavy Crossbow and Bolt
                *base_to_hit = py().misc.bth_with_bows as i32;
                *plus_to_hit += 2 * py().inventory[PlayerEquipment::Wield as usize].to_hit as i32;
                *damage += py().inventory[PlayerEquipment::Wield as usize].to_damage as i32;
                *damage *= 4;
                *distance = 35;
            }
        }
        _ => {
            // NOOP
        }
    }
}

fn inventory_drop_or_throw_item(coord: crate::types::Coord, item: &Inventory) {
    let mut position = coord;

    let mut flag = false;

    if random_number(10) > 1 {
        let mut k = 0;
        while !flag && k <= 9 {
            if coord_in_bounds(position) {
                let tile = dg().floor[position.y as usize][position.x as usize];
                if tile.feature_id <= MAX_OPEN_SPACE && tile.treasure_id == 0 {
                    flag = true;
                }
            }

            if !flag {
                position.y = coord.y + random_number(3) - 2;
                position.x = coord.x + random_number(3) - 2;
                k += 1;
            }
        }
    }

    if flag {
        let cur_pos = popt();
        dg().floor[position.y as usize][position.x as usize].treasure_id = cur_pos as u8;
        game().treasure.list[cur_pos as usize] = *item;
        dungeon_lite_spot(position);
    } else {
        let description = item_description(item, false);
        let msg = format!("The {} disappears.", description);
        print_message(Some(&msg));
    }
}

// Throw an object across the dungeon. -RAK-
// Note: Flasks of oil do fire damage
// Note: Extra damage and chance of hitting when missiles are used
// with correct weapon. i.e. wield bow and throw arrow.
pub fn player_throw_item() {
    if py().pack.unique_items == 0 {
        print_message(Some("But you are not carrying anything."));
        game().player_free_turn = true;
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, "Fire/Throw which one?", 0, py().pack.unique_items as i32 - 1, None, None) {
        return;
    }
    let item_id = item_id as usize;

    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    item_type_remaining_count_description(item_id);

    if py().flags.confused > 0 {
        print_message(Some("You are confused."));
        dir = get_random_direction();
    }

    let mut thrown_item = Inventory::empty();
    inventory_throw(item_id, &mut thrown_item);

    let mut tbth = 0;
    let mut tpth = 0;
    let mut tdam = 0;
    let mut tdis = 0;
    weapon_missile_facts(&thrown_item, &mut tbth, &mut tpth, &mut tdam, &mut tdis);

    let tile_char = thrown_item.sprite as char;
    let mut current_distance = 0;

    let mut coord = py().pos;
    let mut old_coord = py().pos;

    let mut flag = false;

    while !flag {
        player_move_position(dir, &mut coord);
        current_distance += 1;
        dungeon_lite_spot(old_coord);

        if current_distance > tdis {
            flag = true;
        }

        let tile = dg().floor[coord.y as usize][coord.x as usize];

        if tile.feature_id <= MAX_OPEN_SPACE && !flag {
            if tile.creature_id > 1 {
                flag = true;

                let m_ptr = monsters()[tile.creature_id as usize];

                tbth -= current_distance;

                // if monster not lit, make it much more difficult to hit, subtract
                // off most bonuses, and reduce bth_with_bows depending on distance.
                if !m_ptr.lit {
                    tbth /= current_distance + 2;
                    tbth -= py().misc.level as i32 * CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_BTHB] as i32 / 2;
                    tbth -= tpth * (BTH_PER_PLUS_TO_HIT_ADJUST - 1);
                }

                if player_test_being_hit(tbth, py().misc.level as i32, tpth, CREATURES_LIST[m_ptr.creature_id as usize].ac as i32, CLASS_BTHB) {
                    let creature_id = m_ptr.creature_id as usize;

                    let description = item_description(&thrown_item, false);

                    // Does the player know what they're fighting?
                    let visible;
                    let msg;
                    if !m_ptr.lit {
                        msg = format!("You hear a cry as the {} finds a mark.", description);
                        visible = false;
                    } else {
                        msg = format!("The {} hits the {}.", description, CREATURES_LIST[creature_id].name);
                        visible = true;
                    }
                    print_message(Some(&msg));

                    tdam = item_magic_ability_damage(&thrown_item, tdam, creature_id);
                    tdam = player_weapon_critical_blow(thrown_item.weight as i32, tpth, tdam, CLASS_BTHB);

                    if tdam < 0 {
                        tdam = 0;
                    }

                    let kill_result = monster_take_hit(tile.creature_id as i32, tdam);

                    if kill_result >= 0 {
                        if !visible {
                            print_message(Some("You have killed something!"));
                        } else {
                            let msg2 = format!("You have killed the {}.", CREATURES_LIST[kill_result as usize].name);
                            print_message(Some(&msg2));
                        }
                        display_character_experience();
                    }
                } else {
                    inventory_drop_or_throw_item(old_coord, &thrown_item);
                }
            } else {
                // do not test tile.field_mark here
                if coord_inside_panel(coord) && py().flags.blind < 1 && (tile.temporary_light || tile.permanent_light) {
                    panel_put_tile(tile_char, coord);
                    put_qio(); // show object moving
                }
            }
        } else {
            flag = true;
            inventory_drop_or_throw_item(old_coord, &thrown_item);
        }

        old_coord = coord;
    }
}
