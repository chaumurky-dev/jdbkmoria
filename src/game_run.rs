// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Run the game: the main loop

use crate::character::character_create;
use crate::config;
use crate::config::player::status;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::{CLASSES, CLASS_BASE_PROVISIONS, MAGIC_SPELLS};
use crate::data_treasure::GAME_OBJECTS;
use crate::dungeon::{dg, dungeon_display_map, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::dungeon_generate::generate_cave;
use crate::dungeon_los::look;
use crate::game::{
    game, get_direction_with_memory, random_number, seeds_initialize, set_game_options,
    sorted_objects, treasure_levels, MAX_DUNGEON_OBJECTS, TREASURE_MAX_LEVELS,
};
use crate::game_death::end_game;
use crate::game_files::{display_splash_screen, display_text_help_file, output_random_level_objects_to_file};
use crate::game_save::{load_game, save_game};
use crate::helpers::{get_and_clear_first_bit, get_current_unix_time};
use crate::identification::{
    identify_game_object, item_append_to_inscription, item_identify_as_store_bought, item_inscribe,
    item_type_remaining_count_description, magic_initialize_item_names, spell_item_identified,
};
use crate::inventory::{
    inventory_carry_item, inventory_destroy_item, inventory_find_range, inventory_item_copy_to,
    inventory_item_is_cursed, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use crate::mage_spells::get_and_cast_magic_spell;
use crate::monster::{
    monster_levels, monster_multiply_total, monsters, next_free_monster_id, update_monsters,
    MON_MAX_CREATURES, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS,
};
use crate::monster_manager::{compact_monsters, monster_place_new_within_distance};
use crate::player::{
    py, player_calculate_allowed_spells_count, player_change_speed, player_close_door,
    player_disturb, player_gain_mana, player_gain_spells, player_no_light,
    player_open_closed_object, player_recalculate_bonuses, player_rest_off, player_rest_on,
    player_search, player_search_off, player_search_on, player_strength, player_takes_hit,
    player_teleport, A_INT, A_WIS,
};
use crate::player_bash::player_bash;
use crate::player_eat::player_eat;
use crate::player::player_move_position;
use crate::player_move::player_move;
use crate::player_pray::pray;
use crate::player_quaff::quaff;
use crate::player_run::{player_end_running, player_find_initialize, player_run_and_find};
use crate::player_stats::{player_initialize_base_experience_levels, player_stat_adjustment_constitution};
use crate::player_throw::player_throw_item;
use crate::player_traps::player_disarm_trap;
use crate::player_tunnel::player_tunnel;
use crate::scores::show_scores_screen;
use crate::scrolls::scroll_read;
use crate::spells::{spell_identify_item, spell_map_current_area, spell_mass_genocide};
use crate::staves::{staff_use, wand_aim};
use crate::store::store_initialize_owners;
use crate::store_inventory::store_maintenance;
use crate::treasure::{
    TV_CLOSED_DOOR, TV_DOWN_STAIR, TV_FLASK, TV_MAGIC_BOOK, TV_MAX_ENCHANT, TV_MIN_ENCHANT,
    TV_NEVER, TV_NOTHING, TV_OPEN_DOOR, TV_PRAYER_BOOK, TV_SPIKE, TV_SWORD, TV_UP_STAIR,
};
use crate::types::Coord;
use crate::ui::{
    change_character_name, coord_outside_panel, ctrl_key, display_character_stats,
    display_spells_list, draw_dungeon_panel, dungeon_reset_view, print_character_blind_status,
    print_character_confused_state, print_character_current_armor_class,
    print_character_current_depth, print_character_current_hit_points, print_character_current_mana,
    print_character_fear_state, print_character_hunger_status, print_character_max_hit_points,
    print_character_movement_state, print_character_poisoned_state, print_character_speed,
    print_character_stats_block, print_character_study_instruction, print_character_winner,
    DELETE, ESCAPE, MESSAGE_HISTORY_SIZE,
};
use crate::ui_inventory::{
    inventory_execute_command, inventory_get_input_for_item_id, player_item_wearing_description,
};
use crate::ui_io::{
    check_for_non_blocking_key_press, clear_screen, eof_flag, erase_line, flush_input_buffer,
    get_command, get_input_confirmation, get_key_input, keypad_direction, last_message_id, message_line_clear,
    message_ready_to_print, messages, panel_move_cursor, print_message,
    print_message_no_command_interrupt, put_qio, put_string, put_string_clear_to_eol,
    terminal_bell_sound, terminal_restore_screen, terminal_save_screen, wait_for_continue_key,
};
use crate::wizard::{
    enter_wizard_mode, wizard_character_adjustment, wizard_create_objects, wizard_cure_all,
    wizard_drop_random_items, wizard_gain_experience, wizard_generate_object, wizard_jump_level,
    wizard_light_up_dungeon, wizard_summon_monster,
};

// Control-key command values, usable as `match` patterns.
const CTRL_A: char = ctrl_key('A');
const CTRL_B: char = ctrl_key('B');
const CTRL_D: char = ctrl_key('D');
const CTRL_E: char = ctrl_key('E');
const CTRL_F: char = ctrl_key('F');
const CTRL_G: char = ctrl_key('G');
const CTRL_H: char = ctrl_key('H');
const CTRL_I: char = ctrl_key('I');
const CTRL_J: char = ctrl_key('J');
const CTRL_K: char = ctrl_key('K');
const CTRL_L: char = ctrl_key('L');
const CTRL_M: char = ctrl_key('M');
const CTRL_N: char = ctrl_key('N');
const CTRL_O: char = ctrl_key('O');
const CTRL_P: char = ctrl_key('P');
const CTRL_Q: char = ctrl_key('Q');
const CTRL_S: char = ctrl_key('S');
const CTRL_T: char = ctrl_key('T');
const CTRL_U: char = ctrl_key('U');
const CTRL_V: char = ctrl_key('V');
const CTRL_W: char = ctrl_key('W');
const CTRL_X: char = ctrl_key('X');
const CTRL_Y: char = ctrl_key('Y');

const LIGHT: usize = PlayerEquipment::Light as usize;

pub fn start_moria(seed: u32, start_new_game: bool, roguelike_keys: bool) {
    // Start the game with Roguelike keys (disabled by default)
    // NOTE: this will be overridden by the game save file.
    config::options::options().use_roguelike_keys = roguelike_keys;

    price_adjust();

    // Show the game splash screen
    display_splash_screen();

    // Grab a random seed from the clock
    seeds_initialize(seed);

    // Init monster and treasure levels for allocate
    initialize_monster_levels();
    initialize_treasure_levels();

    // Init the store inventories
    store_initialize_owners();

    // NOTE: base exp levels need initializing before loading a game
    player_initialize_base_experience_levels();

    // initialize some player fields - may or may not be needed -MRC-
    py().flags.spells_learnt = 0;
    py().flags.spells_worked = 0;
    py().flags.spells_forgotten = 0;

    // If -n is not passed, the calling routine will know
    // save file name, hence, this code is not necessary.

    // This restoration of a saved character may get ONLY the monster memory. In
    // this case, `loadGame()` returns false. It may also resurrect a dead character
    // (if you are the wizard). In this case, it returns true, but also sets the
    // parameter "generate" to true, as it does not recover any cave details.

    let mut result = false;
    let mut generate = false;

    if !start_new_game
        && std::path::Path::new(&config::files::save_game()).exists()
        && load_game(&mut generate)
    {
        result = true;
    }

    // enter wizard mode before showing the character display, but must wait
    // until after loadGame() in case it was just a resurrection
    if game().to_be_wizard && !enter_wizard_mode() {
        end_game();
    }

    if result {
        change_character_name();

        // could be restoring a dead character after a signal or HANGUP
        if py().misc.current_hp < 0 {
            game().character_is_dead = true;
        }
    } else {
        // Create character
        character_create();

        py().misc.date_of_birth = get_current_unix_time() as i32;

        initialize_character_inventory();
        py().flags.food = 7500;
        py().flags.food_digested = 2;

        // Spell and Mana based on class: Mage or Clerical realm.
        let class_id = py().misc.class_id as usize;
        if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
            clear_screen(); // makes spell list easier to read
            player_calculate_allowed_spells_count(A_INT);
            player_gain_mana(A_INT);
        } else if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_PRIEST {
            player_calculate_allowed_spells_count(A_WIS);
            clear_screen(); // force out the 'learn prayer' message
            player_gain_mana(A_WIS);
        }

        // Set some default values -MRC-
        py().temporary_light_only = false;
        py().weapon_is_heavy = false;
        py().pack.heaviness = 0;

        // prevent ^c quit from entering score into scoreboard,
        // and prevent signal from creating panic save until this
        // point, all info needed for save file is now valid.
        game().character_generated = true;
        generate = true;
    }

    magic_initialize_item_names();

    //
    // Begin the game
    //
    clear_screen();
    put_string("Press ? for help", Coord::new(0, 63));
    print_character_stats_block();

    if generate {
        generate_cave();
    }

    // Loop till dead, or exit
    while !game().character_is_dead {
        // Dungeon logic
        play_dungeon();

        // check for eof here, see getKeyInput() in io.c
        // eof can occur if the process gets a HANGUP signal
        if *eof_flag() != 0 {
            game().character_died_from = "(end of input: saved)".to_string();
            if !save_game() {
                game().character_died_from = "unexpected eof".to_string();
            }

            // should not reach here, but if we do, this guarantees exit
            game().character_is_dead = true;
        }

        // New level if not dead
        if !game().character_is_dead {
            generate_cave();
        }
    }

    // Character gets buried.
    end_game();
}

// Init players with some belongings -RAK-
fn initialize_character_inventory() {
    // this is needed for bash to work right, it can't hurt anyway
    for i in 0..PLAYER_INVENTORY_SIZE {
        inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut py().inventory[i]);
    }

    let class_id = py().misc.class_id as usize;
    for &item_id in CLASS_BASE_PROVISIONS[class_id].iter() {
        let mut item = Inventory::empty();
        inventory_item_copy_to(item_id as usize, &mut item);

        // this makes it spellItemIdentifyAndRemoveRandomInscription and itemSetAsIdentified
        item_identify_as_store_bought(&mut item);

        // must set this bit to display to_hit/to_damage for stiletto
        if item.category_id == TV_SWORD {
            item.identification |= config::identification::ID_SHOW_HIT_DAM;
        }

        inventory_carry_item(&mut item);
    }

    // weird place for it, but why not?
    for id in py().flags.spells_learned_order.iter_mut() {
        *id = 99;
    }
}

