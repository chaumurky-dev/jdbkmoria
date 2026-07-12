// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config;
use crate::dungeon::dg;
use crate::game::{game, Screen};
use crate::identification::item_description;
use crate::inventory::{
    Inventory, PlayerEquipment, ITEM_GROUP_MIN, ITEM_SINGLE_STACK_MAX, PLAYER_INVENTORY_SIZE,
};
use crate::player::{py, A_STR};
use crate::treasure::{TV_MAX_WEAR, TV_MIN_WEAR, TV_NOTHING, TV_AMULET, TV_ARROW, TV_BOLT, TV_BOOTS, TV_BOW, TV_CLOAK, TV_DIGGING, TV_GLOVES, TV_HAFTED, TV_HARD_ARMOR, TV_HELM, TV_LIGHT, TV_POLEARM, TV_RING, TV_SHIELD, TV_SLING_AMMO, TV_SOFT_ARMOR, TV_SPIKE, TV_SWORD};
use crate::types::Coord;
use crate::ui::ESCAPE;
use crate::ui_io::{
    erase_line, get_command, get_input_confirmation, get_input_confirmation_with_abort,
    get_key_input, get_menu_item_id, message_line_clear, print_message, put_string,
    put_string_clear_to_eol, screen_has_changed, terminal_bell_sound, terminal_restore_screen,
    terminal_save_screen,
};

const WIELD: usize = PlayerEquipment::Wield as usize;
const AUXILIARY: usize = PlayerEquipment::Auxiliary as usize;
const LIGHT: usize = PlayerEquipment::Light as usize;

fn inventory_item_weight_text(item_id: usize) -> String {
    let total_weight = py().inventory[item_id].weight as i32 * py().inventory[item_id].items_count as i32;
    let quotient = total_weight / 10;
    let remainder = total_weight % 10;

    format!("{:3}.{} lb", quotient, remainder)
}

// Displays inventory items from `item_id_start` to `item_id_end` -RAK-
// Designed to keep the display as far to the right as possible. -CJS-
// The parameter col gives a column at which to start, but if the display
// does not fit, it may be moved left.  The return value is the left edge
// used. If mask is non-zero, then only display those items which have a
// non-zero entry in the mask array.
pub fn display_inventory_items(item_id_start: i32, item_id_end: i32, weighted: bool, column: i32, mask: Option<&[bool]>) -> i32 {
    let mut descriptions: Vec<String> = vec![String::new(); (item_id_end + 1).max(0) as usize];

    let mut len = 79 - column;

    let lim = if weighted { 68 } else { 76 };

    // Generate the descriptions text
    for i in item_id_start..=item_id_end {
        let i = i as usize;
        if let Some(mask) = mask {
            if !mask[i] {
                continue;
            }
        }

        let mut description = item_description(&py().inventory[i], true);

        // Truncate if too long.
        description.truncate(lim);

        descriptions[i] = format!("{}) {}", (b'a' + i as u8) as char, description);

        let mut l = descriptions[i].chars().count() as i32 + 2;

        if weighted {
            l += 9;
        }

        if l > len {
            len = l;
        }
    }

    let mut column = 79 - len;
    if column < 0 {
        column = 0;
    }

    let mut current_line = 1;

    // Print the descriptions
    for i in item_id_start..=item_id_end {
        let i = i as usize;
        if let Some(mask) = mask {
            if !mask[i] {
                continue;
            }
        }

        // don't need first two spaces if in first column
        if column == 0 {
            put_string_clear_to_eol(&descriptions[i], Coord::new(current_line, column));
        } else {
            put_string("  ", Coord::new(current_line, column));
            put_string_clear_to_eol(&descriptions[i], Coord::new(current_line, column + 2));
        }

        if weighted {
            let text = inventory_item_weight_text(i);
            put_string_clear_to_eol(&text, Coord::new(current_line, 71));
        }

        current_line += 1;
    }

    column
}

// Return a string describing how a given equipment item is carried. -CJS-
pub fn player_item_wearing_description(body_location: usize) -> &'static str {
    match body_location {
        x if x == PlayerEquipment::Wield as usize => "wielding",
        x if x == PlayerEquipment::Head as usize => "wearing on your head",
        x if x == PlayerEquipment::Neck as usize => "wearing around your neck",
        x if x == PlayerEquipment::Body as usize => "wearing on your body",
        x if x == PlayerEquipment::Arm as usize => "wearing on your arm",
        x if x == PlayerEquipment::Hands as usize => "wearing on your hands",
        x if x == PlayerEquipment::Right as usize => "wearing on your right hand",
        x if x == PlayerEquipment::Left as usize => "wearing on your left hand",
        x if x == PlayerEquipment::Feet as usize => "wearing on your feet",
        x if x == PlayerEquipment::Outer as usize => "wearing about your body",
        x if x == PlayerEquipment::Light as usize => "using to light the way",
        x if x == PlayerEquipment::Auxiliary as usize => "holding ready by your side",
        _ => "carrying in your pack",
    }
}

