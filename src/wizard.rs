// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Version history and info, and wizard mode debugging aids.

use crate::config;
use crate::dungeon::{coord_in_bounds, dg, dungeon_delete_object, dungeon_place_random_object_near};
use crate::{tr, tr_fmt};
use crate::dungeon_tile::MAX_CAVE_FLOOR;
use crate::game::{game, random_number};
use crate::game_objects::popt;
use crate::helpers::string_to_number;
use crate::identification::{item_append_to_inscription, item_replace_inscription, item_set_as_identified, item_set_colorless_as_identified};
use crate::inventory::{inventory_item_copy_to, inventory_item_is_cursed, inventory_item_single_stackable, Inventory};
use crate::monster::update_monsters;
use crate::monster_manager::monster_summon;
use crate::player::{player_change_speed, py, A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS};
use crate::player_magic::{player_cure_blindness, player_cure_confusion, player_cure_poison, player_remove_fear};
use crate::player_stats::player_stat_restore;
use crate::spells::spell_remove_curse_from_all_worn_items;
use crate::treasure_magic::magic_treasure_magical_ability;
use crate::types::Coord;
use crate::ui::{
    display_character_experience, draw_dungeon_panel, print_character_current_hit_points, print_character_current_mana,
    print_character_gold_value, print_character_max_hit_points, print_character_speed,
};
use crate::ui_io::{get_command, get_input_confirmation, get_string_input, message_line_clear, print_message, put_string_clear_to_eol};

// lets anyone enter wizard mode after a disclaimer... -JEW-
pub fn enter_wizard_mode() -> bool {
    let mut answer = false;

    if game().noscore == 0 {
        print_message(Some(tr!("Wizard mode is for debugging and experimenting.")));
        answer = get_input_confirmation(tr!("The game will not be scored if you enter wizard mode. Are you sure?"));
    }

    if game().noscore != 0 || answer {
        game().noscore |= 0x2;
        game().wizard_mode = true;
        return true;
    }

    false
}

pub fn wizard_cure_all() {
    let _ = spell_remove_curse_from_all_worn_items();
    let _ = player_cure_blindness();
    let _ = player_cure_confusion();
    let _ = player_cure_poison();
    let _ = player_remove_fear();
    let _ = player_stat_restore(A_STR);
    let _ = player_stat_restore(A_INT);
    let _ = player_stat_restore(A_WIS);
    let _ = player_stat_restore(A_CON);
    let _ = player_stat_restore(A_DEX);
    let _ = player_stat_restore(A_CHR);

    if py().flags.slow > 1 {
        py().flags.slow = 1;
    }
    if py().flags.image > 1 {
        py().flags.image = 1;
    }
}

// Generate random items
pub fn wizard_drop_random_items() {
    let i;

    if game().command_count > 0 {
        i = game().command_count as i32;
        game().command_count = 0;
    } else {
        i = 1;
    }
    dungeon_place_random_object_near(py().pos, i);

    draw_dungeon_panel();
}

// Go up/down to specified depth
pub fn wizard_jump_level() {
    let mut i: i32;

    if game().command_count > 0 {
        if game().command_count > 99 {
            i = 0;
        } else {
            i = game().command_count as i32;
        }
        game().command_count = 0;
    } else {
        i = -1;
        let mut input = String::new();

        put_string_clear_to_eol(tr!("Go to which level (0-99) ? "), Coord::new(0, 0));

        if get_string_input(&mut input, Coord::new(0, 27), 10) {
            let _ = string_to_number(&input, &mut i);
        }
    }

    if i >= 0 {
        dg().current_level = i as i16;
        if dg().current_level > 99 {
            dg().current_level = 99;
        }
        dg().generate_new_level = true;
    } else {
        message_line_clear();
    }
}

// Increase Experience
pub fn wizard_gain_experience() {
    if game().command_count > 0 {
        py().misc.exp = game().command_count as i32;
        game().command_count = 0;
    } else if py().misc.exp == 0 {
        py().misc.exp = 1;
    } else {
        py().misc.exp *= 2;
    }
    display_character_experience();
}

// Summon a random monster
pub fn wizard_summon_monster() {
    let mut coord = Coord::new(py().pos.y, py().pos.x);

    let _ = monster_summon(&mut coord, true);

    update_monsters(false);
}