// Initializes M_LEVEL array for use with PLACE_MONSTER -RAK-
// pub so integration tests can generate a dungeon without a terminal
pub fn initialize_monster_levels() {
    for level in monster_levels().iter_mut() {
        *level = 0;
    }

    for i in 0..(MON_MAX_CREATURES - config::monsters::MON_ENDGAME_MONSTERS as usize) {
        monster_levels()[CREATURES_LIST[i].level as usize] += 1;
    }

    for i in 1..=MON_MAX_LEVELS {
        let prev = monster_levels()[i - 1];
        monster_levels()[i] += prev;
    }
}

// Initializes T_LEVEL array for use with PLACE_OBJECT -RAK-
// pub so integration tests can generate a dungeon without a terminal
pub fn initialize_treasure_levels() {
    for level in treasure_levels().iter_mut() {
        *level = 0;
    }

    for i in 0..MAX_DUNGEON_OBJECTS {
        treasure_levels()[GAME_OBJECTS[i].depth_first_found as usize] += 1;
    }

    for i in 1..=TREASURE_MAX_LEVELS {
        let prev = treasure_levels()[i - 1];
        treasure_levels()[i] += prev;
    }

    // now produce an array with object indexes sorted by level,
    // by using the info in treasure_levels, this is an O(n) sort!
    // this is not a stable sort, but that does not matter
    let mut indexes = [1i32; TREASURE_MAX_LEVELS + 1];

    for i in 0..MAX_DUNGEON_OBJECTS {
        let level = GAME_OBJECTS[i].depth_first_found as usize;
        let object_id = treasure_levels()[level] as i32 - indexes[level];

        sorted_objects()[object_id as usize] = i as i16;

        indexes[level] += 1;
    }
}

// Adjust prices of objects -RAK-
fn price_adjust() {
    // No-op: COST_ADJUSTMENT is always 100 in this port, so the price
    // adjustment loop compiles to nothing (dead code).
}

// Moria game module -RAK-
// The code in this section has gone through many revisions, and
// some of it could stand some more hard work. -RAK-

// It has had a bit more hard work. -CJS-

// Reset flags and initialize variables
fn reset_dungeon_flags() {
    game().command_count = 0;
    dg().generate_new_level = false;
    py().running_tracker = 0;
    game().teleport_player = false;
    *monster_multiply_total() = 0;
    dg().tile_mut(py().pos).creature_id = 1;
}

// Check light status for dungeon setup
fn player_initialize_player_light() {
    py().carrying_light = py().inventory[LIGHT].misc_use > 0;
}

// Check for a maximum level
fn player_update_max_dungeon_depth() {
    if dg().current_level > py().misc.max_dungeon_depth as i16 {
        py().misc.max_dungeon_depth = dg().current_level as u16;
    }
}

// Check light status
fn player_update_light_status() {
    if py().carrying_light {
        if py().inventory[LIGHT].misc_use > 0 {
            py().inventory[LIGHT].misc_use -= 1;

            if py().inventory[LIGHT].misc_use == 0 {
                py().carrying_light = false;
                print_message(Some("Your light has gone out!"));
                player_disturb(0, 1);

                // unlight creatures
                update_monsters(false);
            } else if py().inventory[LIGHT].misc_use < 40 && random_number(5) == 1 && py().flags.blind < 1 {
                player_disturb(0, 0);
                print_message(Some("Your light is growing faint."));
            }
        } else {
            py().carrying_light = false;
            player_disturb(0, 1);

            // unlight creatures
            update_monsters(false);
        }
    } else if py().inventory[LIGHT].misc_use > 0 {
        py().inventory[LIGHT].misc_use -= 1;
        py().carrying_light = true;
        player_disturb(0, 1);

        // light creatures
        update_monsters(false);
    }
}

fn player_activate_heroism() {
    py().flags.status |= status::PY_HERO;
    player_disturb(0, 0);

    py().misc.max_hp += 10;
    py().misc.current_hp += 10;
    py().misc.bth += 12;
    py().misc.bth_with_bows += 12;

    print_message(Some("You feel like a HERO!"));
    print_character_max_hit_points();
    print_character_current_hit_points();
}

fn player_disable_heroism() {
    py().flags.status &= !status::PY_HERO;
    player_disturb(0, 0);

    py().misc.max_hp -= 10;
    if py().misc.current_hp > py().misc.max_hp {
        py().misc.current_hp = py().misc.max_hp;
        py().misc.current_hp_fraction = 0;
        print_character_current_hit_points();
    }
    py().misc.bth -= 12;
    py().misc.bth_with_bows -= 12;

    print_message(Some("The heroism wears off."));
    print_character_max_hit_points();
}

fn player_activate_super_heroism() {
    py().flags.status |= status::PY_SHERO;
    player_disturb(0, 0);

    py().misc.max_hp += 20;
    py().misc.current_hp += 20;
    py().misc.bth += 24;
    py().misc.bth_with_bows += 24;

    print_message(Some("You feel like a SUPER HERO!"));
    print_character_max_hit_points();
    print_character_current_hit_points();
}

fn player_disable_super_heroism() {
    py().flags.status &= !status::PY_SHERO;
    player_disturb(0, 0);

    py().misc.max_hp -= 20;
    if py().misc.current_hp > py().misc.max_hp {
        py().misc.current_hp = py().misc.max_hp;
        py().misc.current_hp_fraction = 0;
        print_character_current_hit_points();
    }
    py().misc.bth -= 24;
    py().misc.bth_with_bows -= 24;

    print_message(Some("The super heroism wears off."));
    print_character_max_hit_points();
}

fn player_update_hero_status() {
    // Heroism
    if py().flags.heroism > 0 {
        if (py().flags.status & status::PY_HERO) == 0 {
            player_activate_heroism();
        }

        py().flags.heroism -= 1;

        if py().flags.heroism == 0 {
            player_disable_heroism();
        }
    }

    // Super Heroism
    if py().flags.super_heroism > 0 {
        if (py().flags.status & status::PY_SHERO) == 0 {
            player_activate_super_heroism();
        }

        py().flags.super_heroism -= 1;

        if py().flags.super_heroism == 0 {
            player_disable_super_heroism();
        }
    }
}