fn equipment_position_description(id: usize, weight: u16) -> &'static str {
    match id {
        x if x == PlayerEquipment::Wield as usize => {
            if (py().stats.used[A_STR] as i32) * 15 < weight as i32 {
                "Just lifting"
            } else {
                "Wielding"
            }
        }
        x if x == PlayerEquipment::Head as usize => "On head",
        x if x == PlayerEquipment::Neck as usize => "Around neck",
        x if x == PlayerEquipment::Body as usize => "On body",
        x if x == PlayerEquipment::Arm as usize => "On arm",
        x if x == PlayerEquipment::Hands as usize => "On hands",
        x if x == PlayerEquipment::Right as usize => "On right hand",
        x if x == PlayerEquipment::Left as usize => "On left hand",
        x if x == PlayerEquipment::Feet as usize => "On feet",
        x if x == PlayerEquipment::Outer as usize => "About body",
        x if x == PlayerEquipment::Light as usize => "Light source",
        x if x == PlayerEquipment::Auxiliary as usize => "Spare weapon",
        _ => "Unknown equipment position ID",
    }
}

// Displays equipment items from r1 to end -RAK-
// Keep display as far right as possible. -CJS-
pub fn display_equipment(show_weights: bool, column: i32) -> i32 {
    let mut descriptions: Vec<String> = Vec::with_capacity(PLAYER_INVENTORY_SIZE - WIELD);
    let mut weight_ids: Vec<usize> = Vec::with_capacity(PLAYER_INVENTORY_SIZE - WIELD);

    let mut len = 79 - column;

    let lim = if show_weights { 52 } else { 60 };

    // Range of equipment
    let mut line = 0;
    for i in WIELD..PLAYER_INVENTORY_SIZE {
        if py().inventory[i].category_id == TV_NOTHING {
            continue;
        }

        // Get position
        let equipped_description = equipment_position_description(i, py().inventory[i].weight);

        let mut description = item_description(&py().inventory[i], true);

        // Truncate if necessary
        description.truncate(lim);

        descriptions.push(format!("{}) {:<14}: {}", (b'a' + line as u8) as char, equipped_description, description));
        weight_ids.push(i);

        let mut l = descriptions[line].chars().count() as i32 + 2;

        if show_weights {
            l += 9;
        }

        if l > len {
            len = l;
        }

        line += 1;
    }

    let mut column = 79 - len;
    if column < 0 {
        column = 0;
    }

    // Range of equipment
    for (line, description) in descriptions.iter().enumerate() {
        let line_pos = line as i32 + 1;

        // don't need first two spaces when using whole screen
        if column == 0 {
            put_string_clear_to_eol(description, Coord::new(line_pos, column));
        } else {
            put_string("  ", Coord::new(line_pos, column));
            put_string_clear_to_eol(description, Coord::new(line_pos, column + 2));
        }

        if show_weights {
            let text = inventory_item_weight_text(weight_ids[line]);
            put_string_clear_to_eol(&text, Coord::new(line_pos, 71));
        }
    }
    erase_line(Coord::new(descriptions.len() as i32 + 1, column));

    column
}

fn show_equipment_help_menu(left_column: i32) -> i32 {
    let left_column = std::cmp::min(left_column, 52);

    put_string_clear_to_eol("  ESC: exit", Coord::new(1, left_column));
    put_string_clear_to_eol("  w  : wear or wield object", Coord::new(2, left_column));
    put_string_clear_to_eol("  t  : take off item", Coord::new(3, left_column));
    put_string_clear_to_eol("  d  : drop object", Coord::new(4, left_column));
    put_string_clear_to_eol("  x  : exchange weapons", Coord::new(5, left_column));
    put_string_clear_to_eol("  i  : inventory of pack", Coord::new(6, left_column));
    put_string_clear_to_eol("  e  : list used equipment", Coord::new(7, left_column));

    7 // current line position
}

// All inventory commands (wear, exchange, take off, drop, inventory and
// equipment) are handled in an alternative command input mode, which accepts
// any of the inventory commands.
//
// See the original umoria ui_inventory.cpp for the full description of this
// state machine; the behaviour is preserved as-is.

fn ui_command_switch_screen(next_screen: Screen) {
    if next_screen == game().screen.current_screen_id {
        return;
    }
    game().screen.current_screen_id = next_screen;

    let mut current_line_pos: i32 = 0;
    match next_screen {
        Screen::Blank => {}
        Screen::Help => {
            current_line_pos = show_equipment_help_menu(game().screen.screen_left_pos);
        }
        Screen::Inventory => {
            game().screen.screen_left_pos = display_inventory_items(
                0,
                py().pack.unique_items as i32 - 1,
                config::options::options().show_inventory_weights,
                game().screen.screen_left_pos,
                None,
            );
            current_line_pos = py().pack.unique_items as i32;
        }
        Screen::Wear => {
            game().screen.screen_left_pos = display_inventory_items(
                game().screen.wear_low_id,
                game().screen.wear_high_id,
                config::options::options().show_inventory_weights,
                game().screen.screen_left_pos,
                None,
            );
            current_line_pos = game().screen.wear_high_id - game().screen.wear_low_id + 1;
        }
        Screen::Equipment => {
            game().screen.screen_left_pos = display_equipment(config::options::options().show_inventory_weights, game().screen.screen_left_pos);
            current_line_pos = py().equipment_count as i32;
        }
        Screen::Wrong => {}
    }

    if current_line_pos >= game().screen.screen_bottom_pos {
        game().screen.screen_bottom_pos = current_line_pos + 1;
        erase_line(Coord::new(game().screen.screen_bottom_pos, game().screen.screen_left_pos));
        return;
    }
    current_line_pos += 1;

    while current_line_pos <= game().screen.screen_bottom_pos {
        erase_line(Coord::new(current_line_pos, game().screen.screen_left_pos));
        current_line_pos += 1;
    }
}

