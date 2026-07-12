// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Note: naming of all the scroll functions needs verifying -MRC-

use crate::config;
use crate::dice::max_dice_roll;
use crate::dungeon::dg;
use crate::game::{game, random_number};
use crate::helpers::get_and_clear_first_bit;
use crate::identification::{
    item_description, item_identify, item_remove_magic_naming, item_set_as_tried,
    item_set_colorless_as_identified, item_type_remaining_count_description,
};
use crate::inventory::{
    inventory_destroy_item, inventory_find_range, inventory_item_remove_curse, PlayerEquipment,
};
use crate::monster::monster_sleep;
use crate::monster_manager::{monster_summon, monster_summon_undead};
use crate::player::{
    player_adjust_bonuses_for_item, player_no_light, player_recalculate_bonuses, player_teleport,
    player_worn_item_is_cursed, py,
};
use crate::player_magic::{player_bless, player_protect_evil};
use crate::spells::{
    spell_create_food, spell_darken_area, spell_destroy_adjacent_doors_traps, spell_destroy_area,
    spell_detect_invisible_creatures_within_vicinity, spell_detect_objects_within_vicinity,
    spell_detect_secret_doors_within_vicinity, spell_detect_traps_within_vicinity,
    spell_detect_treasure_within_vicinity, spell_dispel_creature, spell_enchant_item,
    spell_aggravate_monsters, spell_genocide, spell_identify_item, spell_light_area,
    spell_map_current_area, spell_mass_genocide, spell_recharge_item,
    spell_remove_curse_from_all_worn_items, spell_surround_player_with_doors,
    spell_surround_player_with_traps, spell_warding_glyph,
};
use crate::treasure::{TV_DIGGING, TV_HAFTED, TV_NOTHING, TV_SCROLL1, TV_SCROLL2};
use crate::ui::display_character_experience;
use crate::ui_inventory::inventory_get_input_for_item_id;
use crate::ui_io::print_message;

fn player_can_read_scroll(item_pos_start: &mut i32, item_pos_end: &mut i32) -> bool {
    if py().flags.blind > 0 {
        print_message(Some("You can't see to read the scroll."));
        return false;
    }

    if player_no_light() {
        print_message(Some("You have no light to read by."));
        return false;
    }

    if py().flags.confused > 0 {
        print_message(Some("You are too confused to read a scroll."));
        return false;
    }

    if py().pack.unique_items == 0 {
        print_message(Some("You are not carrying anything!"));
        return false;
    }

    if !inventory_find_range(TV_SCROLL1 as i32, TV_SCROLL2 as i32, item_pos_start, item_pos_end) {
        print_message(Some("You are not carrying any scrolls!"));
        return false;
    }

    true
}

fn inventory_item_id_of_cursed_equipment() -> usize {
    let mut items: [usize; 6] = [0; 6];
    let mut item_count = 0;

    if py().inventory[PlayerEquipment::Body as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Body as usize;
        item_count += 1;
    }
    if py().inventory[PlayerEquipment::Arm as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Arm as usize;
        item_count += 1;
    }
    if py().inventory[PlayerEquipment::Outer as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Outer as usize;
        item_count += 1;
    }
    if py().inventory[PlayerEquipment::Hands as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Hands as usize;
        item_count += 1;
    }
    if py().inventory[PlayerEquipment::Head as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Head as usize;
        item_count += 1;
    }
    // also enchant boots
    if py().inventory[PlayerEquipment::Feet as usize].category_id != TV_NOTHING {
        items[item_count] = PlayerEquipment::Feet as usize;
        item_count += 1;
    }

    let mut item_id = 0;

    if item_count > 0 {
        item_id = items[(random_number(item_count as i32) - 1) as usize];
    }

    if player_worn_item_is_cursed(PlayerEquipment::Body) {
        item_id = PlayerEquipment::Body as usize;
    } else if player_worn_item_is_cursed(PlayerEquipment::Arm) {
        item_id = PlayerEquipment::Arm as usize;
    } else if player_worn_item_is_cursed(PlayerEquipment::Outer) {
        item_id = PlayerEquipment::Outer as usize;
    } else if player_worn_item_is_cursed(PlayerEquipment::Head) {
        item_id = PlayerEquipment::Head as usize;
    } else if player_worn_item_is_cursed(PlayerEquipment::Hands) {
        item_id = PlayerEquipment::Hands as usize;
    } else if player_worn_item_is_cursed(PlayerEquipment::Feet) {
        item_id = PlayerEquipment::Feet as usize;
    }

    item_id
}

