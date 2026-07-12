// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Code for mage spells

use crate::config;
use crate::data_player::{CLASSES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::game::{game, get_direction_with_memory, random_number};
use crate::inventory::{inventory_find_range, inventory_item_remove_curse, PLAYER_INVENTORY_SIZE};
use crate::monster::monster_sleep;
use crate::player::{
    py, player_no_light, player_stat_random_decrease, player_teleport, A_CON, A_INT, A_WIS,
};
use crate::player_magic::player_cure_poison;
use crate::player_stats::player_stat_adjustment_wisdom_intelligence;
use crate::spells::{
    cast_spell_get_id, spell_change_player_hit_points, spell_confuse_monster,
    spell_destroy_adjacent_doors_traps, spell_destroy_area, spell_detect_monsters,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity, spell_fire_ball,
    spell_fire_bolt, spell_genocide, spell_identify_item, spell_light_area,
    spell_polymorph_monster, spell_recharge_item, spell_sleep_all_monsters, spell_sleep_monster,
    spell_speed_monster, spell_teleport_away_monster_in_direction, spell_wall_to_mud,
};
use crate::spells_data::MagicSpellFlags;
use crate::treasure::{TV_MAGIC_BOOK, TV_NEVER};
use crate::ui::{display_character_experience, print_character_current_mana};
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

// names based on SPELL_NAMES[62] in data_player.rs
// MagicMissile = 1, DetectMonsters, PhaseDoor, LightArea, CureLightWounds,
// FindHiddenTrapsDoors, StinkingCloud, Confusion, LightningBolt, TrapDoorDestruction,
// Sleep1, CurePoison, TeleportSelf, RemoveCurse, FrostBolt, WallToMud, CreateFood,
// RechargeItem1, Sleep2, PolymorphOther, IdentifyItem, Sleep3, FireBolt, SpeedMonster,
// FrostBall, RechargeItem2, TeleportOther, HasteSelf, FireBall, WordOfDestruction, Genocide

fn can_read_spells() -> bool {
    if py().flags.blind > 0 {
        print_message(Some("You can't see to read your spell book!"));
        return false;
    }

    if player_no_light() {
        print_message(Some("You have no light to read by."));
        return false;
    }

    if py().flags.confused > 0 {
        print_message(Some("You are too confused."));
        return false;
    }

    if CLASSES[py().misc.class_id as usize].class_to_use_mage_spells != config::spells::SPELL_TYPE_MAGE {
        print_message(Some("You can't cast spells!"));
        return false;
    }

    true
}

fn cast_spell(spell_id: i32) {
    let mut dir = 0;

    match spell_id {
        1 => {
            // Magic Missile
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_bolt(py().pos, dir, dice_roll(Dice::new(2, 6)), MagicSpellFlags::MagicMissile as i32, SPELL_NAMES[0]);
            }
        }
        2 => {
            // Detect Monsters
            let _ = spell_detect_monsters();
        }
        3 => {
            // Phase Door
            player_teleport(10);
        }
        4 => {
            // Light Area
            let _ = spell_light_area(py().pos);
        }
        5 => {
            // Cure Light Wounds
            let _ = spell_change_player_hit_points(dice_roll(Dice::new(4, 4)));
        }
        6 => {
            // Find Hidden Traps/Doors
            let _ = spell_detect_secret_doors_within_vicinity();
            let _ = spell_detect_traps_within_vicinity();
        }
        7 => {
            // Stinking Cloud
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_ball(py().pos, dir, 12, MagicSpellFlags::PoisonGas as i32, SPELL_NAMES[6]);
            }
        }
        8 => {
            // Confusion
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_confuse_monster(py().pos, dir);
            }
        }
        9 => {
            // Lightning Bolt
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_bolt(py().pos, dir, dice_roll(Dice::new(4, 8)), MagicSpellFlags::Lightning as i32, SPELL_NAMES[8]);
            }
        }
        10 => {
            // Trap Door Destruction
            let _ = spell_destroy_adjacent_doors_traps();
        }
        11 => {
            // Sleep I
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_sleep_monster(py().pos, dir);
            }
        }
        12 => {
            // Cure Poison
            let _ = player_cure_poison();
        }
        13 => {
            // Teleport Self
            player_teleport(py().misc.level as i32 * 5);
        }
        14 => {
            // Remove Curse
            for id in 22..PLAYER_INVENTORY_SIZE {
                inventory_item_remove_curse(&mut py().inventory[id]);
            }
        }
        15 => {
            // Frost Bolt
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_bolt(py().pos, dir, dice_roll(Dice::new(6, 8)), MagicSpellFlags::Frost as i32, SPELL_NAMES[14]);
            }
        }
        16 => {
            // Wall to Mud
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_wall_to_mud(py().pos, dir);
            }
        }
        17 => {
            // Create Food
            crate::spells::spell_create_food();
        }
        18 => {
            // Recharge Item I
            let _ = spell_recharge_item(20);
        }
        19 => {
            // Sleep II
            let _ = monster_sleep(py().pos);
        }
        20 => {
            // Polymorph Other
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_polymorph_monster(py().pos, dir);
            }
        }
        21 => {
            // Identify Item
            let _ = spell_identify_item();
        }
        22 => {
            // Sleep III
            let _ = spell_sleep_all_monsters();
        }
        23 => {
            // Fire Bolt
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_bolt(py().pos, dir, dice_roll(Dice::new(9, 8)), MagicSpellFlags::Fire as i32, SPELL_NAMES[22]);
            }
        }
        24 => {
            // Speed Monster
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_speed_monster(py().pos, dir, -1);
            }
        }
        25 => {
            // Frost Ball
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_ball(py().pos, dir, 48, MagicSpellFlags::Frost as i32, SPELL_NAMES[24]);
            }
        }
        26 => {
            // Recharge Item II
            let _ = spell_recharge_item(60);
        }
        27 => {
            // Teleport Other
            if get_direction_with_memory(None, &mut dir) {
                let _ = spell_teleport_away_monster_in_direction(py().pos, dir);
            }
        }
        28 => {
            // Haste Self
            py().flags.fast += (random_number(20) + py().misc.level as i32) as i16;
        }
        29 => {
            // Fire Ball
            if get_direction_with_memory(None, &mut dir) {
                spell_fire_ball(py().pos, dir, 72, MagicSpellFlags::Fire as i32, SPELL_NAMES[28]);
            }
        }
        30 => {
            // Word of Destruction
            spell_destroy_area(py().pos);
        }
        31 => {
            // Genocide
            let _ = spell_genocide();
        }
        _ => {
            // All cases are handled, so this should never be reached!
        }
    }
}