// Used to verify if this really is the item we wish to wear or read. -CJS-
fn verify_action(prompt: &str, item: usize) -> bool {
    let mut description = item_description(&py().inventory[item], true);

    // change the period to a question mark
    description.pop();
    description.push('?');

    get_input_confirmation(&format!("{} {}", prompt, description))
}

fn request_and_show_inventory_screen(recover_screen: bool) {
    if game().doing_inventory_command == '\0' {
        game().screen.screen_left_pos = 50;
        game().screen.screen_bottom_pos = 0;
        game().screen.current_screen_id = Screen::Blank; // this forces exit of inventory_execute_command() if selecting is not set true
        return;
    }

    // Take up where we left off after a previous inventory command. -CJS-

    // If the screen has been flushed, we need to redraw. If the command
    // is a simple ' ' to recover the screen, just quit. Otherwise, check
    // and see what the user wants.
    if *screen_has_changed() {
        if recover_screen || !get_input_confirmation("Continuing with inventory command?") {
            game().doing_inventory_command = '\0';
            return;
        }
        game().screen.screen_left_pos = 50;
        game().screen.screen_bottom_pos = 0;
    }

    let current_screen = game().screen.current_screen_id;
    game().screen.current_screen_id = Screen::Wrong;
    ui_command_switch_screen(current_screen);
}

fn ui_command_inventory_take_off_item(selecting: bool) -> bool {
    if py().equipment_count == 0 {
        print_message(Some("You are not using any equipment."));
        // don't print message restarting inven command after taking off something, it is confusing
        return selecting;
    }

    if py().pack.unique_items as usize >= WIELD && game().doing_inventory_command == '\0' {
        print_message(Some("You will have to drop something first."));
        return selecting;
    }

    if game().screen.current_screen_id != Screen::Blank {
        ui_command_switch_screen(Screen::Equipment);
    }

    true
}

fn ui_command_inventory_drop_item(command: &mut char, selecting: bool) -> bool {
    if py().pack.unique_items == 0 && py().equipment_count == 0 {
        print_message(Some("But you're not carrying anything."));
        return selecting;
    }

    if dg().tile(py().pos).treasure_id != 0 {
        print_message(Some("There's no room to drop anything here."));
        return selecting;
    }

    if (game().screen.current_screen_id == Screen::Equipment && py().equipment_count > 0) || py().pack.unique_items == 0 {
        if game().screen.current_screen_id != Screen::Blank {
            ui_command_switch_screen(Screen::Equipment);
        }
        *command = 'r'; // Remove - or take off and drop.
    } else if game().screen.current_screen_id != Screen::Blank {
        ui_command_switch_screen(Screen::Inventory);
    }

    true
}

fn ui_command_inventory_wear_wield_item(selecting: bool) -> bool {
    // Note: simple loop to get the global game.screen.wear_low_id value
    game().screen.wear_low_id = 0;
    while game().screen.wear_low_id < py().pack.unique_items as i32 && py().inventory[game().screen.wear_low_id as usize].category_id > TV_MAX_WEAR {
        game().screen.wear_low_id += 1;
    }

    // Note: simple loop to get the global wear_high value
    game().screen.wear_high_id = game().screen.wear_low_id;
    while game().screen.wear_high_id < py().pack.unique_items as i32 && py().inventory[game().screen.wear_high_id as usize].category_id >= TV_MIN_WEAR {
        game().screen.wear_high_id += 1;
    }
    game().screen.wear_high_id -= 1;

    if game().screen.wear_low_id > game().screen.wear_high_id {
        print_message(Some("You have nothing to wear or wield."));
        return selecting;
    }

    if game().screen.current_screen_id != Screen::Blank && game().screen.current_screen_id != Screen::Inventory {
        ui_command_switch_screen(Screen::Wear);
    }

    true
}

