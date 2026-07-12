// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Code for potions

use crate::config;
use crate::dice::{dice_roll, Dice};
use crate::game::{game, random_number};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{item_identify, item_set_as_tried, item_set_colorless_as_identified, item_type_remaining_count_description};
use crate::inventory::{inventory_destroy_item, inventory_find_range};
use crate::player::{py, A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS};
use crate::player_eat::player_ingest_food;
use crate::player_magic::{
    player_cure_blindness, player_cure_confusion, player_cure_poison, player_detect_invisible,
    player_remove_fear,
};
use crate::player_stats::{player_stat_random_increase, player_stat_restore};
use crate::spells::{
    spell_change_player_hit_points, spell_lose_chr, spell_lose_exp, spell_lose_int,
    spell_lose_str, spell_lose_wis, spell_restore_player_levels, spell_slow_poison,
};
use crate::treasure::{TV_POTION1, TV_POTION2};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

// names based on PotionSpellTypes in player_quaff.cpp:
// Strength = 1, Weakness, RestoreStrength, Intelligence, LoseIntelligence, RestoreIntelligence,
// Wisdom, LoseWisdom, RestoreWisdom, Charisma, Ugliness, RestoreCharisma,
// CureLightWounds, CureSeriousWounds, CureCriticalWounds, Healing, Constitution, GainExperience,
// Sleep, Blindness, Confusion, Poison, HasteSelf, Slowness, (25 not used)
// Dexterity = 26, RestoreDexterity, RestoreConstitution, CureBlindness, CureConfusion, CurePoison,
// (32, 33 not used)
// LoseExperience = 34, SaltWater, Invulnerability, Heroism, SuperHeroism, Boldness,
// RestoreLifeLevels, ResistHeat, ResistCold, DetectInvisible, SlowPoison, NeutralizePoison,
// RestoreMana, InfraVision