// Light up the dungeon -RAK-
pub fn wizard_light_up_dungeon() {
    let pos = py().pos;
    let flag = !dg().tile(pos).permanent_light;

    for y in 0..dg().height as i32 {
        for x in 0..dg().width as i32 {
            if dg().tile(Coord::new(y, x)).feature_id <= MAX_CAVE_FLOOR {
                for yy in (y - 1)..=(y + 1) {
                    for xx in (x - 1)..=(x + 1) {
                        let c = Coord::new(yy, xx);
                        dg().tile_mut(c).permanent_light = flag;
                        if !flag {
                            dg().tile_mut(c).field_mark = false;
                        }
                    }
                }
            }
        }
    }

    draw_dungeon_panel();
}

// Wizard routine for gaining on stats -RAK-
pub fn wizard_character_adjustment() {
    let mut input = String::new();
    let mut number: i32;

    put_string_clear_to_eol("(3 - 118) Strength     = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_STR] = number as u8;
            let _ = player_stat_restore(A_STR);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Intelligence = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_INT] = number as u8;
            let _ = player_stat_restore(A_INT);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Wisdom       = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_WIS] = number as u8;
            let _ = player_stat_restore(A_WIS);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Dexterity    = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_DEX] = number as u8;
            let _ = player_stat_restore(A_DEX);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Constitution = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_CON] = number as u8;
            let _ = player_stat_restore(A_CON);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(3 - 118) Charisma     = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 3) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 2 && number < 119 {
            py().stats.max[A_CHR] = number as u8;
            let _ = player_stat_restore(A_CHR);
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(1 - 32767) Hit points = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 5) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > 0 && number <= i16::MAX as i32 {
            py().misc.max_hp = number as i16;
            py().misc.current_hp = number as i16;
            py().misc.current_hp_fraction = 0;
            print_character_max_hit_points();
            print_character_current_hit_points();
        }
    } else {
        return;
    }

    put_string_clear_to_eol("(0 - 32767) Mana       = ", Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, 25), 5) {
        number = 0;
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 && number <= i16::MAX as i32 {
            py().misc.mana = number as i16;
            py().misc.current_mana = number as i16;
            py().misc.current_mana_fraction = 0;
            print_character_current_mana();
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  Gold = ", py().misc.au);
    // jdbkmoria extension: count chars, not bytes, so a translated prompt
    // with multi-byte UTF-8 (accented French) lands the input column at the
    // right display width; identical to `.len()` for ASCII text.
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 7) {
        let mut new_gold: i32 = 0;
        let valid_number = string_to_number(&input, &mut new_gold);
        if valid_number && new_gold > -1 {
            py().misc.au = new_gold;
            print_character_gold_value();
        }
    } else {
        return;
    }

    // jdbkmoria extension note: this prompt is deliberately left
    // untranslated. The quirk below (faithfully ported from the original
    // wizardCharacterAdjustment) stores the prompt STRING'S LENGTH -- not
    // the parsed input -- into chance_in_search; translating the label
    // would silently change that stat to a different value depending on
    // locale. Flagged as grammar/logic machinery in the report rather than
    // wrapped.
    let prompt = format!("Current={}  (0-200) Searching = ", py().misc.chance_in_search);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let mut new_gold: i32 = 0;
        let valid_number = string_to_number(&input, &mut new_gold);
        // Faithfully reproduces an upstream quirk (wizardCharacterAdjustment in
        // character.cpp / wizard.cpp): the freshly parsed `new_gold` is not
        // actually used here; the stale `number` (the prompt's string length,
        // reused above as the input column) gets stored instead.
        if valid_number && number > -1 && number < 201 {
            py().misc.chance_in_search = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  (-1-18) Stealth = ", py().misc.stealth_factor);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -2 && number < 19 {
            py().misc.stealth_factor = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  (0-200) Disarming = ", py().misc.disarm);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 && number < 201 {
            py().misc.disarm = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  (0-100) Save = ", py().misc.saving_throw);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 && number < 201 {
            py().misc.saving_throw = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  (0-200) Base to hit = ", py().misc.bth);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 && number < 201 {
            py().misc.bth = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  (0-200) Bows/Throwing = ", py().misc.bth_with_bows);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 && number < 201 {
            py().misc.bth_with_bows = number as i16;
        }
    } else {
        return;
    }

    let prompt = tr_fmt!("Current={}  Weight = ", py().misc.weight);
    number = prompt.chars().count() as i32;
    put_string_clear_to_eol(&prompt, Coord::new(0, 0));
    if get_string_input(&mut input, Coord::new(0, number), 3) {
        let valid_number = string_to_number(&input, &mut number);
        if valid_number && number > -1 {
            py().misc.weight = number as u16;
        }
    } else {
        return;
    }

    let mut command = ' ';
    while get_command(tr!("Alter speed? (+/-)"), &mut command) {
        if command == '+' {
            player_change_speed(-1);
        } else if command == '-' {
            player_change_speed(1);
        } else {
            break;
        }
        print_character_speed();
    }
}