fn ui_command_inventory_unwield_item() {
    if !crate::player::player_is_wielding_item() {
        print_message(Some("But you are wielding no weapons."));
        return;
    }

    if crate::player::player_worn_item_is_cursed(PlayerEquipment::Wield) {
        let description = item_description(&py().inventory[WIELD], false);
        let msg = format!("The {} you are wielding appears to be cursed.", description);

        print_message(Some(&msg));

        return;
    }

    game().player_free_turn = false;

    // swap auxiliary and wield items
    let saved_item = py().inventory[AUXILIARY];
    py().inventory[AUXILIARY] = py().inventory[WIELD];
    py().inventory[WIELD] = saved_item;

    if game().screen.current_screen_id == Screen::Equipment {
        game().screen.screen_left_pos = display_equipment(config::options::options().show_inventory_weights, game().screen.screen_left_pos);
    }

    crate::player::player_adjust_bonuses_for_item(py().inventory[AUXILIARY], -1); // Subtract bonuses
    crate::player::player_adjust_bonuses_for_item(py().inventory[WIELD], 1); // Add bonuses

    if py().inventory[WIELD].category_id != TV_NOTHING {
        let mut label = String::from("Primary weapon   : ");
        label.push_str(&item_description(&py().inventory[WIELD], true));

        print_message(Some(&label));
    } else {
        print_message(Some("No primary weapon."));
    }

    // this is a new weapon, so clear the heavy flag
    py().weapon_is_heavy = false;
    crate::player::player_strength();
}

// look for item whose inscription matches `which`
fn inventory_get_item_matching_inscription(which: char, command: char, from: i32, to: i32) -> i32 {
    if ('0'..='9').contains(&which) && command != 'r' && command != 't' {
        // Note: simple loop to get id
        let mut m = from;
        while m <= to
            && (m as usize) < PLAYER_INVENTORY_SIZE
            && (py().inventory[m as usize].inscription[0] != which as u8 || py().inventory[m as usize].inscription[1] != 0)
        {
            m += 1;
        }

        if m <= to {
            m
        } else {
            -1
        }
    } else if which.is_ascii_uppercase() {
        which as i32 - 'A' as i32
    } else {
        which as i32 - 'a' as i32
    }
}

fn build_command_heading(from: i32, to: i32, swap: &str, command: char, prompt: &str) -> String {
    let from = (b'a' + from as u8) as char;
    let to = (b'a' + to as u8) as char;

    let list = if game().screen.current_screen_id == Screen::Blank {
        ", * to list"
    } else {
        ""
    };

    let digits = if command == 'w' || command == 'd' { ", 0-9" } else { "" };

    format!("({}-{}{}{}{}, space to break, ESC to exit) {} which one?", from, to, list, swap, digits, prompt)
}

fn change_screen_for_command(command: char) {
    if command == 't' || command == 'r' {
        ui_command_switch_screen(Screen::Equipment);
    } else if command == 'w' && game().screen.current_screen_id != Screen::Inventory {
        ui_command_switch_screen(Screen::Wear);
    } else {
        ui_command_switch_screen(Screen::Inventory);
    }
}

fn flip_inventory_equipment_screens() {
    if game().screen.current_screen_id == Screen::Equipment {
        ui_command_switch_screen(Screen::Inventory);
    } else if game().screen.current_screen_id == Screen::Inventory {
        ui_command_switch_screen(Screen::Equipment);
    }
}

fn request_put_ring_on_which_hand() -> i32 {
    let mut hand: i32 = 0;

    // Rings. Give choice over where they go.
    while hand == 0 {
        let mut query = '\0';
        if !get_menu_item_id("Put ring on which hand (l/r/L/R)?", &mut query) {
            hand = -1;
        } else if query == 'l' {
            hand = PlayerEquipment::Left as i32;
        } else if query == 'r' {
            hand = PlayerEquipment::Right as i32;
        } else {
            if query == 'L' {
                hand = PlayerEquipment::Left as i32;
            } else if query == 'R' {
                hand = PlayerEquipment::Right as i32;
            } else {
                terminal_bell_sound();
            }
            if hand != 0 && !verify_action("Replace", hand as usize) {
                hand = 0;
            }
        }
    }

    hand
}

fn inventory_get_slot_to_wear_equipment(category_id: u8) -> i32 {
    // Slot for equipment
    match category_id {
        TV_SLING_AMMO | TV_BOLT | TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_SWORD | TV_DIGGING | TV_SPIKE => {
            PlayerEquipment::Wield as i32
        }
        TV_LIGHT => PlayerEquipment::Light as i32,
        TV_BOOTS => PlayerEquipment::Feet as i32,
        TV_GLOVES => PlayerEquipment::Hands as i32,
        TV_CLOAK => PlayerEquipment::Outer as i32,
        TV_HELM => PlayerEquipment::Head as i32,
        TV_SHIELD => PlayerEquipment::Arm as i32,
        TV_HARD_ARMOR | TV_SOFT_ARMOR => PlayerEquipment::Body as i32,
        TV_AMULET => PlayerEquipment::Neck as i32,
        TV_RING => {
            if crate::player::player_right_hand_ring_empty() {
                PlayerEquipment::Right as i32
            } else if crate::player::player_left_hand_ring_empty() {
                PlayerEquipment::Left as i32
            } else {
                request_put_ring_on_which_hand()
            }
        }
        _ => {
            print_message(Some("IMPOSSIBLE: I don't see how you can use that."));
            -1
        }
    }
}

fn inventory_item_is_cursed_message(item_id: usize) {
    let description = item_description(&py().inventory[item_id], false);

    let mut msg = format!("The {} you are ", description);

    if item_id == PlayerEquipment::Head as usize {
        msg.push_str("wielding ");
    } else {
        msg.push_str("wearing ");
    }

    msg.push_str("appears to be cursed.");
    print_message(Some(&msg));
}

