// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Code executed when player dies

use crate::config;
use crate::data_player::CLASSES;
use crate::dungeon::dg;
use crate::game::game;
use crate::game_files::{display_death_file, output_player_character_to_file};
use crate::helpers::human_date_string;
use crate::identification::{item_set_as_identified, spell_item_identify_and_remove_random_inscription};
use crate::inventory::PLAYER_INVENTORY_SIZE;
use crate::player::{player_is_male, player_rank_title, player_recalculate_bonuses, py, PLAYER_MAX_LEVEL};
use crate::scores::{record_new_high_score, show_scores_screen};
use crate::spells::spell_restore_player_levels;
use crate::types::Coord;
use crate::ui::{print_character, ESCAPE};
use crate::ui_inventory::{display_equipment, display_inventory_items};
use crate::ui_io::{
    center_for_static_screen, clear_screen, clear_to_bottom, erase_line, flush_input_buffer, get_key_input,
    get_string_input, print_message, put_string, wait_for_continue_key,
};
use crate::locale::format_number;
use crate::{tr, tr_fmt};

// Prints the gravestone of the character -RAK-
fn death_tomb() {
    display_death_file(&config::files::localized(config::files::DEATH_TOMB));

    let text = py().misc.name.clone();
    put_string(&text, Coord::new(6, 26 - text.chars().count() as i32 / 2));

    let text = if !game().total_winner { player_rank_title() } else { tr!("Magnificent").to_string() };
    put_string(&text, Coord::new(8, 26 - text.chars().count() as i32 / 2));

    let text = if !game().total_winner {
        tr!(CLASSES[py().misc.class_id as usize].title).to_string()
    } else if player_is_male() {
        tr!("*King*").to_string()
    } else {
        tr!("*Queen*").to_string()
    };
    put_string(&text, Coord::new(10, 26 - text.chars().count() as i32 / 2));

    let text = py().misc.level.to_string();
    put_string(&text, Coord::new(11, 30));

    let text = tr_fmt!("{} Exp", format_number(py().misc.exp as i64));
    put_string(&text, Coord::new(12, 26 - text.chars().count() as i32 / 2));

    let text = tr_fmt!("{} Au", format_number(py().misc.au as i64));
    put_string(&text, Coord::new(13, 26 - text.chars().count() as i32 / 2));

    let text = dg().current_level.to_string();
    put_string(&text, Coord::new(14, 34));

    let text = game().character_died_from.clone();
    put_string(&text, Coord::new(16, 26 - text.chars().count() as i32 / 2));

    let text = human_date_string();
    put_string(&text, Coord::new(17, 26 - text.chars().count() as i32 / 2));

    loop {
        flush_input_buffer();

        put_string(tr!("(ESC to abort, return to print on screen, or file name)"), Coord::new(23, 0));
        put_string(tr!("Character record?"), Coord::new(22, 0));

        let mut str_input = String::new();
        if !get_string_input(&mut str_input, Coord::new(22, 18), 60) {
            break;
        }

        for i in 0..PLAYER_INVENTORY_SIZE {
            let category_id = py().inventory[i].category_id;
            let sub_category_id = py().inventory[i].sub_category_id;
            item_set_as_identified(category_id, sub_category_id);
            spell_item_identify_and_remove_random_inscription(&mut py().inventory[i]);
        }

        player_recalculate_bonuses();

        if !str_input.is_empty() {
            if !output_player_character_to_file(&str_input) {
                continue;
            }
        } else {
            clear_screen();
            print_character();
            put_string(tr!("Type ESC to skip the inventory:"), Coord::new(23, 0));
            if get_key_input() != ESCAPE {
                clear_screen();
                print_message(Some(tr!("You are using:")));
                display_equipment(true, 0);
                print_message(None);
                print_message(Some(tr!("You are carrying:")));
                clear_to_bottom(1);
                display_inventory_items(0, py().pack.unique_items as i32 - 1, true, 0, None);
                print_message(None);
            }
        }

        break;
    }
}

// Let the player know they did good.
fn death_royal() {
    display_death_file(&config::files::localized(config::files::DEATH_ROYAL));

    if player_is_male() {
        put_string(tr!("King!"), Coord::new(17, 45));
    } else {
        put_string(tr!("Queen!"), Coord::new(17, 45));
    }

    flush_input_buffer();
    wait_for_continue_key(23);
}

// Change the player into a King! -RAK-
fn kingly() {
    // Change the character attributes.
    dg().current_level = 0;
    game().character_died_from = tr!("Ripe Old Age").to_string();

    let _ = spell_restore_player_levels();

    py().misc.level += PLAYER_MAX_LEVEL as u16;
    py().misc.au += 250000;
    py().misc.max_exp += 5000000;
    py().misc.exp = py().misc.max_exp;

    death_royal();
}

// What happens upon dying -RAK-
// Handles the gravestone and top-twenty routines -RAK-
pub fn end_game() -> ! {
    // jdbkmoria extension: the tomb and high-score screens that follow are
    // classic static screens; end_game() never returns (the process exits
    // or restarts below), so there's no prior mode to restore.
    center_for_static_screen();

    print_message(None);

    // flush all input
    flush_input_buffer();

    // If the game has been saved, then save sets turn back to -1,
    // which inhibits the printing of the tomb.
    if dg().game_turn >= 0 {
        if game().total_winner {
            kingly();
        }
        death_tomb();
    }

    // Save the memory at least.
    if game().character_generated && !game().character_saved {
        let _ = crate::game_save::save_game();
    }

    // add score to score file if applicable
    if game().character_generated {
        // Clear `game.character_saved`, strange thing to do, but it prevents
        // get_key_input() from recursively calling end_game() when there has
        // been an eof on stdin detected.
        game().character_saved = false;
        record_new_high_score();
        show_scores_screen();
    }
    erase_line(Coord::new(23, 0));

    crate::game::exit_program();
}