fn player_food_consumption() -> i32 {
    // Regenerate hp and mana
    let mut regen_amount = config::player::PLAYER_REGEN_NORMAL as i32;

    if (py().flags.food as i32) < config::player::PLAYER_FOOD_ALERT as i32 {
        if (py().flags.food as i32) < config::player::PLAYER_FOOD_WEAK as i32 {
            if py().flags.food < 0 {
                regen_amount = 0;
            } else if (py().flags.food as i32) < config::player::PLAYER_FOOD_FAINT as i32 {
                regen_amount = config::player::PLAYER_REGEN_FAINT as i32;
            } else if (py().flags.food as i32) < config::player::PLAYER_FOOD_WEAK as i32 {
                regen_amount = config::player::PLAYER_REGEN_WEAK as i32;
            }

            if (py().flags.status & status::PY_WEAK) == 0 {
                py().flags.status |= status::PY_WEAK;
                print_message(Some("You are getting weak from hunger."));
                player_disturb(0, 0);
                print_character_hunger_status();
            }

            if (py().flags.food as i32) < config::player::PLAYER_FOOD_FAINT as i32 && random_number(8) == 1 {
                py().flags.paralysis += random_number(5) as i16;
                print_message(Some("You faint from the lack of food."));
                player_disturb(1, 0);
            }
        } else if (py().flags.status & status::PY_HUNGRY) == 0 {
            py().flags.status |= status::PY_HUNGRY;
            print_message(Some("You are getting hungry."));
            player_disturb(0, 0);
            print_character_hunger_status();
        }
    }

    // Food consumption
    // Note: Sped up characters really burn up the food!
    if py().flags.speed < 0 {
        py().flags.food -= py().flags.speed * py().flags.speed;
    }

    py().flags.food -= py().flags.food_digested;

    if py().flags.food < 0 {
        player_takes_hit(-(py().flags.food as i32) / 16, "starvation"); // -CJS-
        player_disturb(1, 0);
    }

    regen_amount
}

fn player_update_regeneration(mut amount: i32) {
    if py().flags.regenerate_hp {
        amount = amount * 3 / 2;
    }

    if (py().flags.status & status::PY_SEARCH) != 0 || py().flags.rest != 0 {
        amount *= 2;
    }

    if py().flags.poisoned < 1 && py().misc.current_hp < py().misc.max_hp {
        player_regenerate_hit_points(amount);
    }

    if py().misc.current_mana < py().misc.mana {
        player_regenerate_mana(amount);
    }
}

fn player_update_blindness() {
    if py().flags.blind <= 0 {
        return;
    }

    if (py().flags.status & status::PY_BLIND) == 0 {
        py().flags.status |= status::PY_BLIND;

        draw_dungeon_panel();
        print_character_blind_status();
        player_disturb(0, 1);

        // unlight creatures
        update_monsters(false);
    }

    py().flags.blind -= 1;

    if py().flags.blind == 0 {
        py().flags.status &= !status::PY_BLIND;

        print_character_blind_status();
        draw_dungeon_panel();
        player_disturb(0, 1);

        // light creatures
        update_monsters(false);

        print_message(Some("The veil of darkness lifts."));
    }
}

fn player_update_confusion() {
    if py().flags.confused <= 0 {
        return;
    }

    if (py().flags.status & status::PY_CONFUSED) == 0 {
        py().flags.status |= status::PY_CONFUSED;
        print_character_confused_state();
    }

    py().flags.confused -= 1;

    if py().flags.confused == 0 {
        py().flags.status &= !status::PY_CONFUSED;

        print_character_confused_state();
        print_message(Some("You feel less confused now."));

        if py().flags.rest != 0 {
            player_rest_off();
        }
    }
}

fn player_update_fear_state() {
    if py().flags.afraid <= 0 {
        return;
    }

    if (py().flags.status & status::PY_FEAR) == 0 {
        if py().flags.super_heroism + py().flags.heroism > 0 {
            py().flags.afraid = 0;
        } else {
            py().flags.status |= status::PY_FEAR;
            print_character_fear_state();
        }
    } else if py().flags.super_heroism + py().flags.heroism > 0 {
        py().flags.afraid = 1;
    }

    py().flags.afraid -= 1;

    if py().flags.afraid == 0 {
        py().flags.status &= !status::PY_FEAR;

        print_character_fear_state();
        print_message(Some("You feel bolder now."));
        player_disturb(0, 0);
    }
}

fn player_update_poisoned_state() {
    if py().flags.poisoned <= 0 {
        return;
    }

    if (py().flags.status & status::PY_POISONED) == 0 {
        py().flags.status |= status::PY_POISONED;
        print_character_poisoned_state();
    }

    py().flags.poisoned -= 1;

    if py().flags.poisoned == 0 {
        py().flags.status &= !status::PY_POISONED;

        print_character_poisoned_state();
        print_message(Some("You feel better."));
        player_disturb(0, 0);

        return;
    }

    let damage = match player_stat_adjustment_constitution() {
        -4 => 4,
        -3 | -2 => 3,
        -1 => 2,
        0 => 1,
        1..=3 => {
            if (dg().game_turn % 2) == 0 {
                1
            } else {
                0
            }
        }
        4 | 5 => {
            if (dg().game_turn % 3) == 0 {
                1
            } else {
                0
            }
        }
        6 => {
            if (dg().game_turn % 4) == 0 {
                1
            } else {
                0
            }
        }
        _ => 0,
    };

    player_takes_hit(damage, "poison");
    player_disturb(1, 0);
}

fn player_update_fastness() {
    if py().flags.fast <= 0 {
        return;
    }

    if (py().flags.status & status::PY_FAST) == 0 {
        py().flags.status |= status::PY_FAST;
        player_change_speed(-1);

        print_message(Some("You feel yourself moving faster."));
        player_disturb(0, 0);
    }

    py().flags.fast -= 1;

    if py().flags.fast == 0 {
        py().flags.status &= !status::PY_FAST;
        player_change_speed(1);

        print_message(Some("You feel yourself slow down."));
        player_disturb(0, 0);
    }
}

fn player_update_slowness() {
    if py().flags.slow <= 0 {
        return;
    }

    if (py().flags.status & status::PY_SLOW) == 0 {
        py().flags.status |= status::PY_SLOW;
        player_change_speed(1);

        print_message(Some("You feel yourself moving slower."));
        player_disturb(0, 0);
    }

    py().flags.slow -= 1;

    if py().flags.slow == 0 {
        py().flags.status &= !status::PY_SLOW;
        player_change_speed(-1);

        print_message(Some("You feel yourself speed up."));
        player_disturb(0, 0);
    }
}

fn player_update_speed() {
    player_update_fastness();
    player_update_slowness();
}

// Resting is over?
fn player_update_resting_state() {
    if py().flags.rest > 0 {
        py().flags.rest -= 1;

        // Resting over
        if py().flags.rest == 0 {
            player_rest_off();
        }
    } else if py().flags.rest < 0 {
        // Rest until reach max mana and max hit points.
        py().flags.rest += 1;

        if (py().misc.current_hp == py().misc.max_hp && py().misc.current_mana == py().misc.mana) || py().flags.rest == 0 {
            player_rest_off();
        }
    }
}

// Hallucinating?   (Random characters appear!)
fn player_update_hallucination() {
    if py().flags.image <= 0 {
        return;
    }

    player_end_running();

    py().flags.image -= 1;

    if py().flags.image == 0 {
        // Used to draw entire screen! -CJS-
        draw_dungeon_panel();
    }
}

fn player_update_paralysis() {
    if py().flags.paralysis <= 0 {
        return;
    }

    // when paralysis true, you can not see any movement that occurs
    py().flags.paralysis -= 1;

    player_disturb(1, 0);
}

// Protection from evil counter
fn player_update_evil_protection() {
    if py().flags.protect_evil <= 0 {
        return;
    }

    py().flags.protect_evil -= 1;

    if py().flags.protect_evil == 0 {
        print_message(Some("You no longer feel safe from evil."));
    }
}

fn player_update_invulnerability() {
    if py().flags.invulnerability <= 0 {
        return;
    }

    if (py().flags.status & status::PY_INVULN) == 0 {
        py().flags.status |= status::PY_INVULN;
        player_disturb(0, 0);

        py().misc.ac += 100;
        py().misc.display_ac += 100;

        print_character_current_armor_class();
        print_message(Some("Your skin turns into steel!"));
    }

    py().flags.invulnerability -= 1;

    if py().flags.invulnerability == 0 {
        py().flags.status &= !status::PY_INVULN;
        player_disturb(0, 0);

        py().misc.ac -= 100;
        py().misc.display_ac -= 100;

        print_character_current_armor_class();
        print_message(Some("Your skin returns to normal."));
    }
}

fn player_update_blessedness() {
    if py().flags.blessed <= 0 {
        return;
    }

    if (py().flags.status & status::PY_BLESSED) == 0 {
        py().flags.status |= status::PY_BLESSED;
        player_disturb(0, 0);

        py().misc.bth += 5;
        py().misc.bth_with_bows += 5;
        py().misc.ac += 2;
        py().misc.display_ac += 2;

        print_message(Some("You feel righteous!"));
        print_character_current_armor_class();
    }

    py().flags.blessed -= 1;

    if py().flags.blessed == 0 {
        py().flags.status &= !status::PY_BLESSED;
        player_disturb(0, 0);

        py().misc.bth -= 5;
        py().misc.bth_with_bows -= 5;
        py().misc.ac -= 2;
        py().misc.display_ac -= 2;

        print_message(Some("The prayer has expired."));
        print_character_current_armor_class();
    }
}