fn player_drink_potion(flags: u32, item_type: u8) -> bool {
    let mut flags = flags;
    let mut identified = false;

    while flags != 0 {
        let mut potion_id = get_and_clear_first_bit(&mut flags) + 1;

        if item_type == TV_POTION2 {
            potion_id += 32;
        }

        // Potions
        match potion_id {
            1 => {
                // Strength
                if player_stat_random_increase(A_STR) {
                    print_message(Some("Wow!  What bulging muscles!"));
                    identified = true;
                }
            }
            2 => {
                // Weakness
                spell_lose_str();
                identified = true;
            }
            3 => {
                // Restore Strength
                if player_stat_restore(A_STR) {
                    print_message(Some("You feel warm all over."));
                    identified = true;
                }
            }
            4 => {
                // Intelligence
                if player_stat_random_increase(A_INT) {
                    print_message(Some("Aren't you brilliant!"));
                    identified = true;
                }
            }
            5 => {
                // Lose Intelligence
                spell_lose_int();
                identified = true;
            }
            6 => {
                // Restore Intelligence
                if player_stat_restore(A_INT) {
                    print_message(Some("You have have a warm feeling."));
                    identified = true;
                }
            }
            7 => {
                // Wisdom
                if player_stat_random_increase(A_WIS) {
                    print_message(Some("You suddenly have a profound thought!"));
                    identified = true;
                }
            }
            8 => {
                // Lose Wisdom
                spell_lose_wis();
                identified = true;
            }
            9 => {
                // Restore Wisdom
                if player_stat_restore(A_WIS) {
                    print_message(Some("You feel your wisdom returning."));
                    identified = true;
                }
            }
            10 => {
                // Charisma
                if player_stat_random_increase(A_CHR) {
                    print_message(Some("Gee, ain't you cute!"));
                    identified = true;
                }
            }
            11 => {
                // Ugliness
                spell_lose_chr();
                identified = true;
            }
            12 => {
                // Restore Charisma
                if player_stat_restore(A_CHR) {
                    print_message(Some("You feel your looks returning."));
                    identified = true;
                }
            }
            13 => {
                // Cure Light Wounds
                identified = spell_change_player_hit_points(dice_roll(Dice::new(2, 7)));
            }
            14 => {
                // Cure Serious Wounds
                identified = spell_change_player_hit_points(dice_roll(Dice::new(4, 7)));
            }
            15 => {
                // Cure Critical Wounds
                identified = spell_change_player_hit_points(dice_roll(Dice::new(6, 7)));
            }
            16 => {
                // Healing
                identified = spell_change_player_hit_points(1000);
            }
            17 => {
                // Constitution
                if player_stat_random_increase(A_CON) {
                    print_message(Some("You feel tingly for a moment."));
                    identified = true;
                }
            }
            18 => {
                // Gain Experience
                if py().misc.exp < config::player::PLAYER_MAX_EXP {
                    let mut exp = (py().misc.exp / 2) + 10;
                    if exp > 100_000 {
                        exp = 100_000;
                    }
                    py().misc.exp += exp;

                    print_message(Some("You feel more experienced."));
                    display_character_experience();
                    identified = true;
                }
            }
            19 => {
                // Sleep
                if !py().flags.free_action {
                    // paralysis must == 0, otherwise could not drink potion
                    print_message(Some("You fall asleep."));
                    py().flags.paralysis += (random_number(4) + 4) as i16;
                    identified = true;
                }
            }
            20 => {
                // Blindness
                if py().flags.blind == 0 {
                    print_message(Some("You are covered by a veil of darkness."));
                    identified = true;
                }
                py().flags.blind += (random_number(100) + 100) as i16;
            }
            21 => {
                // Confusion
                if py().flags.confused == 0 {
                    print_message(Some("Hey!  This is good stuff!  * Hick! *"));
                    identified = true;
                }
                py().flags.confused += (random_number(20) + 12) as i16;
            }
            22 => {
                // Poison
                if py().flags.poisoned == 0 {
                    print_message(Some("You feel very sick."));
                    identified = true;
                }
                py().flags.poisoned += (random_number(15) + 10) as i16;
            }
            23 => {
                // Haste Self
                if py().flags.fast == 0 {
                    identified = true;
                }
                py().flags.fast += (random_number(25) + 15) as i16;
            }
            24 => {
                // Slowness
                if py().flags.slow == 0 {
                    identified = true;
                }
                py().flags.slow += (random_number(25) + 15) as i16;
            }
            26 => {
                // Dexterity
                if player_stat_random_increase(A_DEX) {
                    print_message(Some("You feel more limber!"));
                    identified = true;
                }
            }
            27 => {
                // Restore Dexterity
                if player_stat_restore(A_DEX) {
                    print_message(Some("You feel less clumsy."));
                    identified = true;
                }
            }
            28 => {
                // Restore Constitution
                if player_stat_restore(A_CON) {
                    print_message(Some("You feel your health returning!"));
                    identified = true;
                }
            }
            29 => {
                // Cure Blindness
                identified = player_cure_blindness();
            }
            30 => {
                // Cure Confusion
                identified = player_cure_confusion();
            }
            31 => {
                // Cure Poison
                identified = player_cure_poison();
            }
            // case 33: no longer useful, now that there is a 'G'ain magic spells command
            34 => {
                // Lose Experience
                if py().misc.exp > 0 {
                    print_message(Some("You feel your memories fade."));

                    // Lose between 1/5 and 2/5 of your experience
                    let mut exp = py().misc.exp / 5;

                    if py().misc.exp > i16::MAX as i32 {
                        let scale = i32::MAX / py().misc.exp;
                        exp += (random_number(scale) * py().misc.exp) / (scale * 5);
                    } else {
                        exp += random_number(py().misc.exp) / 5;
                    }
                    spell_lose_exp(exp);
                    identified = true;
                }
            }
            35 => {
                // Salt Water
                let _ = player_cure_poison();
                if py().flags.food > 150 {
                    py().flags.food = 150;
                }
                py().flags.paralysis = 4;

                print_message(Some("The potion makes you vomit!"));
                identified = true;
            }
            36 => {
                // Invulnerability
                if py().flags.invulnerability == 0 {
                    identified = true;
                }
                py().flags.invulnerability += (random_number(10) + 10) as i16;
            }
            37 => {
                // Heroism
                if py().flags.heroism == 0 {
                    identified = true;
                }
                py().flags.heroism += (random_number(25) + 25) as i16;
            }
            38 => {
                // Super Heroism
                if py().flags.super_heroism == 0 {
                    identified = true;
                }
                py().flags.super_heroism += (random_number(25) + 25) as i16;
            }
            39 => {
                // Boldness
                identified = player_remove_fear();
            }
            40 => {
                // Restore Life Levels
                identified = spell_restore_player_levels();
            }
            41 => {
                // Resist Heat
                if py().flags.heat_resistance == 0 {
                    identified = true;
                }
                py().flags.heat_resistance += (random_number(10) + 10) as i16;
            }
            42 => {
                // Resist Cold
                if py().flags.cold_resistance == 0 {
                    identified = true;
                }
                py().flags.cold_resistance += (random_number(10) + 10) as i16;
            }
            43 => {
                // Detect Invisible
                if py().flags.detect_invisible == 0 {
                    identified = true;
                }
                player_detect_invisible(random_number(12) + 12);
            }
            44 => {
                // Slow Poison
                identified = spell_slow_poison();
            }
            45 => {
                // Neutralize Poison
                identified = player_cure_poison();
            }
            46 => {
                // Restore Mana
                if py().misc.current_mana < py().misc.mana {
                    py().misc.current_mana = py().misc.mana;
                    print_message(Some("Your feel your head clear."));
                    print_character_current_mana();
                    identified = true;
                }
            }
            47 => {
                // Infra-Vision
                if py().flags.timed_infra == 0 {
                    print_message(Some("Your eyes begin to tingle."));
                    identified = true;
                }
                py().flags.timed_infra += 100 + random_number(100) as i16;
            }
            _ => {
                // All cases are handled, so this should never be reached!
                print_message(Some("Internal error in potion()"));
            }
        }
    }

    identified
}

// Potions for the quaffing -RAK-
pub fn quaff() {
    game().player_free_turn = true;

    if py().pack.unique_items == 0 {
        print_message(Some("But you are not carrying anything."));
        return;
    }

    let mut item_pos_begin = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(TV_POTION1 as i32, TV_POTION2 as i32, &mut item_pos_begin, &mut item_pos_end) {
        print_message(Some("You are not carrying any potions."));
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, "Quaff which potion?", item_pos_begin, item_pos_end, None, None) {
        return;
    }
    let item_id = item_id as usize;

    game().player_free_turn = false;

    let mut item = py().inventory[item_id];

    let identified = if item.flags == 0 {
        print_message(Some("You feel less thirsty."));
        true
    } else {
        player_drink_potion(item.flags, item.category_id)
    };

    if identified {
        if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
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
    item_type_remaining_count_description(item_id);
    inventory_destroy_item(item_id);
}
