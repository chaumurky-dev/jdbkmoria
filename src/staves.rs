// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::data_player::{CLASS_LEVEL_ADJ, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::game::{game, get_random_direction, random_number};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_append_to_inscription, item_charges_remaining_description, item_identify,
    item_set_as_tried, item_set_colorless_as_identified, spell_item_identified,
};
use crate::inventory::inventory_find_range;
use crate::monster_manager::monster_summon;
use crate::player::{player_teleport, py, CLASS_DEVICE, A_INT};
use crate::player_magic::{player_cure_blindness, player_cure_confusion, player_cure_poison};
use crate::player_stats::player_stat_adjustment_wisdom_intelligence;
use crate::spells::{
    spell_build_wall, spell_change_monster_hit_points, spell_change_player_hit_points,
    spell_clone_monster, spell_confuse_monster, spell_darken_area, spell_destroy_area,
    spell_destroy_doors_traps_in_direction, spell_detect_evil,
    spell_detect_invisible_creatures_within_vicinity, spell_detect_objects_within_vicinity,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity,
    spell_detect_treasure_within_vicinity, spell_dispel_creature, spell_disarm_all_in_direction,
    spell_drain_life_from_monster, spell_earthquake, spell_fire_ball, spell_fire_bolt,
    spell_light_area, spell_light_line, spell_mass_polymorph, spell_polymorph_monster,
    spell_remove_curse_from_all_worn_items, spell_sleep_all_monsters, spell_sleep_monster,
    spell_speed_all_monsters, spell_speed_monster, spell_starlite, spell_teleport_away_monster_in_direction,
    spell_wall_to_mud,
};
use crate::spells_data::MagicSpellFlags;
use crate::tr;
use crate::treasure::{TV_NEVER, TV_STAFF, TV_WAND};
use crate::ui::display_character_experience;
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

// Staff spell types (StaffSpellTypes in staves.cpp), numbered here as plain
// integers to mirror the original `switch` on a cast-to-int enum:
// 1 StaffLight, 2 DetectDoorsStairs, 3 TrapLocation, 4 TreasureLocation,
// 5 ObjectLocation, 6 Teleportation, 7 Earthquakes, 8 Summoning, (9 skipped),
// 10 Destruction, 11 Starlight, 12 HasteMonsters, 13 SlowMonsters,
// 14 SleepMonsters, 15 CureLightWounds, 16 DetectInvisible, 17 Speed,
// 18 Slowness, 19 MassPolymorph, 20 RemoveCurse, 21 DetectEvil, 22 Curing,
// 23 DispelEvil, (24 skipped), 25 Darkness, (26-31 skipped), 32 StoreBoughtFlag.

fn staff_player_is_carrying(item_pos_start: &mut i32, item_pos_end: &mut i32) -> bool {
    if py().pack.unique_items == 0 {
        print_message(Some(tr!("But you are not carrying anything.")));
        return false;
    }

    if !inventory_find_range(TV_STAFF as i32, TV_NEVER as i32, item_pos_start, item_pos_end) {
        print_message(Some(tr!("You are not carrying any staffs.")));
        return false;
    }

    true
}

fn staff_player_can_use(item_id: usize) -> bool {
    let item = py().inventory[item_id];

    let mut chance = py().misc.saving_throw as i32;
    chance += player_stat_adjustment_wisdom_intelligence(A_INT);
    chance -= item.depth_first_found as i32 - 5;
    chance += CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_DEVICE] as i32 * py().misc.level as i32 / 3;

    if py().flags.confused > 0 {
        chance /= 2;
    }

    // Give everyone a slight chance
    if chance < config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32
        && random_number(config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32 - chance + 1) == 1
    {
        chance = config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32;
    }

    if chance < 1 {
        chance = 1;
    }

    if random_number(chance) < config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32 {
        print_message(Some(tr!("You failed to use the staff properly.")));
        return false;
    }

    if item.misc_use < 1 {
        print_message(Some(tr!("The staff has no charges left.")));
        if !spell_item_identified(&item) {
            item_append_to_inscription(&mut py().inventory[item_id], config::identification::ID_EMPTY);
        }
        return false;
    }

    true
}