// Resist Heat
fn player_update_heat_resistance() {
    if py().flags.heat_resistance <= 0 {
        return;
    }

    py().flags.heat_resistance -= 1;

    if py().flags.heat_resistance == 0 {
        print_message(Some("You no longer feel safe from flame."));
    }
}

fn player_update_cold_resistance() {
    if py().flags.cold_resistance <= 0 {
        return;
    }

    py().flags.cold_resistance -= 1;

    if py().flags.cold_resistance == 0 {
        print_message(Some("You no longer feel safe from cold."));
    }
}

fn player_update_detect_invisible() {
    if py().flags.detect_invisible <= 0 {
        return;
    }

    if (py().flags.status & status::PY_DET_INV) == 0 {
        py().flags.status |= status::PY_DET_INV;
        py().flags.see_invisible = true;

        // light but don't move creatures
        update_monsters(false);
    }

    py().flags.detect_invisible -= 1;

    if py().flags.detect_invisible == 0 {
        py().flags.status &= !status::PY_DET_INV;

        // may still be able to see_invisible if wearing magic item
        player_recalculate_bonuses();

        // unlight but don't move creatures
        update_monsters(false);
    }
}

// Timed infra-vision
fn player_update_infra_vision() {
    if py().flags.timed_infra <= 0 {
        return;
    }

    if (py().flags.status & status::PY_TIM_INFRA) == 0 {
        py().flags.status |= status::PY_TIM_INFRA;
        py().flags.see_infra += 1;

        // light but don't move creatures
        update_monsters(false);
    }

    py().flags.timed_infra -= 1;

    if py().flags.timed_infra == 0 {
        py().flags.status &= !status::PY_TIM_INFRA;
        py().flags.see_infra -= 1;

        // unlight but don't move creatures
        update_monsters(false);
    }
}

// Word-of-Recall  Note: Word-of-Recall is a delayed action
fn player_update_word_of_recall() {
    if py().flags.word_of_recall <= 0 {
        return;
    }

    if py().flags.word_of_recall == 1 {
        dg().generate_new_level = true;

        py().flags.paralysis += 1;
        py().flags.word_of_recall = 0;

        if dg().current_level > 0 {
            dg().current_level = 0;
            print_message(Some("You feel yourself yanked upwards!"));
        } else if py().misc.max_dungeon_depth != 0 {
            dg().current_level = py().misc.max_dungeon_depth as i16;
            print_message(Some("You feel yourself yanked downwards!"));
        }
    } else {
        py().flags.word_of_recall -= 1;
    }
}

fn player_update_status_flags() {
    if (py().flags.status & status::PY_SPEED) != 0 {
        py().flags.status &= !status::PY_SPEED;
        print_character_speed();
    }

    if (py().flags.status & status::PY_PARALYSED) != 0 && py().flags.paralysis < 1 {
        print_character_movement_state();
        py().flags.status &= !status::PY_PARALYSED;
    } else if py().flags.paralysis > 0 {
        print_character_movement_state();
        py().flags.status |= status::PY_PARALYSED;
    } else if py().flags.rest != 0 {
        print_character_movement_state();
    }

    if (py().flags.status & status::PY_ARMOR) != 0 {
        print_character_current_armor_class();
        py().flags.status &= !status::PY_ARMOR;
    }

    if (py().flags.status & status::PY_STATS) != 0 {
        for n in 0..6 {
            if ((status::PY_STR << n) & py().flags.status) != 0 {
                display_character_stats(n as usize);
            }
        }

        py().flags.status &= !status::PY_STATS;
    }

    if (py().flags.status & status::PY_HP) != 0 {
        print_character_max_hit_points();
        print_character_current_hit_points();
        py().flags.status &= !status::PY_HP;
    }

    if (py().flags.status & status::PY_MANA) != 0 {
        print_character_current_mana();
        py().flags.status &= !status::PY_MANA;
    }
}

// Allow for a slim chance of detect enchantment -CJS-
fn player_detect_enchantment() {
    let mut i = 0usize;
    while i < PLAYER_INVENTORY_SIZE {
        if i == py().pack.unique_items as usize {
            i = 22;
        }

        // if in inventory, succeed 1 out of 50 times,
        // if in equipment list, success 1 out of 10 times
        let chance = if i < 22 { 50 } else { 10 };

        if py().inventory[i].category_id != TV_NOTHING && item_enchanted(&py().inventory[i]) && random_number(chance) == 1 {
            let description = player_item_wearing_description(i);
            let tmp_str = format!("There's something about what you are {}...", description);
            player_disturb(0, 0);
            print_message(Some(&tmp_str));
            item_append_to_inscription(&mut py().inventory[i], config::identification::ID_MAGIK);
        }

        i += 1;
    }
}

fn get_command_repeat_count(last_input_command: &mut char) -> i32 {
    put_string_clear_to_eol("Repeat count:", Coord::new(0, 0));

    if *last_input_command == '#' {
        *last_input_command = '0';
    }

    let mut repeat_count: i32 = 0;

    loop {
        if *last_input_command == DELETE || *last_input_command == CTRL_H {
            repeat_count /= 10;
            let text_buffer = format!("{:07}", repeat_count);
            put_string_clear_to_eol(&text_buffer, Coord::new(0, 14));
        } else if *last_input_command >= '0' && *last_input_command <= '9' {
            if repeat_count > 99 {
                terminal_bell_sound();
            } else {
                repeat_count = repeat_count * 10 + (*last_input_command as i32 - '0' as i32);
                let text_buffer = format!("{:07}", repeat_count);
                put_string_clear_to_eol(&text_buffer, Coord::new(0, 14));
            }
        } else {
            break;
        }
        *last_input_command = get_key_input();
    }

    if repeat_count == 0 {
        repeat_count = 99;
        let text_buffer = format!("{}", repeat_count);
        put_string_clear_to_eol(&text_buffer, Coord::new(0, 14));
    }

    // a special hack to allow numbers as commands
    if *last_input_command == ' ' {
        put_string_clear_to_eol("Command:", Coord::new(0, 20));
        *last_input_command = get_key_input();
    }

    repeat_count
}

fn parse_alternate_ctrl_input(mut last_input_command: char) -> char {
    if game().command_count > 0 {
        print_character_movement_state();
    }

    if get_command("Control-", &mut last_input_command) {
        if last_input_command >= 'A' && last_input_command <= 'Z' {
            last_input_command = ((last_input_command as u8) - (b'A' - 1)) as char;
        } else if last_input_command >= 'a' && last_input_command <= 'z' {
            last_input_command = ((last_input_command as u8) - (b'a' - 1)) as char;
        } else {
            last_input_command = ' ';
            print_message(Some("Type ^ <letter> for a control char"));
        }
    } else {
        last_input_command = ' ';
    }

    last_input_command
}

// Accept a command and execute it
fn execute_input_commands(command: &mut char, find_count: &mut i32) {
    let mut last_input_command = *command;

    // Accept a command and execute it
    loop {
        if (py().flags.status & status::PY_REPEAT) != 0 {
            print_character_movement_state();
        }

        game().use_last_direction = false;
        game().player_free_turn = false;

        if py().running_tracker != 0 {
            player_run_and_find();
            *find_count -= 1;

            if *find_count == 0 {
                player_end_running();
            }

            put_qio();
        } else if game().doing_inventory_command != '\0' {
            inventory_execute_command(game().doing_inventory_command);
        } else {
            // move the cursor to the players character
            panel_move_cursor(py().pos);

            *message_ready_to_print() = false;

            if game().command_count > 0 {
                game().use_last_direction = true;
            } else {
                last_input_command = get_key_input();

                // Get a count for a command.
                let mut repeat_count = 0;
                if (config::options::options().use_roguelike_keys && last_input_command >= '0' && last_input_command <= '9')
                    || (!config::options::options().use_roguelike_keys && last_input_command == '#')
                {
                    repeat_count = get_command_repeat_count(&mut last_input_command);
                }

                // Another way of typing control codes -CJS-
                if last_input_command == '^' {
                    last_input_command = parse_alternate_ctrl_input(last_input_command);
                }

                // move cursor to player char again, in case it moved
                panel_move_cursor(py().pos);

                // Commands are always converted to rogue form. -CJS-
                // Arrow/keypad keys (see get_key_input()) map straight onto the
                // rogue-form movement commands: a plain key walks, a shifted key runs.
                if let Some((direction, shifted)) = keypad_direction(last_input_command) {
                    last_input_command = keypad_movement_command(direction, shifted);
                } else if !config::options::options().use_roguelike_keys {
                    last_input_command = original_commands(last_input_command);
                }

                if repeat_count > 0 {
                    if !valid_count_command(last_input_command) {
                        game().player_free_turn = true;
                        last_input_command = ' ';
                        print_message(Some("Invalid command with a count."));
                    } else {
                        game().command_count = repeat_count as u32;
                        print_character_movement_state();
                    }
                }
            }

            // Flash the message line.
            message_line_clear();
            panel_move_cursor(py().pos);
            put_qio();

            do_command(last_input_command);

            // Find is counted differently, as the command changes.
            if py().running_tracker != 0 {
                *find_count = game().command_count as i32 - 1;
                game().command_count = 0;
            } else if game().player_free_turn {
                game().command_count = 0;
            } else if game().command_count != 0 {
                game().command_count -= 1;
            }
        }

        // A teleport painting can trigger during a free-turn command (look);
        // return to the main loop so the teleport happens immediately.
        if game().teleport_player {
            break;
        }

        if !(game().player_free_turn && !dg().generate_new_level && *eof_flag() == 0) {
            break;
        }
    }

    *command = last_input_command;
}