// Throw a magic spell -RAK-
pub fn get_and_cast_magic_spell() {
    game().player_free_turn = true;

    if !can_read_spells() {
        return;
    }

    let mut i = 0;
    let mut j = 0;
    if !inventory_find_range(TV_MAGIC_BOOK as i32, TV_NEVER as i32, &mut i, &mut j) {
        print_message(Some("But you are not carrying any spell-books!"));
        return;
    }

    let mut item_val: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_val, "Use which spell-book?", i, j, None, None) {
        return;
    }

    let mut choice = 0;
    let mut chance = 0;
    let result = cast_spell_get_id("Cast which spell?", item_val, &mut choice, &mut chance);
    if result < 0 {
        print_message(Some("You don't know any spells in that book."));
        return;
    }
    if result == 0 {
        return;
    }

    game().player_free_turn = false;

    let magic_spell = MAGIC_SPELLS[py().misc.class_id as usize - 1][choice as usize];

    if random_number(100) < chance {
        print_message(Some("You failed to get the spell off!"));
    } else {
        cast_spell(choice + 1);

        if !game().player_free_turn && (py().flags.spells_worked & (1u32 << choice)) == 0 {
            py().misc.exp += (magic_spell.exp_gain_for_learning as i32) << 2;
            py().flags.spells_worked |= 1u32 << choice;

            display_character_experience();
        }
    }

    if game().player_free_turn {
        return;
    }

    if magic_spell.mana_required as i16 > py().misc.current_mana {
        print_message(Some("You faint from the effort!"));

        py().flags.paralysis = random_number(5 * (magic_spell.mana_required as i32 - py().misc.current_mana as i32)) as i16;
        py().misc.current_mana = 0;
        py().misc.current_mana_fraction = 0;

        if random_number(3) == 1 {
            print_message(Some("You have damaged your health!"));
            let _ = player_stat_random_decrease(A_CON);
        }
    } else {
        py().misc.current_mana -= magic_spell.mana_required as i16;
    }

    print_character_current_mana();
}

// Returns spell chance of failure for class_to_use_mage_spells -RAK-
pub fn spell_chance_of_success(spell_id: i32) -> i32 {
    let class_id = py().misc.class_id as usize;
    let spell = MAGIC_SPELLS[class_id - 1][spell_id as usize];

    let mut chance = spell.failure_chance as i32 - 3 * (py().misc.level as i32 - spell.level_required as i32);

    let stat = if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        A_INT
    } else {
        A_WIS
    };

    chance -= 3 * (player_stat_adjustment_wisdom_intelligence(stat) - 1);

    if spell.mana_required as i16 > py().misc.current_mana {
        chance += 5 * (spell.mana_required as i32 - py().misc.current_mana as i32);
    }

    chance.clamp(5, 95)
}