// Get its place in the equipment list.
fn execute_remove_item_command(selecting: bool, item_id: i32, command: &mut char, which: char, prompt: &str) -> bool {
    let mut selecting = selecting;
    let mut item_id_to_take_off = item_id;
    let mut item_id: i32 = 21;

    loop {
        item_id += 1;
        if py().inventory[item_id as usize].category_id != TV_NOTHING {
            item_id_to_take_off -= 1;
        }
        if item_id_to_take_off < 0 {
            break;
        }
    }

    if which.is_ascii_uppercase() && !verify_action(prompt, item_id as usize) {
        item_id = -1;
    } else if crate::inventory::inventory_item_is_cursed(&py().inventory[item_id as usize]) {
        item_id = -1;
        print_message(Some("Hmmm, it seems to be cursed."));
    } else if *command == 't' && !crate::inventory::inventory_can_carry_item_count(&py().inventory[item_id as usize]) {
        if dg().tile(py().pos).treasure_id != 0 {
            item_id = -1;
            print_message(Some("You can't carry it."));
        } else if get_input_confirmation("You can't carry it.  Drop it?") {
            *command = 'r';
        } else {
            item_id = -1;
        }
    }

    if item_id >= 0 {
        if *command == 'r' {
            crate::inventory::inventory_drop_item(item_id as usize, true);
            // As a safety measure, set the player's inven
            // weight to 0, when the last object is dropped.
            if py().pack.unique_items == 0 && py().equipment_count == 0 {
                py().pack.weight = 0;
            }
        } else {
            let mut item_copy = py().inventory[item_id as usize];
            let carry_id = crate::inventory::inventory_carry_item(&mut item_copy);
            py().inventory[item_id as usize] = item_copy;
            crate::player::player_take_off(item_id, carry_id);
        }

        crate::player::player_strength();
        game().player_free_turn = false;

        if *command == 'r' {
            selecting = false;
        }
    }

    selecting
}

// Wearing. Go to a bit of trouble over replacing existing equipment.
fn execute_wear_item_command(item_id: i32, which: char, prompt: &str) {
    let mut item_id = item_id;
    let mut slot: i32 = 0;

    if which.is_ascii_uppercase() && !verify_action(prompt, item_id as usize) {
        item_id = -1;
    } else {
        slot = inventory_get_slot_to_wear_equipment(py().inventory[item_id as usize].category_id);
        if slot == -1 {
            item_id = -1;
        }
    }

    if item_id >= 0 && py().inventory[slot as usize].category_id != TV_NOTHING {
        if crate::inventory::inventory_item_is_cursed(&py().inventory[slot as usize]) {
            inventory_item_is_cursed_message(slot as usize);
            item_id = -1;
        } else if py().inventory[item_id as usize].sub_category_id == ITEM_GROUP_MIN
            && py().inventory[item_id as usize].items_count > 1
            && !crate::inventory::inventory_can_carry_item_count(&py().inventory[slot as usize])
        {
            // this can happen if try to wield a torch,
            // and have more than one in inventory
            print_message(Some("You will have to drop something first."));
            item_id = -1;
        }
    }

    if item_id == -1 {
        return;
    }

    // OK. Wear it.
    game().player_free_turn = false;

    //
    // 1. remove new item from inventory
    //
    let mut saved_item: Inventory = py().inventory[item_id as usize];

    game().screen.wear_high_id -= 1;

    // Fix for torches
    if saved_item.items_count > 1 && saved_item.sub_category_id <= ITEM_SINGLE_STACK_MAX {
        saved_item.items_count = 1;
        game().screen.wear_high_id += 1;
    }

    py().pack.weight += saved_item.weight as i16 * saved_item.items_count as i16;
    crate::inventory::inventory_destroy_item(item_id as usize); // Subtracts weight

    //
    // 2. add old item to inv and remove from equipment list, if necessary.
    //
    let slot = slot as usize;
    if py().inventory[slot].category_id != TV_NOTHING {
        let uniq_items = py().pack.unique_items;

        let mut item_copy = py().inventory[slot];
        let id = crate::inventory::inventory_carry_item(&mut item_copy);
        py().inventory[slot] = item_copy;

        // If item removed did not stack with anything
        // in inventory, then increment wear_high.
        if py().pack.unique_items != uniq_items {
            game().screen.wear_high_id += 1;
        }

        crate::player::player_take_off(slot as i32, id);
    }

    //
    // 3. wear new item
    //
    py().inventory[slot] = saved_item;
    py().equipment_count += 1;

    crate::player::player_adjust_bonuses_for_item(py().inventory[slot], 1);

    let text = if slot == WIELD {
        "You are wielding"
    } else if slot == LIGHT {
        "Your light source is"
    } else {
        "You are wearing"
    };

    let description = item_description(&py().inventory[slot], true);

    // Get the right equipment letter.
    let mut item_id_to_take_off = WIELD;
    let mut letter_id = 0;

    while item_id_to_take_off != slot {
        if py().inventory[item_id_to_take_off].category_id != TV_NOTHING {
            letter_id += 1;
        }
        item_id_to_take_off += 1;
    }

    let msg = format!("{} {} ({})", text, description, (b'a' + letter_id as u8) as char);
    print_message(Some(&msg));

    // this is a new weapon, so clear heavy flag
    if slot == WIELD {
        py().weapon_is_heavy = false;
    }
    crate::player::player_strength();

    if crate::inventory::inventory_item_is_cursed(&py().inventory[slot]) {
        print_message(Some("Oops! It feels deathly cold!"));
        crate::identification::item_append_to_inscription(&mut py().inventory[slot], config::identification::ID_DAMD);

        // To force a cost of 0, even if unidentified.
        py().inventory[slot].cost = -1;
    }
}