fn scroll_enchant_weapon_to_hit() -> bool {
    let item = py().inventory[PlayerEquipment::Wield as usize];

    if item.category_id == TV_NOTHING {
        return false;
    }

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows faintly!", desc);
    print_message(Some(&msg));

    if spell_enchant_item(&mut py().inventory[PlayerEquipment::Wield as usize].to_hit, 10) {
        inventory_item_remove_curse(&mut py().inventory[PlayerEquipment::Wield as usize]);
        player_recalculate_bonuses();
    } else {
        print_message(Some("The enchantment fails."));
    }

    true
}

fn scroll_enchant_weapon_to_damage() -> bool {
    let item = py().inventory[PlayerEquipment::Wield as usize];

    if item.category_id == TV_NOTHING {
        return false;
    }

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows faintly!", desc);
    print_message(Some(&msg));

    let scroll_type: i16 = if item.category_id >= TV_HAFTED && item.category_id <= TV_DIGGING {
        max_dice_roll(item.damage) as i16
    } else {
        // Bows' and arrows' enchantments should not be
        // limited by their low base damages
        10
    };

    if spell_enchant_item(&mut py().inventory[PlayerEquipment::Wield as usize].to_damage, scroll_type) {
        inventory_item_remove_curse(&mut py().inventory[PlayerEquipment::Wield as usize]);
        player_recalculate_bonuses();
    } else {
        print_message(Some("The enchantment fails."));
    }

    true
}

fn scroll_enchant_item_to_ac() -> bool {
    let item_id = inventory_item_id_of_cursed_equipment();

    if item_id == 0 {
        return false;
    }

    let item = py().inventory[item_id];

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows faintly!", desc);
    print_message(Some(&msg));

    if spell_enchant_item(&mut py().inventory[item_id].to_ac, 10) {
        inventory_item_remove_curse(&mut py().inventory[item_id]);
        player_recalculate_bonuses();
    } else {
        print_message(Some("The enchantment fails."));
    }

    true
}

fn scroll_identify_item(item_id: usize, is_used_up: &mut bool) -> usize {
    print_message(Some("This is an identify scroll."));

    *is_used_up = spell_identify_item();

    // The identify may merge objects, causing the identify scroll
    // to move to a different place.  Check for that here.  It can
    // move arbitrarily far if an identify scroll was used on
    // another identify scroll, but it always moves down.
    let mut item_id = item_id;
    let mut item = py().inventory[item_id];
    while item_id > 0 && (item.category_id != TV_SCROLL1 || item.flags != 0x0000_0008) {
        item_id -= 1;
        item = py().inventory[item_id];
    }

    item_id
}

fn scroll_remove_curse() -> bool {
    if spell_remove_curse_from_all_worn_items() {
        print_message(Some("You feel as if someone is watching over you."));
        return true;
    }
    false
}

fn scroll_summon_monster() -> bool {
    let mut identified = false;

    for _ in 0..random_number(3) {
        let mut coord = py().pos;
        identified |= monster_summon(&mut coord, false);
    }

    identified
}

fn scroll_teleport_level() {
    dg().current_level += -3 + 2 * random_number(2) as i16;
    if dg().current_level < 1 {
        dg().current_level = 1;
    }
    dg().generate_new_level = true;
}

fn scroll_confuse_monster() -> bool {
    if !py().flags.confuse_monster {
        print_message(Some("Your hands begin to glow."));
        py().flags.confuse_monster = true;
        return true;
    }
    false
}

fn scroll_enchant_weapon() -> bool {
    let item = py().inventory[PlayerEquipment::Wield as usize];

    if item.category_id == TV_NOTHING {
        return false;
    }

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows brightly!", desc);
    print_message(Some(&msg));

    let mut enchanted = false;

    for _ in 0..random_number(2) {
        if spell_enchant_item(&mut py().inventory[PlayerEquipment::Wield as usize].to_hit, 10) {
            enchanted = true;
        }
    }

    let scroll_type: i16 = if item.category_id >= TV_HAFTED && item.category_id <= TV_DIGGING {
        max_dice_roll(item.damage) as i16
    } else {
        // Bows' and arrows' enchantments should not be limited
        // by their low base damages
        10
    };

    for _ in 0..random_number(2) {
        if spell_enchant_item(&mut py().inventory[PlayerEquipment::Wield as usize].to_damage, scroll_type) {
            enchanted = true;
        }
    }

    if enchanted {
        inventory_item_remove_curse(&mut py().inventory[PlayerEquipment::Wield as usize]);
        player_recalculate_bonuses();
    } else {
        print_message(Some("The enchantment fails."));
    }

    true
}