fn original_commands(mut command: char) -> char {
    let mut direction = 0i32;

    match command {
        CTRL_K => command = 'Q', // ^K = exit
        CTRL_J | CTRL_M => command = '+',
        // ^P = repeat, ^W = password, ^X = save, ^V = view license
        CTRL_P | CTRL_W | CTRL_X | CTRL_V | ' ' | '!' | '$' => {}
        '.' => {
            if get_direction_with_memory(None, &mut direction) {
                command = match direction {
                    1 => 'B',
                    2 => 'J',
                    3 => 'N',
                    4 => 'H',
                    6 => 'L',
                    7 => 'Y',
                    8 => 'K',
                    9 => 'U',
                    _ => ' ',
                };
            } else {
                command = ' ';
            }
        }
        '/' | '<' | '>' | '-' | '=' | '{' | '?' | 'A' => {}
        '1' => command = 'b',
        '2' => command = 'j',
        '3' => command = 'n',
        '4' => command = 'h',
        '5' => command = '.', // Rest one turn
        '6' => command = 'l',
        '7' => command = 'y',
        '8' => command = 'k',
        '9' => command = 'u',
        'B' => command = 'f',
        'C' | 'D' | 'E' | 'F' | 'G' => {}
        'L' => command = 'W',
        'M' | 'R' => {}
        'S' => command = '#',
        'T' => {
            if get_direction_with_memory(None, &mut direction) {
                command = match direction {
                    1 => CTRL_B,
                    2 => CTRL_J,
                    3 => CTRL_N,
                    4 => CTRL_H,
                    6 => CTRL_L,
                    7 => CTRL_Y,
                    8 => CTRL_K,
                    9 => CTRL_U,
                    _ => ' ',
                };
            } else {
                command = ' ';
            }
        }
        'V' => {}
        'a' => command = 'z',
        'b' => command = 'P',
        'c' | 'd' | 'e' => {}
        'f' => command = 't',
        'h' => command = '?',
        'i' => {}
        'j' => command = 'S',
        'l' => command = 'x',
        'm' | 'o' | 'p' | 'q' | 'r' | 's' => {}
        't' => command = 'T',
        'u' => command = 'Z',
        'v' | 'w' => {}
        'x' => command = 'X',
        // jdbkmoria extension: (g)rope at a painting -- same letter in both keysets
        'g' => {}

        // wizard mode commands follow
        CTRL_A => {}                // ^A = cure all
        CTRL_B => command = CTRL_O, // ^B = objects
        CTRL_D => {}                // ^D = up/down
        CTRL_H => command = '\\',   // ^H = wizhelp
        CTRL_I => {}                // ^I = identify
        CTRL_L => command = '*',    // ^L = wizlight
        ':' | CTRL_T | CTRL_E | CTRL_F | CTRL_G | '@' | '+' => {}
        CTRL_U => command = '&', // ^U = summon
        _ => command = '~',      // Anything illegal.
    }

    command
}

// The rogue-form movement command for a keypad direction: walking is the
// lowercase movement letter ('.' = rest for the keypad center), running
// the uppercase one.
fn keypad_movement_command(direction: i32, run: bool) -> char {
    let walk = match direction {
        1 => 'b',
        2 => 'j',
        3 => 'n',
        4 => 'h',
        6 => 'l',
        7 => 'y',
        8 => 'k',
        9 => 'u',
        _ => '.',
    };

    if run {
        walk.to_ascii_uppercase()
    } else {
        walk
    }
}

fn move_without_pickup(command: &mut char) -> bool {
    let mut cmd = *command;

    // hack for move without pickup.  Map '-' to a movement command.
    if cmd != '-' {
        return true;
    }

    let mut direction = 0i32;

    // Save current game.command_count as getDirectionWithMemory() may change it
    let count_save = game().command_count;

    if get_direction_with_memory(None, &mut direction) {
        // Restore game.command_count
        game().command_count = count_save;

        cmd = match direction {
            1 => 'b',
            2 => 'j',
            3 => 'n',
            4 => 'h',
            6 => 'l',
            7 => 'y',
            8 => 'k',
            9 => 'u',
            _ => '~',
        };
    } else {
        cmd = ' ';
    }

    *command = cmd;

    false
}

fn command_quit() {
    flush_input_buffer();

    if get_input_confirmation("Do you really want to quit?") {
        game().character_is_dead = true;
        dg().generate_new_level = true;

        game().character_died_from = "Quitting".to_string();
    }
}

fn calculate_max_message_count() -> u8 {
    let mut max_messages = MESSAGE_HISTORY_SIZE as u8;

    if game().command_count > 0 {
        if (game().command_count as usize) < MESSAGE_HISTORY_SIZE {
            max_messages = game().command_count as u8;
        }
        game().command_count = 0;
    } else if game().last_command != CTRL_P {
        max_messages = 1;
    }

    max_messages
}

fn command_previous_message() {
    let mut max_messages = calculate_max_message_count();

    if max_messages <= 1 {
        // Distinguish real and recovered messages with a '>'. -CJS-
        put_string(">", Coord::new(0, 0));
        put_string_clear_to_eol(&messages()[*last_message_id() as usize], Coord::new(1, 0));
        return;
    }

    terminal_save_screen();

    let line_number = max_messages;
    let mut msg_id = *last_message_id();

    while max_messages > 0 {
        max_messages -= 1;

        put_string_clear_to_eol(&messages()[msg_id as usize], Coord::new(max_messages as i32, 0));

        if msg_id == 0 {
            msg_id = MESSAGE_HISTORY_SIZE as i16 - 1;
        } else {
            msg_id -= 1;
        }
    }

    erase_line(Coord::new(line_number as i32, 0));
    wait_for_continue_key(line_number as i32);
    terminal_restore_screen();
}

fn command_flip_wizard_mode() {
    if game().wizard_mode {
        game().wizard_mode = false;
        print_message(Some("Wizard mode off."));
    } else if enter_wizard_mode() {
        print_message(Some("Wizard mode on."));
    }

    print_character_winner();
}

fn command_save_and_exit() {
    if game().total_winner {
        print_message(Some("You are a Total Winner,  your character must be retired."));

        if config::options::options().use_roguelike_keys {
            print_message(Some("Use 'Q' to when you are ready to quit."));
        } else {
            print_message(Some("Use <Control>-K when you are ready to quit."));
        }
    } else {
        game().character_died_from = "(saved)".to_string();
        print_message(Some("Saving game..."));

        if save_game() {
            end_game();
        }

        game().character_died_from = "(alive and well)".to_string();
    }
}