fn execute_drop_item_command(item_id: i32, which: char, prompt: &str) {
    let mut item_id = item_id;
    let mut confirmed: i32 = -1;

    if py().inventory[item_id as usize].items_count > 1 {
        let mut description = item_description(&py().inventory[item_id as usize], true);
        description.pop();
        description.push('?'); // replace period with question

        let msg = format!("Drop all {}", description);

        // request command from player
        confirmed = get_input_confirmation_with_abort(0, &msg);
        // if aborted
        if confirmed == -1 {
            item_id = -1;
        }
    } else if which.is_ascii_uppercase() && !verify_action(prompt, item_id as usize) {
        item_id = -1;
    }

    if item_id >= 0 {
        game().player_free_turn = false;

        crate::inventory::inventory_drop_item(item_id as usize, confirmed == 1);
        crate::player::player_strength();
    }

    // As a safety measure, set the player's inven weight
    // to 0, when the last object is dropped.
    if py().pack.unique_items == 0 && py().equipment_count == 0 {
        py().pack.weight = 0;
    }
}

fn select_item_commands(command: &mut char, which: &mut char, selecting: bool) -> bool {
    let mut selecting = selecting;

    while selecting && game().player_free_turn {
        let mut swap = "";
        let from_line;
        let to_line;
        let prompt;

        if *command == 'w' {
            from_line = game().screen.wear_low_id;
            to_line = game().screen.wear_high_id;
            prompt = "Wear/Wield";
        } else {
            from_line = 0;
            if *command == 'd' {
                to_line = py().pack.unique_items as i32 - 1;
                prompt = "Drop";

                if py().equipment_count > 0 {
                    swap = ", / for Equip";
                }
            } else {
                to_line = py().equipment_count as i32 - 1;

                if *command == 't' {
                    prompt = "Take off";
                } else {
                    // command == 'r'

                    prompt = "Throw off";
                    if py().pack.unique_items > 0 {
                        swap = ", / for Inven";
                    }
                }
            }
        }

        if from_line > to_line {
            selecting = false;
            continue;
        }

        let heading_msg = build_command_heading(from_line, to_line, swap, *command, prompt);

        // Abort everything.
        if !get_command(&heading_msg, which) {
            *which = ESCAPE;
            selecting = false;
            continue;
        }

        // Draw the screen and maybe exit to main prompt.
        if *which == ' ' || *which == '*' {
            change_screen_for_command(*command);
            if *which == ' ' {
                selecting = false;
            }
            continue;
        }

        // Swap screens (for drop)
        if *which == '/' && !swap.is_empty() {
            if *command == 'd' {
                *command = 'r';
            } else {
                *command = 'd';
            }
            flip_inventory_equipment_screens();
            continue;
        }

        // look for item whose inscription matches "which"
        let item_id = inventory_get_item_matching_inscription(*which, *command, from_line, to_line);
        if item_id < from_line || item_id > to_line {
            terminal_bell_sound();
            continue;
        }

        //
        // Found an item - do something with it!
        //
        if *command == 'r' || *command == 't' {
            // Get its place in the equipment list.
            selecting = execute_remove_item_command(selecting, item_id, command, *which, prompt);
        } else if *command == 'w' {
            execute_wear_item_command(item_id, *which, prompt);
        } else {
            // command == 'd'
            execute_drop_item_command(item_id, *which, prompt);
            selecting = false;
        }

        if !game().player_free_turn && game().screen.current_screen_id == Screen::Blank {
            selecting = false;
        }
    }

    selecting
}