fn scroll_curse_weapon() -> bool {
    let item = py().inventory[PlayerEquipment::Wield as usize];

    if item.category_id == TV_NOTHING {
        return false;
    }

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows black, fades.", desc);
    print_message(Some(&msg));

    item_remove_magic_naming(&mut py().inventory[PlayerEquipment::Wield as usize]);

    py().inventory[PlayerEquipment::Wield as usize].to_hit = -(random_number(5) as i16) - random_number(5) as i16;
    py().inventory[PlayerEquipment::Wield as usize].to_damage = -(random_number(5) as i16) - random_number(5) as i16;
    py().inventory[PlayerEquipment::Wield as usize].to_ac = 0;

    // Must call playerAdjustBonusesForItem() before set (clear) flags, and
    // must call playerRecalculateBonuses() after set (clear) flags, so that
    // all attributes will be properly turned off.
    let item = py().inventory[PlayerEquipment::Wield as usize];
    player_adjust_bonuses_for_item(item, -1);
    py().inventory[PlayerEquipment::Wield as usize].flags = config::treasure::flags::TR_CURSED;
    player_recalculate_bonuses();

    true
}

fn scroll_enchant_armor() -> bool {
    let item_id = inventory_item_id_of_cursed_equipment();

    if item_id == 0 {
        return false;
    }

    let item = py().inventory[item_id];

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows brightly!", desc);
    print_message(Some(&msg));

    let mut enchanted = false;

    for _ in 0..(random_number(2) + 1) {
        if spell_enchant_item(&mut py().inventory[item_id].to_ac, 10) {
            enchanted = true;
        }
    }

    if enchanted {
        inventory_item_remove_curse(&mut py().inventory[item_id]);
        player_recalculate_bonuses();
    } else {
        print_message(Some("The enchantment fails."));
    }

    true
}

fn scroll_curse_armor() -> bool {
    let item_id: usize;

    if py().inventory[PlayerEquipment::Body as usize].category_id != TV_NOTHING && random_number(4) == 1 {
        item_id = PlayerEquipment::Body as usize;
    } else if py().inventory[PlayerEquipment::Arm as usize].category_id != TV_NOTHING && random_number(3) == 1 {
        item_id = PlayerEquipment::Arm as usize;
    } else if py().inventory[PlayerEquipment::Outer as usize].category_id != TV_NOTHING && random_number(3) == 1 {
        item_id = PlayerEquipment::Outer as usize;
    } else if py().inventory[PlayerEquipment::Head as usize].category_id != TV_NOTHING && random_number(3) == 1 {
        item_id = PlayerEquipment::Head as usize;
    } else if py().inventory[PlayerEquipment::Hands as usize].category_id != TV_NOTHING && random_number(3) == 1 {
        item_id = PlayerEquipment::Hands as usize;
    } else if py().inventory[PlayerEquipment::Feet as usize].category_id != TV_NOTHING && random_number(3) == 1 {
        item_id = PlayerEquipment::Feet as usize;
    } else if py().inventory[PlayerEquipment::Body as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Body as usize;
    } else if py().inventory[PlayerEquipment::Arm as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Arm as usize;
    } else if py().inventory[PlayerEquipment::Outer as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Outer as usize;
    } else if py().inventory[PlayerEquipment::Head as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Head as usize;
    } else if py().inventory[PlayerEquipment::Hands as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Hands as usize;
    } else if py().inventory[PlayerEquipment::Feet as usize].category_id != TV_NOTHING {
        item_id = PlayerEquipment::Feet as usize;
    } else {
        item_id = 0;
    }

    if item_id == 0 {
        return false;
    }

    let item = py().inventory[item_id];

    let desc = item_description(&item, false);
    let msg = format!("Your {} glows black, fades.", desc);
    print_message(Some(&msg));

    item_remove_magic_naming(&mut py().inventory[item_id]);

    py().inventory[item_id].flags = config::treasure::flags::TR_CURSED;
    py().inventory[item_id].to_hit = 0;
    py().inventory[item_id].to_damage = 0;
    py().inventory[item_id].to_ac = -(random_number(5) as i16) - random_number(5) as i16;

    player_recalculate_bonuses();

    true
}