fn command_locate_on_map() {
    if py().flags.blind > 0 || player_no_light() {
        print_message(Some("You can't see your map."));
        return;
    }

    let mut player_coord = py().pos;
    if coord_outside_panel(player_coord, true) {
        draw_dungeon_panel();
    }

    let old_panel = Coord::new(dg().panel.row, dg().panel.col);

    loop {
        let panel = Coord::new(dg().panel.row, dg().panel.col);

        let tmp_str = if panel.y == old_panel.y && panel.x == old_panel.x {
            String::new()
        } else {
            let north_south = if panel.y < old_panel.y {
                " North"
            } else if panel.y > old_panel.y {
                " South"
            } else {
                ""
            };
            let west_east = if panel.x < old_panel.x {
                " West"
            } else if panel.x > old_panel.x {
                " East"
            } else {
                ""
            };
            format!("{}{} of", north_south, west_east)
        };

        let out_val = format!(
            "Map sector [{},{}], which is{} your sector. Look which direction?",
            panel.y, panel.x, tmp_str
        );

        let mut dir_val = 0i32;
        if !get_direction_with_memory(Some(&out_val), &mut dir_val) {
            break;
        }

        // -CJS-
        // Should really use the move function, but what the hell. This
        // is nicer, as it moves exactly to the same place in another
        // section. The direction calculation is not intuitive. Sorry.
        loop {
            player_coord.x += ((dir_val - 1) % 3 - 1) * SCREEN_WIDTH / 2;
            player_coord.y -= ((dir_val - 1) / 3 - 1) * SCREEN_HEIGHT / 2;

            if player_coord.x < 0 || player_coord.y < 0 || player_coord.x >= dg().width as i32 || player_coord.y >= dg().width as i32 {
                print_message(Some("You've gone past the end of your map."));

                player_coord.x -= ((dir_val - 1) % 3 - 1) * SCREEN_WIDTH / 2;
                player_coord.y += ((dir_val - 1) / 3 - 1) * SCREEN_HEIGHT / 2;

                break;
            }

            if coord_outside_panel(player_coord, true) {
                draw_dungeon_panel();
                break;
            }
        }
    }

    // Move to a new panel - but only if really necessary.
    if coord_outside_panel(py().pos, false) {
        draw_dungeon_panel();
    }
}

fn command_toggle_search() {
    if (py().flags.status & status::PY_SEARCH) != 0 {
        player_search_off();
    } else {
        player_search_on();
    }
}

fn do_wizard_commands(command: char) {
    match command {
        CTRL_A => {
            // Cure all!
            wizard_cure_all();
        }
        CTRL_E => {
            // Edit Character
            wizard_character_adjustment();
            message_line_clear();
        }
        CTRL_F => {
            // Mass Genocide, vanquish all monsters
            spell_mass_genocide();
        }
        CTRL_G => {
            // Generate random items
            wizard_drop_random_items();
        }
        CTRL_D => {
            // Go up/down to specified depth
            wizard_jump_level();
        }
        CTRL_O => {
            // Print random level object to a file
            output_random_level_objects_to_file();
        }
        '\\' => {
            // Display wizard help
            if config::options::options().use_roguelike_keys {
                display_text_help_file(config::files::HELP_ROGUELIKE_WIZARD);
            } else {
                display_text_help_file(config::files::HELP_WIZARD);
            }
        }
        CTRL_I => {
            // Identify an item
            spell_identify_item();
        }
        '*' => {
            // Light up entire dungeon
            wizard_light_up_dungeon();
        }
        ':' => {
            // Light up current panel
            spell_map_current_area();
        }
        CTRL_T => {
            // Random player teleportation
            player_teleport(100);
        }
        '%' => {
            // Generate a dungeon item!
            wizard_generate_object();
            draw_dungeon_panel();
        }
        '+' => {
            // Increase Experience
            wizard_gain_experience();
        }
        '&' => {
            // Summon a random monster
            wizard_summon_monster();
        }
        '@' => {
            // Generate an object
            // NOTE: every field from the struct needs to be filled correctly
            wizard_create_objects();
        }
        _ => {
            if config::options::options().use_roguelike_keys {
                put_string_clear_to_eol("Type '?' or '\\' for help.", Coord::new(0, 0));
            } else {
                put_string_clear_to_eol("Type '?' or ^H for help.", Coord::new(0, 0));
            }
        }
    }
}

fn do_command(command: char) {
    let mut command = command;
    let do_pickup = move_without_pickup(&mut command);

    match command {
        'Q' => {
            // (Q)uit    (^K)ill
            command_quit();
            game().player_free_turn = true;
        }
        CTRL_P => {
            // (^P)revious message.
            command_previous_message();
            game().player_free_turn = true;
        }
        CTRL_V => {
            // (^V)iew license
            display_text_help_file(config::files::LICENSE);
            game().player_free_turn = true;
        }
        CTRL_W => {
            // (^W)izard mode
            command_flip_wizard_mode();
            game().player_free_turn = true;
        }
        CTRL_X => {
            // e(^X)it and save
            command_save_and_exit();
            game().player_free_turn = true;
        }
        '=' => {
            // (=) set options
            terminal_save_screen();
            set_game_options();
            terminal_restore_screen();
            game().player_free_turn = true;
        }
        '{' => {
            // ({) inscribe an object
            item_inscribe();
            game().player_free_turn = true;
        }
        // (!) escape to the shell / ($) escaping to shell disabled -MRC- /
        // (ESC) do nothing / (space) do nothing.
        '!' | '$' | ESCAPE | ' ' => {
            game().player_free_turn = true;
        }
        'b' => player_move(1, do_pickup), // (b) down, left  (1)
        'j' => player_move(2, do_pickup), // (j) down    (2)
        'n' => player_move(3, do_pickup), // (n) down, right  (3)
        'h' => player_move(4, do_pickup), // (h) left    (4)
        'l' => player_move(6, do_pickup), // (l) right    (6)
        'y' => player_move(7, do_pickup), // (y) up, left    (7)
        'k' => player_move(8, do_pickup), // (k) up    (8)
        'u' => player_move(9, do_pickup), // (u) up, right  (9)
        'B' => player_find_initialize(1), // (B) run down, left  (. 1)
        'J' => player_find_initialize(2), // (J) run down    (. 2)
        'N' => player_find_initialize(3), // (N) run down, right  (. 3)
        'H' => player_find_initialize(4), // (H) run left    (. 4)
        'L' => player_find_initialize(6), // (L) run right  (. 6)
        'Y' => player_find_initialize(7), // (Y) run up, left  (. 7)
        'K' => player_find_initialize(8), // (K) run up    (. 8)
        'U' => player_find_initialize(9), // (U) run up, right  (. 9)
        '/' => {
            // (/) identify a symbol
            identify_game_object();
            game().player_free_turn = true;
        }
        '.' => {
            // (.) stay in one place (5)
            player_move(5, do_pickup);

            if game().command_count > 1 {
                game().command_count -= 1;
                player_rest_on();
            }
        }
        '<' => dungeon_go_up_level(),   // (<) go down a staircase
        '>' => dungeon_go_down_level(), // (>) go up a staircase
        '?' => {
            // (?) help with commands
            if config::options::options().use_roguelike_keys {
                display_text_help_file(config::files::HELP_ROGUELIKE);
            } else {
                display_text_help_file(config::files::HELP);
            }
            game().player_free_turn = true;
        }
        'f' => player_bash(), // (f)orce    (B)ash
        'C' => {
            // (C)haracter description
            terminal_save_screen();
            change_character_name();
            terminal_restore_screen();
            game().player_free_turn = true;
        }
        'D' => player_disarm_trap(), // (D)isarm trap
        'E' => player_eat(),         // (E)at food
        'F' => inventory_refill_lamp(), // (F)ill lamp
        'G' => player_gain_spells(), // (G)ain magic spells
        'V' => {
            // (V)iew scores
            terminal_save_screen();
            show_scores_screen();
            terminal_restore_screen();
            game().player_free_turn = true;
        }
        'W' => {
            // (W)here are we on the map  (L)ocate on map
            command_locate_on_map();
            game().player_free_turn = true;
        }
        'R' => player_rest_on(), // (R)est a while
        '#' => {
            // (#) search toggle  (S)earch toggle
            command_toggle_search();
            game().player_free_turn = true;
        }
        CTRL_B => player_tunnel(1),        // (^B) tunnel down left  (T 1)
        CTRL_M | CTRL_J => player_tunnel(2), // cr must be treated same as lf; (^J) tunnel down (T 2)
        CTRL_N => player_tunnel(3),        // (^N) tunnel down right  (T 3)
        CTRL_H => player_tunnel(4),        // (^H) tunnel left    (T 4)
        CTRL_L => player_tunnel(6),        // (^L) tunnel right    (T 6)
        CTRL_Y => player_tunnel(7),        // (^Y) tunnel up left    (T 7)
        CTRL_K => player_tunnel(8),        // (^K) tunnel up    (T 8)
        CTRL_U => player_tunnel(9),        // (^U) tunnel up right    (T 9)
        'z' => wand_aim(),                 // (z)ap a wand    (a)im a wand
        'M' => {
            dungeon_display_map();
            game().player_free_turn = true;
        }
        'P' => {
            // (P)eruse a book  (B)rowse in a book
            examine_book();
            game().player_free_turn = true;
        }
        'c' => player_close_door(),           // (c)lose an object
        'd' => inventory_execute_command('d'), // (d)rop something
        'e' => inventory_execute_command('e'), // (e)quipment list
        't' => player_throw_item(),           // (t)hrow something  (f)ire something
        'i' => inventory_execute_command('i'), // (i)nventory list
        'S' => dungeon_jam_door(),            // (S)pike a door  (j)am a door
        'x' => {
            // e(x)amine surrounds  (l)ook about
            look();
            game().player_free_turn = true;
        }
        'm' => get_and_cast_magic_spell(),  // (m)agic spells
        'o' => player_open_closed_object(), // (o)pen something
        'p' => pray(),                      // (p)ray
        'q' => quaff(),                     // (q)uaff
        'r' => scroll_read(),               // (r)ead
        's' => player_search(py().pos, py().misc.chance_in_search as i32), // (s)earch for a turn
        'T' => inventory_execute_command('t'), // (T)ake off something  (t)ake off
        'Z' => staff_use(),                 // (Z)ap a staff  (u)se a staff
        'v' => {
            // (v)ersion of game
            display_text_help_file(config::files::VERSIONS_HISTORY);
            game().player_free_turn = true;
        }
        'w' => inventory_execute_command('w'), // (w)ear or wield
        'X' => inventory_execute_command('x'), // e(X)change weapons  e(x)change
        // jdbkmoria extension: (g)rope at a painting
        'g' => crate::paintings::painting_reach_command(),
        _ => {
            // Wizard commands are free moves
            game().player_free_turn = true;

            if game().wizard_mode {
                do_wizard_commands(command);
            } else {
                put_string_clear_to_eol("Type '?' for help.", Coord::new(0, 0));
            }
        }
    }
    game().last_command = command;
}