fn staff_discharge(item_id: usize) -> bool {
    let mut identified = false;

    py().inventory[item_id].misc_use -= 1;

    let mut flags = py().inventory[item_id].flags;
    while flags != 0 {
        match get_and_clear_first_bit(&mut flags) + 1 {
            1 => identified = spell_light_area(py().pos), // StaffLight
            2 => identified = spell_detect_secret_doors_within_vicinity(), // DetectDoorsStairs
            3 => identified = spell_detect_traps_within_vicinity(), // TrapLocation
            4 => identified = spell_detect_treasure_within_vicinity(), // TreasureLocation
            5 => identified = spell_detect_objects_within_vicinity(), // ObjectLocation
            6 => {
                // Teleportation
                player_teleport(100);
                identified = true;
            }
            7 => {
                // Earthquakes
                identified = true;
                spell_earthquake();
            }
            8 => {
                // Summoning
                identified = false;

                for _ in 0..random_number(4) {
                    let mut coord = py().pos;
                    identified |= monster_summon(&mut coord, false);
                }
            }
            10 => {
                // Destruction
                identified = true;
                spell_destroy_area(py().pos);
            }
            11 => {
                // Starlight
                identified = true;
                spell_starlite(py().pos);
            }
            12 => identified = spell_speed_all_monsters(1), // HasteMonsters
            13 => identified = spell_speed_all_monsters(-1), // SlowMonsters
            14 => identified = spell_sleep_all_monsters(), // SleepMonsters
            15 => identified = spell_change_player_hit_points(random_number(8)), // CureLightWounds
            16 => identified = spell_detect_invisible_creatures_within_vicinity(), // DetectInvisible
            17 => {
                // Speed
                if py().flags.fast == 0 {
                    identified = true;
                }
                py().flags.fast += random_number(30) as i16 + 15;
            }
            18 => {
                // Slowness
                if py().flags.slow == 0 {
                    identified = true;
                }
                py().flags.slow += random_number(30) as i16 + 15;
            }
            19 => identified = spell_mass_polymorph(), // MassPolymorph
            20 => {
                // RemoveCurse
                if spell_remove_curse_from_all_worn_items() {
                    if py().flags.blind < 1 {
                        print_message(Some(tr!("The staff glows blue for a moment..")));
                    }
                    identified = true;
                }
            }
            21 => identified = spell_detect_evil(), // DetectEvil
            22 => {
                // Curing
                let a = player_cure_blindness();
                let b = player_cure_poison();
                let c = player_cure_confusion();
                if a || b || c {
                    identified = true;
                }
            }
            23 => identified = spell_dispel_creature(config::monsters::defense::CD_EVIL as i32, 60), // DispelEvil
            25 => identified = spell_darken_area(py().pos), // Darkness
            32 => {
                // StoreBoughtFlag: store bought flag, no-op
            }
            _ => {
                // All cases are handled, so this should never be reached!
                print_message(Some("Internal error in staffs()"));
            }
        }
    }

    identified
}

// Use a staff. -RAK-
pub fn staff_use() {
    game().player_free_turn = true;

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !staff_player_is_carrying(&mut item_pos_start, &mut item_pos_end) {
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, tr!("Use which staff?"), item_pos_start, item_pos_end, None, None) {
        return;
    }
    let mut item_id = item_id as usize;

    // From here on player uses up a turn
    game().player_free_turn = false;

    if !staff_player_can_use(item_id) {
        return;
    }

    let identified = staff_discharge(item_id);

    let item = py().inventory[item_id];

    if identified {
        if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
            // round half-way case up
            py().misc.exp += (item.depth_first_found as i32 + (py().misc.level as i32 >> 1)) / py().misc.level as i32;

            display_character_experience();

            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        item_set_as_tried(&item);
    }

    item_charges_remaining_description(item_id);
}

// Wand spell types (WandSpellTypes in staves.cpp), numbered here as plain
// integers to mirror the original `switch` on a cast-to-int enum:
// 1 WandLight, 2 LightningBolt, 3 FrostBolt, 4 FireBolt, 5 StoneToMud,
// 6 Polymorph, 7 HealMonster, 8 HasteMonster, 9 SlowMonster, 10 ConfuseMonster,
// 11 SleepMonster, 12 DrainLife, 13 TrapDoorDestruction, 14 WandMagicMissile,
// 15 WallBuilding, 16 CloneMonster, 17 TeleportAway, 18 Disarming,
// 19 LightningBall, 20 ColdBall, 21 FireBall, 22 StinkingCloud, 23 AcidBall,
// 24 Wonder.

