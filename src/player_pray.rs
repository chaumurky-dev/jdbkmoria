// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Code for priest spells

use crate::config;
use crate::data_player::{CLASSES, MAGIC_SPELLS};
use crate::dice::{dice_roll, Dice};
use crate::game::{game, get_direction_with_memory, random_number};
use crate::inventory::inventory_find_range;
use crate::inventory::inventory_item_remove_curse;
use crate::monster::monster_sleep;
use crate::player::{
    py, player_no_light, player_stat_random_decrease, player_teleport, A_CHR, A_CON, A_STR,
};
use crate::player_magic::{player_cure_poison, player_detect_invisible, player_protect_evil, player_remove_fear, player_bless};
use crate::spells::{
    spell_change_player_hit_points, spell_create_food, spell_detect_evil,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity,
    spell_dispel_creature, spell_earthquake, spell_fire_ball, spell_light_area,
    spell_map_current_area, spell_slow_poison, spell_turn_undead, spell_warding_glyph,
};
use crate::spells_data::MagicSpellFlags;
use crate::treasure::{TV_MAX_WEAR, TV_MIN_WEAR, TV_NEVER, TV_PRAYER_BOOK};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

fn player_can_pray(item_pos_begin: &mut i32, item_pos_end: &mut i32) -> bool {
    if py().flags.blind > 0 {
        print_message(Some(crate::tr!("You can't see to read your prayer!")));
        return false;
    }

    if player_no_light() {
        print_message(Some(crate::tr!("You have no light to read by.")));
        return false;
    }

    if py().flags.confused > 0 {
        print_message(Some(crate::tr!("You are too confused.")));
        return false;
    }

    if CLASSES[py().misc.class_id as usize].class_to_use_mage_spells != config::spells::SPELL_TYPE_PRIEST {
        print_message(Some(crate::tr!("Pray hard enough and your prayers may be answered.")));
        return false;
    }

    if py().pack.unique_items == 0 {
        print_message(Some(crate::tr!("But you are not carrying anything!")));
        return false;
    }

    if !inventory_find_range(TV_PRAYER_BOOK as i32, TV_NEVER as i32, item_pos_begin, item_pos_end) {
        print_message(Some(crate::tr!("You are not carrying any Holy Books!")));
        return false;
    }

    true
}

// names based on spell_names[62] in data_player.rs
// DetectEvil = 1, CureLightWounds, Bless, RemoveFear, CallLight, FindTraps, DetectDoorsStairs,
// SlowPoison, BlindCreature, Portal, CureMediumWounds, Chant, Sanctuary, CreateFood, RemoveCurse,
// ResistHeadCold, NeutralizePoison, OrbOfDraining, CureSeriousWounds, SenseInvisible,
// ProtectFromEvil, Earthquake, SenseSurroundings, CureCriticalWounds, TurnUndead, Prayer,
// DispelUndead, Heal, DispelEvil, GlyphOfWarding, HolyWord