// Put an appropriate header message (on message line).
fn inventory_display_appropriate_header() {
    if game().screen.current_screen_id == Screen::Inventory {
        let weight_quotient = py().pack.weight / 10;
        let weight_remainder = py().pack.weight % 10;

        let msg;
        if !config::options::options().show_inventory_weights || py().pack.unique_items == 0 {
            msg = format!(
                "You are carrying {}.{} pounds. In your pack there is {}",
                weight_quotient,
                weight_remainder,
                if py().pack.unique_items == 0 { "nothing." } else { "-" }
            );
        } else {
            let capacity = crate::player::player_carrying_load_limit();
            let capacity_quotient = capacity / 10;
            let capacity_remainder = capacity % 10;

            msg = format!(
                "You are carrying {}.{} pounds. Your capacity is {}.{} pounds. In your pack is -",
                weight_quotient, weight_remainder, capacity_quotient, capacity_remainder
            );
        }

        put_string_clear_to_eol(&msg, Coord::new(0, 0));
    } else if game().screen.current_screen_id == Screen::Wear {
        if game().screen.wear_high_id < game().screen.wear_low_id {
            put_string_clear_to_eol("You have nothing you could wield.", Coord::new(0, 0));
        } else {
            put_string_clear_to_eol("You could wield -", Coord::new(0, 0));
        }
    } else if game().screen.current_screen_id == Screen::Equipment {
        if py().equipment_count == 0 {
            put_string_clear_to_eol("You are not using anything.", Coord::new(0, 0));
        } else {
            put_string_clear_to_eol("You are using -", Coord::new(0, 0));
        }
    } else {
        put_string_clear_to_eol("Allowed commands:", Coord::new(0, 0));
    }

    erase_line(Coord::new(game().screen.screen_bottom_pos, game().screen.screen_left_pos));
}

fn ui_command_display_inventory() {
    if py().pack.unique_items == 0 {
        print_message(Some("You are not carrying anything."));
    } else {
        ui_command_switch_screen(Screen::Inventory);
    }
}

fn ui_command_display_equipment() {
    if py().equipment_count == 0 {
        print_message(Some("You are not using any equipment."));
    } else {
        ui_command_switch_screen(Screen::Equipment);
    }
}