fn scroll_summon_undead() -> bool {
    let mut identified = false;

    for _ in 0..random_number(3) {
        let mut coord = py().pos;
        identified |= monster_summon_undead(&mut coord);
    }

    identified
}

fn scroll_word_of_recall() {
    if py().flags.word_of_recall == 0 {
        py().flags.word_of_recall = 25 + random_number(30) as i16;
    }
    print_message(Some("The air about you becomes charged."));
}

// Scrolls for the reading -RAK-
pub fn scroll_read() {
    game().player_free_turn = true;

    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !player_can_read_scroll(&mut item_pos_start, &mut item_pos_end) {
        return;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, "Read which scroll?", item_pos_start, item_pos_end, None, None) {
        return;
    }
    let mut item_id = item_id as usize;

    // From here on, no free turn for the player
    game().player_free_turn = false;

    let mut used_up = true;
    let mut identified = false;

    // The C++ code keeps a raw pointer to the original scroll's inventory slot for
    // the remainder of the loop; only `category_id` is read from it (item_id may be
    // reassigned below by the identify-scroll case, but that never retargets `item`).
    let original_item_category_id = py().inventory[item_id].category_id;
    let mut item_flags = py().inventory[item_id].flags;

    while item_flags != 0 {
        let mut scroll_type = get_and_clear_first_bit(&mut item_flags) + 1;

        if original_item_category_id == TV_SCROLL2 {
            scroll_type += 32;
        }

        match scroll_type {
            1 => identified = scroll_enchant_weapon_to_hit(),
            2 => identified = scroll_enchant_weapon_to_damage(),
            3 => identified = scroll_enchant_item_to_ac(),
            4 => {
                item_id = scroll_identify_item(item_id, &mut used_up);
                identified = true;
            }
            5 => identified = scroll_remove_curse(),
            6 => identified = spell_light_area(py().pos),
            7 => identified = scroll_summon_monster(),
            8 => {
                player_teleport(10); // Teleport Short, aka Phase Door
                identified = true;
            }
            9 => {
                player_teleport(100); // Teleport Long
                identified = true;
            }
            10 => {
                scroll_teleport_level();
                identified = true;
            }
            11 => identified = scroll_confuse_monster(),
            12 => {
                spell_map_current_area();
                identified = true;
            }
            13 => identified = monster_sleep(py().pos),
            14 => {
                spell_warding_glyph();
                identified = true;
            }
            15 => identified = spell_detect_treasure_within_vicinity(),
            16 => identified = spell_detect_objects_within_vicinity(),
            17 => identified = spell_detect_traps_within_vicinity(),
            18 => identified = spell_detect_secret_doors_within_vicinity(),
            19 => {
                print_message(Some("This is a mass genocide scroll."));
                let _ = spell_mass_genocide();
                identified = true;
            }
            20 => identified = spell_detect_invisible_creatures_within_vicinity(),
            21 => {
                print_message(Some("There is a high pitched humming noise."));
                let _ = spell_aggravate_monsters(20);
                identified = true;
            }
            22 => identified = spell_surround_player_with_traps(),
            23 => identified = spell_destroy_adjacent_doors_traps(),
            24 => identified = spell_surround_player_with_doors(),
            25 => {
                print_message(Some("This is a Recharge-Item scroll."));
                used_up = spell_recharge_item(60);
                identified = true;
            }
            26 => {
                print_message(Some("This is a genocide scroll."));
                let _ = spell_genocide();
                identified = true;
            }
            27 => identified = spell_darken_area(py().pos),
            28 => identified = player_protect_evil(),
            29 => {
                spell_create_food();
                identified = true;
            }
            30 => identified = spell_dispel_creature(config::monsters::defense::CD_UNDEAD as i32, 60),
            33 => identified = scroll_enchant_weapon(),
            34 => identified = scroll_curse_weapon(),
            35 => identified = scroll_enchant_armor(),
            36 => identified = scroll_curse_armor(),
            37 => identified = scroll_summon_undead(),
            38 => {
                player_bless(random_number(12) + 6);
                identified = true;
            }
            39 => {
                player_bless(random_number(24) + 12);
                identified = true;
            }
            40 => {
                player_bless(random_number(48) + 24);
                identified = true;
            }
            41 => {
                scroll_word_of_recall();
                identified = true;
            }
            42 => {
                spell_destroy_area(py().pos);
                identified = true;
            }
            _ => print_message(Some("Internal error in scroll()")),
        }
    }

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

    if used_up {
        item_type_remaining_count_description(item_id);
        inventory_destroy_item(item_id);
    }
}