// Recite a prayers.
fn player_recite_prayer(prayer_type: i32) {
    let mut dir = 0;

    match prayer_type + 1 {
        1 => {
            // Detect Evil
            let _ = spell_detect_evil();
        }
        2 => {
            // Cure Light Wounds
            let _ = spell_change_player_hit_points(dice_roll(Dice::new(3, 3)));
        }
        3 => {
            // Bless
            player_bless(random_number(12) + 12);
        }
        4 => {
            // Remove Fear
            let _ = player_remove_fear();
        }
        5 => {
            // Call Light
            let _ = spell_light_area(py().pos);
        }
        6 => {
            // Find Traps
            let _ = spell_detect_traps_within_vicinity();
        }
        7 => {
            // Detect Doors/Stairs
            let _ = spell_detect_secret_doors_within_vicinity();
        }
        8 => {
            // Slow Poison
            let _ = spell_slow_poison();
        }
        9 => {
            // Blind Creature
            if get_direction_with_memory(None, &mut dir) {
                let _ = crate::spells::spell_confuse_monster(py().pos, dir);
            }
        }
        10 => {
            // Portal
            player_teleport(py().misc.level as i32 * 3);
        }
        11 => {
            // Cure Medium Wounds
            let _ = spell_change_player_hit_points(dice_roll(Dice::new(4, 4)));
        }
        12 => {
            // Chant
            player_bless(random_number(24) + 24);
        }
        13 => {
            // Sanctuary
            let _ = monster_sleep(py().pos);
        }
        14 => {
            // Create Food
            spell_create_food();
        }
        15 => {
            // Remove Curse
            for entry in py().inventory.iter_mut() {
                // only clear flag for items that are wielded or worn
                if entry.category_id >= TV_MIN_WEAR && entry.category_id <= TV_MAX_WEAR {
                    inventory_item_remove_curse(entry);
                }
            }
        }
        16 => {
            // Resist Heat and Cold
            py().flags.heat_resistance += (random_number(10) + 10) as i16;
            py().flags.cold_resistance += (random_number(10) + 10) as i16;
        }
        17 => {
            // Neutralize Poison
            let _ = player_cure_poison();
        }
        18 => {
            // Orb of Draining
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_ball(
                    py().pos,
                    dir,
                    dice_roll(Dice::new(3, 6)) + py().misc.level as i32,
                    MagicSpellFlags::HolyOrb as i32,
                    crate::tr!("Black Sphere"),
                );
            }
        }
        19 => {
            // Cure Serious Wounds
            let _ = spell_change_player_hit_points(dice_roll(Dice::new(8, 4)));
        }
        20 => {
            // Sense Invisible
            player_detect_invisible(random_number(24) + 24);
        }
        21 => {
            // Protect From Evil
            let _ = player_protect_evil();
        }
        22 => {
            // Earthquake
            spell_earthquake();
        }
        23 => {
            // Sense Surroundings
            spell_map_current_area();
        }
        24 => {
            // Cure Critical Wounds
            let _ = spell_change_player_hit_points(dice_roll(Dice::new(16, 4)));
        }
        25 => {
            // Turn Undead
            let _ = spell_turn_undead();
        }
        26 => {
            // Prayer
            player_bless(random_number(48) + 48);
        }
        27 => {
            // Dispel Undead
            let _ = spell_dispel_creature(config::monsters::defense::CD_UNDEAD as i32, 3 * py().misc.level as i32);
        }
        28 => {
            // Heal
            let _ = spell_change_player_hit_points(200);
        }
        29 => {
            // Dispel Evil
            let _ = spell_dispel_creature(config::monsters::defense::CD_EVIL as i32, 3 * py().misc.level as i32);
        }
        30 => {
            // Glyph of Warding
            spell_warding_glyph();
        }
        31 => {
            // Holy Word
            let _ = player_remove_fear();
            let _ = player_cure_poison();
            let _ = spell_change_player_hit_points(1000);

            for stat in A_STR..=A_CHR {
                let _ = crate::player_stats::player_stat_restore(stat);
            }

            let _ = spell_dispel_creature(config::monsters::defense::CD_EVIL as i32, 4 * py().misc.level as i32);
            let _ = spell_turn_undead();

            if py().flags.invulnerability < 3 {
                py().flags.invulnerability = 3;
            } else {
                py().flags.invulnerability += 1;
            }
        }
        _ => {
            // All cases are handled, so this should never be reached!
        }
    }
}

// Pray like HELL. -RAK-
pub fn pray() {
    game().player_free_turn = true;

    let mut item_pos_begin = 0;
    let mut item_pos_end = 0;
    if !player_can_pray(&mut item_pos_begin, &mut item_pos_end) {
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, crate::tr!("Use which Holy Book?"), item_pos_begin, item_pos_end, None, None) {
        return;
    }

    let mut choice = 0;
    let mut chance = 0;
    let result = crate::spells::cast_spell_get_id(crate::tr!("Recite which prayer?"), item_id, &mut choice, &mut chance);
    if result < 0 {
        print_message(Some(crate::tr!("You don't know any prayers in that book.")));
        return;
    }
    if result == 0 {
        return;
    }

    let spell = MAGIC_SPELLS[py().misc.class_id as usize - 1][choice as usize];

    // NOTE: at least one function called by `player_recite_prayer()` sets `player_free_turn = true`,
    // e.g. `spell_create_food()`, so this check is required. -MRC-
    game().player_free_turn = false;

    if random_number(100) < chance {
        print_message(Some(crate::tr!("You lost your concentration!")));
    } else {
        player_recite_prayer(choice);

        if !game().player_free_turn && (py().flags.spells_worked & (1u32 << choice)) == 0 {
            py().misc.exp += (spell.exp_gain_for_learning as i32) << 2;
            display_character_experience();
            py().flags.spells_worked |= 1u32 << choice;
        }
    }

    if game().player_free_turn {
        return;
    }

    if spell.mana_required as i16 > py().misc.current_mana {
        print_message(Some(crate::tr!("You faint from fatigue!")));

        py().flags.paralysis = random_number(5 * (spell.mana_required as i32 - py().misc.current_mana as i32)) as i16;
        py().misc.current_mana = 0;
        py().misc.current_mana_fraction = 0;

        if random_number(3) == 1 {
            print_message(Some(crate::tr!("You have damaged your health!")));
            let _ = player_stat_random_decrease(A_CON);
        }
    } else {
        py().misc.current_mana -= spell.mana_required as i16;
    }

    print_character_current_mana();
}