// Request user input to get the array index of the `game_objects[]`
fn wizard_request_object_id(id: &mut i32, label: &str, start_id: i32, end_id: i32) -> bool {
    let id_str = format!("{}-{}", start_id, end_id);

    let msg = tr_fmt!("{} ID ({}): ", label, id_str);
    put_string_clear_to_eol(&msg, Coord::new(0, 0));

    let mut input = String::new();
    // jdbkmoria extension: count chars, not bytes, so a translated `label`
    // with multi-byte UTF-8 lands the input column at the right display
    // width; identical to `.len()` for ASCII text.
    if !get_string_input(&mut input, Coord::new(0, msg.chars().count() as i32), 3) {
        return false;
    }

    let mut given_id = 0;
    if !string_to_number(&input, &mut given_id) {
        return false;
    }

    if given_id < start_id || given_id > end_id {
        put_string_clear_to_eol(&tr_fmt!("Invalid ID. Must be {}", id_str), Coord::new(0, 0));
        return false;
    }
    *id = given_id;

    true
}

// Somethings been identified.
// Extra complexity by CJS so that it can merge store/dungeon objects when appropriate.
//
// Ported locally (rather than reusing `identification::item_identify()`) because
// that function was specialized to operate on the player's inventory, whereas
// this call site (wizardGenerateObject) identifies an item sitting on the
// dungeon floor, i.e. `game().treasure.list[*treasure_id]`. The logic below is
// otherwise identical to the original generic `itemIdentify(Inventory_t &, int &)`.
fn wizard_item_identify(treasure_id: &mut usize) {
    let item = game().treasure.list[*treasure_id];

    if inventory_item_is_cursed(&item) {
        item_append_to_inscription(&mut game().treasure.list[*treasure_id], config::identification::ID_DAMD);
    }

    if item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        return;
    }

    item_set_as_identified(item.category_id, item.sub_category_id);

    // no merging possible
    if !inventory_item_single_stackable(&item) {
        return;
    }

    let mut i: usize = 0;
    while i < py().pack.unique_items as usize {
        let t_ptr = py().inventory[i];

        let matching_cat = t_ptr.category_id == item.category_id;
        let matching_sub_cat = t_ptr.sub_category_id == item.sub_category_id;
        let total_items_count = t_ptr.items_count as i32 + item.items_count as i32;

        if matching_cat && matching_sub_cat && i != *treasure_id && total_items_count < 256 {
            let mut i_mut = i;

            // make *treasure_id the smaller number
            if *treasure_id > i {
                std::mem::swap(treasure_id, &mut i_mut);
            }

            print_message(Some(tr!("You combine similar objects from the shop and dungeon.")));

            py().inventory[*treasure_id].items_count += py().inventory[i_mut].items_count;
            py().pack.unique_items -= 1;

            let mut j = i_mut;
            while j < py().pack.unique_items as usize {
                py().inventory[j] = py().inventory[j + 1];
                j += 1;
            }

            let mut nothing = Inventory::empty();
            inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut nothing);
            py().inventory[j] = nothing;

            // the C loop variable took on the swapped index before continuing
            i = i_mut;
        }
        i += 1;
    }
}