// This does all the work.
pub fn inventory_execute_command(command: char) {
    let mut command = command;

    game().player_free_turn = true;

    terminal_save_screen();

    let recover_screen = command == ' ';
    request_and_show_inventory_screen(recover_screen);

    loop {
        if command.is_ascii_uppercase() {
            command = command.to_ascii_lowercase();
        }

        // Simple command getting and screen selection.
        let mut selecting = false;
        match command {
            'i' => ui_command_display_inventory(),
            'e' => ui_command_display_equipment(),
            't' => selecting = ui_command_inventory_take_off_item(selecting),
            'd' => selecting = ui_command_inventory_drop_item(&mut command, selecting),
            'w' => selecting = ui_command_inventory_wear_wield_item(selecting),
            'x' => ui_command_inventory_unwield_item(),
            '?' => ui_command_switch_screen(Screen::Help),
            ' ' => {
                // Dummy command to return again to main prompt.
            }
            _ => {
                // Nonsense command
                terminal_bell_sound();
            }
        }

        // Clear the game.doing_inventory_command flag here, instead of at beginning, so that
        // can use it to control when messages above appear.
        game().doing_inventory_command = '\0';

        // Keep looking for objects to drop/wear/take off/throw off
        let mut which = 'z';

        selecting = select_item_commands(&mut command, &mut which, selecting);

        if which == ESCAPE || game().screen.current_screen_id == Screen::Blank {
            command = ESCAPE;
        } else if !game().player_free_turn {
            // Save state for recovery if they want to call us again next turn.
            // Otherwise, set a dummy command to recover screen.
            if selecting {
                game().doing_inventory_command = command;
            } else {
                game().doing_inventory_command = ' ';
            }

            // flush last message before clearing screen_has_changed and exiting
            print_message(None);

            // This lets us know if the world changes
            *screen_has_changed() = false;

            command = ESCAPE;
        } else {
            inventory_display_appropriate_header();

            put_string("e/i/t/w/x/d/?/ESC:", Coord::new(game().screen.screen_bottom_pos, 60));
            command = get_key_input();

            erase_line(Coord::new(game().screen.screen_bottom_pos, game().screen.screen_left_pos));
        }

        if command == ESCAPE {
            break;
        }
    }

    if game().screen.current_screen_id != Screen::Blank {
        terminal_restore_screen();
    }

    crate::player::player_recalculate_bonuses();
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum PackMenu {
    CloseMenu,
    Equipment,
    Inventory,
}

// Switch between Equipment/Inventory menu.
// Returns true when menu has changed
fn inventory_switch_pack_menu(prompt: &str, menu: &mut PackMenu, menu_active: bool, item_id_end: &mut i32) -> bool {
    if *menu == PackMenu::Inventory {
        let mut changed = false;

        if py().equipment_count == 0 {
            put_string_clear_to_eol("But you're not using anything -more-", Coord::new(0, 0));
            get_key_input();
        } else {
            *menu = PackMenu::Equipment;
            changed = true;

            if menu_active {
                *item_id_end = py().equipment_count as i32;

                while *item_id_end < py().pack.unique_items as i32 {
                    *item_id_end += 1;
                    erase_line(Coord::new(*item_id_end, 0));
                }
            }
            *item_id_end = py().equipment_count as i32 - 1;
        }

        put_string_clear_to_eol(prompt, Coord::new(0, 0));
        return changed;
    }

    if py().pack.unique_items == 0 {
        put_string_clear_to_eol("But you're not carrying anything -more-", Coord::new(0, 0));
        get_key_input();
        return false;
    }

    *menu = PackMenu::Inventory;
    if menu_active {
        *item_id_end = py().pack.unique_items as i32;

        while *item_id_end < py().equipment_count as i32 {
            *item_id_end += 1;
            erase_line(Coord::new(*item_id_end, 0));
        }
    }
    *item_id_end = py().pack.unique_items as i32 - 1;
    true
}

// Get the ID of an item and return the CTR value of it -RAK-
pub fn inventory_get_input_for_item_id(
    command_key_id: &mut i32,
    prompt: &str,
    item_id_start: i32,
    item_id_end: i32,
    mask: Option<&[bool]>,
    message: Option<&str>,
) -> bool {
    let mut item_id_start = item_id_start;
    let mut item_id_end = item_id_end;

    let mut menu = PackMenu::Inventory;
    let mut pack_full = false;

    if item_id_end > WIELD as i32 {
        pack_full = true;

        if py().pack.unique_items == 0 {
            menu = PackMenu::Equipment;
            item_id_end = py().equipment_count as i32 - 1;
        } else {
            item_id_end = py().pack.unique_items as i32 - 1;
        }
    }

    if py().pack.unique_items < 1 && (!pack_full || py().equipment_count < 1) {
        put_string_clear_to_eol("You are not carrying anything.", Coord::new(0, 0));
        return false;
    }

    *command_key_id = 0;

    let mut item_found = false;
    let mut menu_active = false;

    loop {
        if menu_active {
            if menu == PackMenu::Inventory {
                display_inventory_items(item_id_start, item_id_end, false, 80, mask);
            } else {
                display_equipment(false, 80);
            }
        }

        let description = if pack_full {
            format!(
                "({}: {}-{},{}{} / for {}, or ESC) {}",
                if menu == PackMenu::Inventory { "Inven" } else { "Equip" },
                (b'a' + item_id_start as u8) as char,
                (b'a' + item_id_end as u8) as char,
                if menu == PackMenu::Inventory { " 0-9," } else { "" },
                if menu_active { "" } else { " * to see," },
                if menu == PackMenu::Inventory { "Equip" } else { "Inven" },
                prompt
            )
        } else {
            format!(
                "(Items {}-{},{}{} ESC to exit) {}",
                (b'a' + item_id_start as u8) as char,
                (b'a' + item_id_end as u8) as char,
                if menu == PackMenu::Inventory { " 0-9," } else { "" },
                if menu_active { "" } else { " * for inventory list," },
                prompt
            )
        };

        put_string_clear_to_eol(&description, Coord::new(0, 0));

        let mut done = false;
        while !done {
            let which = get_key_input();

            match which {
                ESCAPE => {
                    menu = PackMenu::CloseMenu;
                    done = true;
                    game().player_free_turn = true;
                }
                '/' => {
                    done = inventory_switch_pack_menu(&description, &mut menu, menu_active, &mut item_id_end);
                }
                '*' => {
                    // activate menu if required
                    if !menu_active {
                        done = true;
                        terminal_save_screen();
                        menu_active = true;
                    }
                }
                _ => {
                    // look for item whose inscription matches "which"
                    if ('0'..='9').contains(&which) && menu != PackMenu::Equipment {
                        // Note: loop to find the inventory item
                        let mut m = item_id_start;
                        while (m as usize) < WIELD
                            && (py().inventory[m as usize].inscription[0] != which as u8 || py().inventory[m as usize].inscription[1] != 0)
                        {
                            m += 1;
                        }

                        if (m as usize) < WIELD {
                            *command_key_id = m;
                        } else {
                            *command_key_id = -1;
                        }
                    } else if which.is_ascii_uppercase() {
                        *command_key_id = which as i32 - 'A' as i32;
                    } else {
                        *command_key_id = which as i32 - 'a' as i32;
                    }

                    let mask_ok = match mask {
                        None => true,
                        Some(mask) => *command_key_id >= 0 && (*command_key_id as usize) < mask.len() && mask[*command_key_id as usize],
                    };

                    if *command_key_id >= item_id_start && *command_key_id <= item_id_end && mask_ok {
                        if menu == PackMenu::Equipment {
                            item_id_start = 21;
                            item_id_end = *command_key_id;

                            loop {
                                // Note: a simple loop to find first inventory item
                                item_id_start += 1;
                                while py().inventory[item_id_start as usize].category_id == TV_NOTHING {
                                    item_id_start += 1;
                                }

                                item_id_end -= 1;
                                if item_id_end < 0 {
                                    break;
                                }
                            }

                            *command_key_id = item_id_start;
                        }

                        if which.is_ascii_uppercase() && !verify_action("Try", *command_key_id as usize) {
                            menu = PackMenu::CloseMenu;
                            done = true;

                            game().player_free_turn = true;
                        } else {
                            menu = PackMenu::CloseMenu;
                            done = true;

                            item_found = true;
                        }
                    } else if let Some(message) = message {
                        print_message(Some(message));

                        // Set `done` to force redraw of the question.
                        done = true;
                    } else {
                        terminal_bell_sound();
                    }
                }
            }
        }

        if menu == PackMenu::CloseMenu {
            break;
        }
    }

    if menu_active {
        terminal_restore_screen();
    }

    message_line_clear();

    item_found
}