fn wand_discharge(item_id: usize, direction: i32) -> bool {
    // decrement "use" variable
    py().inventory[item_id].misc_use -= 1;

    let mut identified = false;
    let mut flags = py().inventory[item_id].flags;

    while flags != 0 {
        let coord = py().pos;

        // Wand types
        match get_and_clear_first_bit(&mut flags) + 1 {
            1 => {
                // WandLight
                print_message(Some(tr!("A line of blue shimmering light appears.")));
                spell_light_line(py().pos, direction);
                identified = true;
            }
            2 => {
                // LightningBolt
                spell_fire_bolt(coord, direction, dice_roll(Dice::new(4, 8)), MagicSpellFlags::Lightning as i32, tr!(SPELL_NAMES[8]));
                identified = true;
            }
            3 => {
                // FrostBolt
                spell_fire_bolt(coord, direction, dice_roll(Dice::new(6, 8)), MagicSpellFlags::Frost as i32, tr!(SPELL_NAMES[14]));
                identified = true;
            }
            4 => {
                // FireBolt
                spell_fire_bolt(coord, direction, dice_roll(Dice::new(9, 8)), MagicSpellFlags::Fire as i32, tr!(SPELL_NAMES[22]));
                identified = true;
            }
            5 => identified = spell_wall_to_mud(coord, direction), // StoneToMud
            6 => identified = spell_polymorph_monster(coord, direction), // Polymorph
            7 => identified = spell_change_monster_hit_points(coord, direction, -dice_roll(Dice::new(4, 6))), // HealMonster
            8 => identified = spell_speed_monster(coord, direction, 1), // HasteMonster
            9 => identified = spell_speed_monster(coord, direction, -1), // SlowMonster
            10 => identified = spell_confuse_monster(coord, direction), // ConfuseMonster
            11 => identified = spell_sleep_monster(coord, direction), // SleepMonster
            12 => identified = spell_drain_life_from_monster(coord, direction), // DrainLife
            13 => identified = spell_destroy_doors_traps_in_direction(coord, direction), // TrapDoorDestruction
            14 => {
                // WandMagicMissile
                spell_fire_bolt(coord, direction, dice_roll(Dice::new(2, 6)), MagicSpellFlags::MagicMissile as i32, tr!(SPELL_NAMES[0]));
                identified = true;
            }
            15 => identified = spell_build_wall(coord, direction), // WallBuilding
            16 => identified = spell_clone_monster(coord, direction), // CloneMonster
            17 => identified = spell_teleport_away_monster_in_direction(coord, direction), // TeleportAway
            18 => identified = spell_disarm_all_in_direction(coord, direction), // Disarming
            19 => {
                // LightningBall
                spell_fire_ball(coord, direction, 32, MagicSpellFlags::Lightning as i32, tr!("Lightning Ball"));
                identified = true;
            }
            20 => {
                // ColdBall
                spell_fire_ball(coord, direction, 48, MagicSpellFlags::Frost as i32, tr!("Cold Ball"));
                identified = true;
            }
            21 => {
                // FireBall
                spell_fire_ball(coord, direction, 72, MagicSpellFlags::Fire as i32, tr!(SPELL_NAMES[28]));
                identified = true;
            }
            22 => {
                // StinkingCloud
                spell_fire_ball(coord, direction, 12, MagicSpellFlags::PoisonGas as i32, tr!(SPELL_NAMES[6]));
                identified = true;
            }
            23 => {
                // AcidBall
                spell_fire_ball(coord, direction, 60, MagicSpellFlags::Acid as i32, tr!("Acid Ball"));
                identified = true;
            }
            24 => {
                // Wonder
                flags = (1u32) << (random_number(23) - 1);
            }
            _ => {
                // All cases are handled, so this should never be reached!
                print_message(Some("Internal error in wands()"));
            }
        }
    }

    identified
}

// Wands for the aiming.
pub fn wand_aim() {
    game().player_free_turn = true;

    if py().pack.unique_items == 0 {
        print_message(Some(tr!("But you are not carrying anything.")));
        return;
    }

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(TV_WAND as i32, TV_NEVER as i32, &mut item_pos_start, &mut item_pos_end) {
        print_message(Some(tr!("You are not carrying any wands.")));
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, tr!("Aim which wand?"), item_pos_start, item_pos_end, None, None) {
        return;
    }
    let mut item_id = item_id as usize;

    game().player_free_turn = false;

    let mut direction = 0;
    if !crate::game::get_direction_with_memory(None, &mut direction) {
        return;
    }

    if py().flags.confused > 0 {
        print_message(Some(tr!("You are confused.")));
        direction = get_random_direction();
    }

    let item = py().inventory[item_id];

    let player_class_lev_adj = CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_DEVICE] as i32 * py().misc.level as i32 / 3;
    let mut chance = py().misc.saving_throw as i32 + player_stat_adjustment_wisdom_intelligence(A_INT) - item.depth_first_found as i32 + player_class_lev_adj;

    if py().flags.confused > 0 {
        chance /= 2;
    }

    if chance < config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32
        && random_number(config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32 - chance + 1) == 1
    {
        chance = config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32; // Give everyone a slight chance
    }

    if chance <= 0 {
        chance = 1;
    }

    if random_number(chance) < config::player::PLAYER_USE_DEVICE_DIFFICULTY as i32 {
        print_message(Some(tr!("You failed to use the wand properly.")));
        return;
    }

    if item.misc_use < 1 {
        print_message(Some(tr!("The wand has no charges left.")));
        if !spell_item_identified(&item) {
            item_append_to_inscription(&mut py().inventory[item_id], config::identification::ID_EMPTY);
        }
        return;
    }

    let identified = wand_discharge(item_id, direction);

    let item = py().inventory[item_id];

    if identified {
        if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
            // round half-way case up
            py().misc.exp += (item.depth_first_found as i32 + (py().misc.level as i32 >> 1)) / py().misc.level as i32;
            display_character_experience();

            item_identify(&mut item_id);
        }
    } else if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        item_set_as_tried(&item);
    }

    item_charges_remaining_description(item_id);
}