// Simplified wizard routine for creating an object
pub fn wizard_generate_object() {
    let mut id: i32 = 0;
    if !wizard_request_object_id(&mut id, tr!("Dungeon/Store object"), 0, 366) {
        return;
    }

    let pos = py().pos;
    let mut coord = Coord::new(0, 0);

    for _ in 0..10 {
        coord.y = pos.y - 3 + random_number(5);
        coord.x = pos.x - 4 + random_number(7);

        if coord_in_bounds(coord)
            && dg().tile(coord).feature_id <= MAX_CAVE_FLOOR
            && dg().tile(coord).treasure_id == 0
        {
            // delete any object at location, before call popt()
            if dg().tile(coord).treasure_id != 0 {
                dungeon_delete_object(coord);
            }

            // place the object
            let free_treasure_id = popt();
            dg().tile_mut(coord).treasure_id = free_treasure_id as u8;
            inventory_item_copy_to(id as usize, &mut game().treasure.list[free_treasure_id as usize]);
            magic_treasure_magical_ability(free_treasure_id, dg().current_level as i32);

            // auto identify the item
            let mut treasure_id = free_treasure_id as usize;
            wizard_item_identify(&mut treasure_id);

            break;
        }
    }
}

// Wizard routine for creating objects -RAK-
pub fn wizard_create_objects() {
    print_message(Some(tr!("Warning: This routine can cause a fatal error.")));

    let mut item = Inventory::empty();

    item.id = config::dungeon::objects::OBJ_WIZARD;
    item.special_name_id = 0;
    item_replace_inscription(&mut item, tr!("wizard item"));
    item.identification = config::identification::ID_KNOWN2 | config::identification::ID_STORE_BOUGHT;

    let mut input = String::new();
    let mut number: i32;

    put_string_clear_to_eol(tr!("Tval   : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.category_id = number as u8;
    }

    put_string_clear_to_eol(tr!("Tchar  : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 1) {
        return;
    }
    item.sprite = input.as_bytes().first().copied().unwrap_or(0);

    put_string_clear_to_eol(tr!("Subval : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 5) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.sub_category_id = number as u8;
    }

    put_string_clear_to_eol(tr!("Weight : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 5) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.weight = number as u16;
    }

    put_string_clear_to_eol(tr!("Number : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 5) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.items_count = number as u8;
    }

    put_string_clear_to_eol(tr!("Damage (dice): "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 15), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.damage.dice = number as u8;
    }

    put_string_clear_to_eol(tr!("Damage (sides): "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 16), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.damage.sides = number as u8;
    }

    put_string_clear_to_eol(tr!("+To hit: "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.to_hit = number as i16;
    }

    put_string_clear_to_eol(tr!("+To dam: "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.to_damage = number as i16;
    }

    put_string_clear_to_eol(tr!("AC     : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.ac = number as i16;
    }

    put_string_clear_to_eol(tr!("+To AC : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.to_ac = number as i16;
    }

    put_string_clear_to_eol(tr!("P1     : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 5) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.misc_use = number as i16;
    }

    put_string_clear_to_eol(tr!("Flags (In HEX): "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 16), 8) {
        return;
    }
    // can't be constant string, this causes problems with
    // the GCC compiler and some scanf routines. (kept for fidelity with the
    // original comment; in Rust we just parse the hex string directly)
    item.flags = u32::from_str_radix(input.trim(), 16).unwrap_or(0);

    put_string_clear_to_eol(tr!("Cost : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 9), 8) {
        return;
    }
    let mut cost: i32 = 0;
    if string_to_number(&input, &mut cost) {
        item.cost = cost;
    }

    put_string_clear_to_eol(tr!("Level : "), Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 10), 3) {
        return;
    }
    number = 0;
    if string_to_number(&input, &mut number) {
        item.depth_first_found = number as u8;
    }

    if get_input_confirmation(tr!("Allocate?")) {
        // delete object first if any, before call popt()
        let pos = py().pos;

        if dg().tile(pos).treasure_id != 0 {
            dungeon_delete_object(pos);
        }

        let allocated_id = popt();

        game().treasure.list[allocated_id as usize] = item;
        dg().tile_mut(pos).treasure_id = allocated_id as u8;

        print_message(Some(tr!("Allocated.")));
    } else {
        print_message(Some(tr!("Aborted.")));
    }
}