// Check whether this command will accept a count. -CJS-
fn valid_count_command(command: char) -> bool {
    match command {
        'Q' | CTRL_W | CTRL_X | '=' | '{' | '/' | '<' | '>' | '?' | 'C' | 'E' | 'F' | 'G' | 'V'
        | '#' | 'z' | 'P' | 'c' | 'd' | 'e' | 't' | 'i' | 'x' | 'm' | 'p' | 'q' | 'r' | 'T' | 'Z'
        | 'v' | 'w' | 'W' | 'X' | CTRL_A | '\\' | CTRL_I | '*' | ':' | CTRL_T | CTRL_E | CTRL_F
        | CTRL_S | CTRL_Q => false,
        CTRL_P | ESCAPE | ' ' | '-' | 'b' | 'f' | 'j' | 'n' | 'h' | 'l' | 'y' | 'k' | 'u' | '.'
        | 'B' | 'J' | 'N' | 'H' | 'L' | 'Y' | 'K' | 'U' | 'D' | 'R' | CTRL_Y | CTRL_K | CTRL_U
        | CTRL_L | CTRL_N | CTRL_J | CTRL_B | CTRL_H | 'S' | 'o' | 's' | CTRL_D | CTRL_G | '+' => true,
        _ => false,
    }
}

// Regenerate hit points -RAK-
fn player_regenerate_hit_points(percent: i32) {
    let old_chp = py().misc.current_hp;
    let new_chp: i32 = py().misc.max_hp as i32 * percent + config::player::PLAYER_REGEN_HPBASE as i32;

    // div 65536
    py().misc.current_hp = py().misc.current_hp.wrapping_add((new_chp >> 16) as i16);

    // check for overflow
    if py().misc.current_hp < 0 && old_chp > 0 {
        py().misc.current_hp = i16::MAX;
    }

    // mod 65536
    let new_chp_fraction: i32 = (new_chp & 0xFFFF) + py().misc.current_hp_fraction as i32;

    if new_chp_fraction >= 0x10000 {
        py().misc.current_hp_fraction = (new_chp_fraction - 0x10000) as u16;
        py().misc.current_hp = py().misc.current_hp.wrapping_add(1);
    } else {
        py().misc.current_hp_fraction = new_chp_fraction as u16;
    }

    // must set frac to zero even if equal
    if py().misc.current_hp >= py().misc.max_hp {
        py().misc.current_hp = py().misc.max_hp;
        py().misc.current_hp_fraction = 0;
    }

    if old_chp != py().misc.current_hp {
        print_character_current_hit_points();
    }
}

// Regenerate mana points -RAK-
fn player_regenerate_mana(percent: i32) {
    let old_cmana = py().misc.current_mana;
    let new_mana: i32 = py().misc.mana as i32 * percent + config::player::PLAYER_REGEN_MNBASE as i32;

    // div 65536
    py().misc.current_mana = py().misc.current_mana.wrapping_add((new_mana >> 16) as i16);

    // check for overflow
    if py().misc.current_mana < 0 && old_cmana > 0 {
        py().misc.current_mana = i16::MAX;
    }

    // mod 65536
    let new_mana_fraction: i32 = (new_mana & 0xFFFF) + py().misc.current_mana_fraction as i32;

    if new_mana_fraction >= 0x10000 {
        py().misc.current_mana_fraction = (new_mana_fraction - 0x10000) as u16;
        py().misc.current_mana = py().misc.current_mana.wrapping_add(1);
    } else {
        py().misc.current_mana_fraction = new_mana_fraction as u16;
    }

    // must set frac to zero even if equal
    if py().misc.current_mana >= py().misc.mana {
        py().misc.current_mana = py().misc.mana;
        py().misc.current_mana_fraction = 0;
    }

    if old_cmana != py().misc.current_mana {
        print_character_current_mana();
    }
}

// Is an item an enchanted weapon or armor and we don't know? -CJS-
// only returns true if it is a good enchantment
fn item_enchanted(item: &Inventory) -> bool {
    if item.category_id < TV_MIN_ENCHANT || item.category_id > TV_MAX_ENCHANT || inventory_item_is_cursed(item) {
        return false;
    }

    if spell_item_identified(item) {
        return false;
    }

    if (item.identification & config::identification::ID_MAGIK) != 0 {
        return false;
    }

    if item.to_hit > 0 || item.to_damage > 0 || item.to_ac > 0 {
        return true;
    }

    if (0x4000107f_u32 & item.flags) != 0 && item.misc_use > 0 {
        return true;
    }

    (0x07ffe980_u32 & item.flags) != 0
}

// Examine a Book -RAK-
fn examine_book() {
    let mut item_pos_start = 0i32;
    let mut item_pos_end = 0i32;
    if !inventory_find_range(TV_MAGIC_BOOK as i32, TV_PRAYER_BOOK as i32, &mut item_pos_start, &mut item_pos_end) {
        print_message(Some("You are not carrying any books."));
        return;
    }

    if py().flags.blind > 0 {
        print_message(Some("You can't see to read your spell book!"));
        return;
    }

    if player_no_light() {
        print_message(Some("You have no light to read by."));
        return;
    }

    if py().flags.confused > 0 {
        print_message(Some("You are too confused."));
        return;
    }

    let mut item_id = 0i32;
    if inventory_get_input_for_item_id(&mut item_id, "Which Book?", item_pos_start, item_pos_end, None, None) {
        let mut spell_index = [0i32; 31];
        let mut can_read = true;

        let treasure_type = py().inventory[item_id as usize].category_id;
        let class_id = py().misc.class_id as usize;

        if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
            if treasure_type != TV_MAGIC_BOOK {
                can_read = false;
            }
        } else if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_PRIEST {
            if treasure_type != TV_PRAYER_BOOK {
                can_read = false;
            }
        } else {
            can_read = false;
        }

        if !can_read {
            print_message(Some("You do not understand the language."));
            return;
        }

        let mut item_flags = py().inventory[item_id as usize].flags;

        let mut spell_id = 0usize;
        while item_flags != 0 {
            let bit = get_and_clear_first_bit(&mut item_flags);

            if (MAGIC_SPELLS[class_id - 1][bit as usize].level_required as i32) < 99 {
                spell_index[spell_id] = bit;
                spell_id += 1;
            }
        }

        terminal_save_screen();
        display_spells_list(&spell_index, spell_id as i32, true, -1);
        wait_for_continue_key(0);
        terminal_restore_screen();
    }
}

// Go up one level -RAK-
fn dungeon_go_up_level() {
    let tile_id = dg().tile(py().pos).treasure_id;

    if tile_id != 0 && game().treasure.list[tile_id as usize].category_id == TV_UP_STAIR {
        dg().current_level -= 1;

        print_message(Some("You enter a maze of up staircases."));
        print_message(Some("You pass through a one-way door."));

        dg().generate_new_level = true;
    } else {
        print_message(Some("I see no up staircase here."));
        game().player_free_turn = true;
    }
}

