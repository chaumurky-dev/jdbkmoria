// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Food code

use crate::config;
use crate::dice::{dice_roll, Dice};
use crate::game::{game, random_number};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{item_identify, item_set_as_tried, item_set_colorless_as_identified, item_type_remaining_count_description};
use crate::inventory::{inventory_destroy_item, inventory_find_range};
use crate::player::{py, A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS};
use crate::player_magic::{player_cure_blindness, player_cure_confusion, player_cure_poison};
use crate::player_stats::player_stat_restore;
use crate::player::player_takes_hit;
use crate::spells::{spell_change_player_hit_points, spell_lose_con, spell_lose_str};
use crate::treasure::{TV_FOOD, TV_NEVER};
use crate::ui::{display_character_experience, draw_cave_panel, print_character_hunger_status};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

// names based on FoodMagicTypes in player_eat.cpp:
// Poison = 1, Blindness, Paranoia, Confusion, Hallucination, CurePoison,
// CureBlindness, CureParanoia, CureConfusion, Weakness, Unhealth,
// (12-15 no longer used)
// RestoreSTR = 16, RestoreCON, RestoreINT, RestoreWIS, RestoreDEX, RestoreCHR,
// FirstAid, MinorCures, LightCures, (25 no longer used) MajorCures = 26, PoisonousFood

// Eat some food. -RAK-
pub fn player_eat() {
    game().player_free_turn = true;

    if py().pack.unique_items == 0 {
        print_message(Some("But you are not carrying anything."));
        return;
    }

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(TV_FOOD as i32, TV_NEVER as i32, &mut item_pos_start, &mut item_pos_end) {
        print_message(Some("You are not carrying any food."));
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, "Eat what?", item_pos_start, item_pos_end, None, None) {
        return;
    }
    let item_id = item_id as usize;

    game().player_free_turn = false;

    let mut identified = false;

    let mut item = py().inventory[item_id];
    let mut item_flags = item.flags;

    while item_flags != 0 {
        match get_and_clear_first_bit(&mut item_flags) + 1 {
            1 => {
                // Poison
                py().flags.poisoned += (random_number(10) + item.depth_first_found as i32) as i16;
                identified = true;
            }
            2 => {
                // Blindness
                py().flags.blind += (random_number(250) + 10 * item.depth_first_found as i32 + 100) as i16;
                draw_cave_panel();
                print_message(Some("A veil of darkness surrounds you."));
                identified = true;
            }
            3 => {
                // Paranoia
                py().flags.afraid += (random_number(10) + item.depth_first_found as i32) as i16;
                print_message(Some("You feel terrified!"));
                identified = true;
            }
            4 => {
                // Confusion
                py().flags.confused += (random_number(10) + item.depth_first_found as i32) as i16;
                print_message(Some("You feel drugged."));
                identified = true;
            }
            5 => {
                // Hallucination
                py().flags.image += (random_number(200) + 25 * item.depth_first_found as i32 + 200) as i16;
                print_message(Some("You feel drugged."));
                identified = true;
            }
            6 => {
                // Cure Poison
                identified = player_cure_poison();
            }
            7 => {
                // Cure Blindness
                identified = player_cure_blindness();
            }
            8 => {
                // Cure Paranoia
                if py().flags.afraid > 1 {
                    py().flags.afraid = 1;
                    identified = true;
                }
            }
            9 => {
                // Cure Confusion
                identified = player_cure_confusion();
            }
            10 => {
                // Weakness
                spell_lose_str();
                identified = true;
            }
            11 => {
                // Unhealth
                spell_lose_con();
                identified = true;
            }
            // 12 through 15 are no longer used
            16 => {
                // Restore STR
                if player_stat_restore(A_STR) {
                    print_message(Some("You feel your strength returning."));
                    identified = true;
                }
            }
            17 => {
                // Restore CON
                if player_stat_restore(A_CON) {
                    print_message(Some("You feel your health returning."));
                    identified = true;
                }
            }
            18 => {
                // Restore INT
                if player_stat_restore(A_INT) {
                    print_message(Some("Your head spins a moment."));
                    identified = true;
                }
            }
            19 => {
                // Restore WIS
                if player_stat_restore(A_WIS) {
                    print_message(Some("You feel your wisdom returning."));
                    identified = true;
                }
            }
            20 => {
                // Restore DEX
                if player_stat_restore(A_DEX) {
                    print_message(Some("You feel more dexterous."));
                    identified = true;
                }
            }
            21 => {
                // Restore CHR
                if player_stat_restore(A_CHR) {
                    print_message(Some("Your skin stops itching."));
                    identified = true;
                }
            }
            22 => {
                // First Aid
                identified = spell_change_player_hit_points(random_number(6));
            }
            23 => {
                // Minor Cures
                identified = spell_change_player_hit_points(random_number(12));
            }
            24 => {
                // Light Cures
                identified = spell_change_player_hit_points(random_number(18));
            }
            // 25 is no longer used
            26 => {
                // Major Cures
                identified = spell_change_player_hit_points(dice_roll(Dice::new(3, 12)));
            }
            27 => {
                // Poisonous Food
                player_takes_hit(random_number(18), "poisonous food.");
                identified = true;
            }
            _ => {
                // All cases are handled, so this should never be reached!
                print_message(Some("Internal error in player_eat()"));
            }
        }
    }

    if identified {
        if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
            // use identified it, gain experience
            // round half-way case up
            py().misc.exp += (item.depth_first_found as i32 + (py().misc.level as i32 >> 1)) / py().misc.level as i32;

            display_character_experience();

            let mut item_id_mut = item_id;
            item_identify(&mut item_id_mut);
            item = py().inventory[item_id_mut];
        }
    } else if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        item_set_as_tried(&item);
    }

    player_ingest_food(item.misc_use as i32);

    py().flags.status &= !(config::player::status::PY_WEAK | config::player::status::PY_HUNGRY);

    print_character_hunger_status();

    item_type_remaining_count_description(item_id);
    inventory_destroy_item(item_id);
}

// Add to the players food time -RAK-
pub fn player_ingest_food(amount: i32) {
    if py().flags.food < 0 {
        py().flags.food = 0;
    }

    py().flags.food += amount as i16;

    if py().flags.food > config::player::PLAYER_FOOD_MAX as i16 {
        print_message(Some("You are bloated from overeating."));

        // Calculate how much of amount is responsible for the bloating. Give the
        // player food credit for 1/50, and also slow them for that many turns.
        let mut extra = py().flags.food as i32 - config::player::PLAYER_FOOD_MAX as i32;
        if extra > amount {
            extra = amount;
        }
        let penalty = extra / 50;

        py().flags.slow += penalty as i16;

        if extra == amount {
            py().flags.food = (py().flags.food as i32 - amount + penalty) as i16;
        } else {
            py().flags.food = (config::player::PLAYER_FOOD_MAX as i32 + penalty) as i16;
        }
    } else if py().flags.food > config::player::PLAYER_FOOD_FULL as i16 {
        print_message(Some("You are full."));
    }
}