// Go down one level -RAK-
fn dungeon_go_down_level() {
    let tile_id = dg().tile(py().pos).treasure_id;

    if tile_id != 0 && game().treasure.list[tile_id as usize].category_id == TV_DOWN_STAIR {
        dg().current_level += 1;

        print_message(Some("You enter a maze of down staircases."));
        print_message(Some("You pass through a one-way door."));

        dg().generate_new_level = true;
    } else {
        print_message(Some("I see no down staircase here."));
        game().player_free_turn = true;
    }
}

// Jam a closed door -RAK-
fn dungeon_jam_door() {
    game().player_free_turn = true;

    let mut coord = py().pos;

    let mut direction = 0i32;
    if !get_direction_with_memory(None, &mut direction) {
        return;
    }
    player_move_position(direction, &mut coord);

    let tile = *dg().tile(coord);

    if tile.treasure_id == 0 {
        print_message(Some("That isn't a door!"));
        return;
    }

    let treasure_id = tile.treasure_id as usize;
    let item_id = game().treasure.list[treasure_id].category_id;

    if item_id != TV_CLOSED_DOOR && item_id != TV_OPEN_DOOR {
        print_message(Some("That isn't a door!"));
        return;
    }

    if item_id == TV_OPEN_DOOR {
        print_message(Some("The door must be closed first."));
        return;
    }

    // If we reach here, the door is closed and we can try to jam it -MRC-

    if tile.creature_id == 0 {
        let mut item_pos_start = 0i32;
        let mut item_pos_end = 0i32;
        if inventory_find_range(TV_SPIKE as i32, TV_NEVER as i32, &mut item_pos_start, &mut item_pos_end) {
            game().player_free_turn = false;

            print_message_no_command_interrupt("You jam the door with a spike.");

            let mut misc_use = game().treasure.list[treasure_id].misc_use;
            if misc_use > 0 {
                // Make locked to stuck.
                misc_use = -misc_use;
            }

            // Successive spikes have a progressively smaller effect.
            // Series is: 0 20 30 37 43 48 52 56 60 64 67 70 ...
            misc_use -= 1 + 190 / (10 - misc_use);
            game().treasure.list[treasure_id].misc_use = misc_use;

            let item_pos_start = item_pos_start as usize;
            if py().inventory[item_pos_start].items_count > 1 {
                py().inventory[item_pos_start].items_count -= 1;
                py().pack.weight -= py().inventory[item_pos_start].weight as i16;
            } else {
                inventory_destroy_item(item_pos_start);
            }
        } else {
            print_message(Some("But you have no spikes."));
        }
    } else {
        game().player_free_turn = false;

        let name = CREATURES_LIST[monsters()[tile.creature_id as usize].creature_id as usize].name;
        let msg = format!("The {} is in your way!", name);
        print_message(Some(&msg));
    }
}

// Refill the players lamp -RAK-
fn inventory_refill_lamp() {
    game().player_free_turn = true;

    if py().inventory[LIGHT].sub_category_id != 0 {
        print_message(Some("But you are not using a lamp."));
        return;
    }

    let mut item_pos_start = 0i32;
    let mut item_pos_end = 0i32;
    if !inventory_find_range(TV_FLASK as i32, TV_NEVER as i32, &mut item_pos_start, &mut item_pos_end) {
        print_message(Some("You have no oil."));
        return;
    }

    game().player_free_turn = false;

    let added = py().inventory[item_pos_start as usize].misc_use;
    py().inventory[LIGHT].misc_use += added;

    let capacity = config::treasure::OBJECT_LAMP_MAX_CAPACITY as i16;

    if py().inventory[LIGHT].misc_use > capacity {
        py().inventory[LIGHT].misc_use = capacity;
        print_message(Some("Your lamp overflows, spilling oil on the ground."));
        print_message(Some("Your lamp is full."));
    } else if py().inventory[LIGHT].misc_use > capacity / 2 {
        print_message(Some("Your lamp is more than half full."));
    } else if py().inventory[LIGHT].misc_use == capacity / 2 {
        print_message(Some("Your lamp is half full."));
    } else {
        print_message(Some("Your lamp is less than half full."));
    }

    item_type_remaining_count_description(item_pos_start as usize);
    inventory_destroy_item(item_pos_start as usize);
}

// Main procedure for dungeon. -RAK-
fn play_dungeon() {
    // Note: There is a lot of preliminary magic going on here at first
    player_initialize_player_light();
    player_update_max_dungeon_depth();
    reset_dungeon_flags();

    // Initialize find counter to `0`
    let mut find_count = 0i32;

    // Ensure we display the panel. Used to do this with a global var. -CJS-
    dg().panel.row = -1;
    dg().panel.col = -1;

    // Light up the area around character
    dungeon_reset_view();

    // must do this after `dg.panel.row` / `dg.panel.col` set to -1, because playerSearchOff() will
    // call dungeonResetView(), and so the panel_* variables must be valid before
    // playerSearchOff() is called
    if (py().flags.status & status::PY_SEARCH) != 0 {
        player_search_off();
    }

    // Light,  but do not move critters
    update_monsters(false);

    // Print the depth
    print_character_current_depth();

    // Note: yes, this last input command needs to be persisted
    // over different iterations of the main loop below -MRC-
    let mut last_input_command: char = '\0';

    // Loop until dead,  or new level
    // Exit when `dg.generate_new_level` and `eof_flag` are both set
    loop {
        // Increment turn counter
        dg().game_turn += 1;

        // turn over the store contents every, say, 1000 turns
        if dg().current_level != 0 && dg().game_turn % 1000 == 0 {
            store_maintenance();
        }

        // Check for creature generation
        if random_number(config::monsters::MON_CHANCE_OF_NEW as i32) == 1 {
            monster_place_new_within_distance(1, config::monsters::MON_MAX_SIGHT as i32, false);
        }

        player_update_light_status();

        //
        // Update counters and messages
        //

        // Heroism and Super Heroism must precede anything that can damage player
        player_update_hero_status();

        let regen_amount = player_food_consumption();
        player_update_regeneration(regen_amount);

        player_update_blindness();
        player_update_confusion();
        player_update_fear_state();
        player_update_poisoned_state();
        player_update_speed();
        player_update_resting_state();

        // Check for interrupts to find or rest.
        let microseconds = if py().running_tracker != 0 { 0 } else { 10000 };
        if (game().command_count > 0 || py().running_tracker != 0 || py().flags.rest != 0) && check_for_non_blocking_key_press(microseconds) {
            player_disturb(0, 0);
        }

        player_update_hallucination();
        player_update_paralysis();
        player_update_evil_protection();
        player_update_invulnerability();
        player_update_blessedness();
        player_update_heat_resistance();
        player_update_cold_resistance();
        player_update_detect_invisible();
        player_update_infra_vision();
        player_update_word_of_recall();

        // Random teleportation
        if py().flags.teleport && random_number(100) == 1 {
            player_disturb(0, 0);
            player_teleport(40);
        }

        // See if we are too weak to handle the weapon or pack. -CJS-
        if (py().flags.status & status::PY_STR_WGT) != 0 {
            player_strength();
        }

        if (py().flags.status & status::PY_STUDY) != 0 {
            print_character_study_instruction();
        }

        player_update_status_flags();

        // Allow for a slim chance of detect enchantment -CJS-
        // for 1st level char, check once every 2160 turns
        // for 40th level char, check once every 416 turns
        let chance = 10 + 750 / (5 + py().misc.level as i32);
        if (dg().game_turn & 0xF) == 0 && py().flags.confused == 0 && random_number(chance) == 1 {
            player_detect_enchantment();
        }

        // Check the state of the monster list, and delete some monsters if
        // the monster list is nearly full.  This helps to avoid problems in
        // creature.c when monsters try to multiply.  Compact_monsters() is
        // much more likely to succeed if called from here, than if called
        // from within updateMonsters().
        if MON_TOTAL_ALLOCATIONS as i32 - (*next_free_monster_id() as i32) < 10 {
            compact_monsters();
        }

        // Accept a command?
        if py().flags.paralysis < 1 && py().flags.rest == 0 && !game().character_is_dead {
            execute_input_commands(&mut last_input_command, &mut find_count);
        } else {
            // if paralyzed, resting, or dead, flush output
            // but first move the cursor onto the player, for aesthetics
            panel_move_cursor(py().pos);
            put_qio();
        }

        // Teleport?
        if game().teleport_player {
            player_teleport(100);
        }

        // Move the creatures
        if !dg().generate_new_level {
            update_monsters(true);
            crate::paintings::update_paintings();
        }

        if dg().generate_new_level || *eof_flag() != 0 {
            break;
        }
    }
}
