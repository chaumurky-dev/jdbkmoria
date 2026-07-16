// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Generate a new player character

use crate::config;
use crate::data_player::{CHARACTER_BACKGROUNDS, CHARACTER_RACES, CLASSES};
use crate::game::{exit_program, random_number, random_number_normal_distribution};
use crate::game_files::display_text_help_file;
use crate::player::{py, player_is_male, player_set_gender, A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS, PLAYER_MAX_CLASSES, PLAYER_MAX_LEVEL, PLAYER_MAX_RACES};
use crate::player_stats::{
    player_armor_class_adjustment, player_damage_adjustment, player_disarm_adjustment, player_set_and_use_stat,
    player_stat_adjustment_constitution, player_to_hit_adjustment,
};
use crate::types::Coord;
use crate::ui::{
    get_character_name, print_character_abilities, print_character_information, print_character_level_experience, print_character_stats,
    print_character_vital_statistics, ESCAPE,
};
use crate::ui_io::{
    center_for_static_screen, clear_to_bottom, erase_line, get_key_input, move_cursor, put_string,
    put_string_clear_to_eol, terminal_bell_sound,
};
use crate::tr;

// Race type for the generated player character
pub struct Race {
    pub name: &'static str,     // Type of race
    pub str_adjustment: i16,    // adjustments
    pub int_adjustment: i16,
    pub wis_adjustment: i16,
    pub dex_adjustment: i16,
    pub con_adjustment: i16,
    pub chr_adjustment: i16,
    pub base_age: u8,           // Base age of character
    pub max_age: u8,            // Maximum age of character
    pub male_height_base: u8,   // base height for males
    pub male_height_mod: u8,    // mod height for males
    pub male_weight_base: u8,   // base weight for males
    pub male_weight_mod: u8,    // mod weight for males
    pub female_height_base: u8, // base height females
    pub female_height_mod: u8,  // mod height for females
    pub female_weight_base: u8, // base weight for female
    pub female_weight_mod: u8,  // mod weight for females
    pub disarm_chance_base: i16, // base chance to disarm
    pub search_chance_base: i16, // base chance for search
    pub stealth: i16,           // Stealth of character
    pub fos: i16,               // frequency of auto search
    pub base_to_hit: i16,       // adj base chance to hit
    pub base_to_hit_bows: i16,  // adj base to hit with bows
    pub saving_throw_base: i16, // Race base for saving throw
    pub hit_points_base: u8,    // Base hit points for race
    pub infra_vision: u8,       // See infra-red
    pub exp_factor_base: u8,    // Base experience factor
    pub classes_bit_field: u8,  // Bit field for class types
}

// Class type for the generated player character
pub struct Class {
    pub title: &'static str,             // type of class
    pub hit_points: u8,                  // Adjust hit points
    pub disarm_traps: u8,                // mod disarming traps
    pub searching: u8,                   // modifier to searching
    pub stealth: u8,                     // modifier to stealth
    pub fos: u8,                         // modifier to freq-of-search
    pub base_to_hit: u8,                 // modifier to base to hit
    pub base_to_hit_with_bows: u8,       // modifier to base to hit - bows
    pub saving_throw: u8,                // Class modifier to save
    pub strength: i16,                   // Class modifier for strength
    pub intelligence: i16,               // Class modifier for intelligence
    pub wisdom: i16,                     // Class modifier for wisdom
    pub dexterity: i16,                  // Class modifier for dexterity
    pub constitution: i16,               // Class modifier for constitution
    pub charisma: i16,                   // Class modifier for charisma
    pub class_to_use_mage_spells: u8,    // class use mage spells
    pub experience_factor: u8,           // Class experience factor
    pub min_level_for_spell_casting: u8, // First level where class can use spells.
}

// Class background for the generated player character
pub struct Background {
    pub info: &'static str, // History information
    pub roll: u8,           // Die roll needed for history
    pub chart: u8,          // Table number
    pub next: u8,           // Pointer to next table
    pub bonus: u8,          // Bonus to the Social Class+50
}

// Generates character's stats -JWT-
fn character_generate_stats() {
    let mut dice = [0i32; 18];
    let mut total;

    loop {
        total = 0;
        for i in 0..18 {
            // Roll 3,4,5 sided dice once each
            dice[i] = random_number(3 + i as i32 % 3);
            total += dice[i];
        }

        if total > 42 && total < 54 {
            break;
        }
    }

    for i in 0..6 {
        py().stats.max[i] = (5 + dice[3 * i] + dice[3 * i + 1] + dice[3 * i + 2]) as u8;
    }
}

fn decrement_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat as i32;

    let mut i: i32 = 0;
    while i > adjustment as i32 {
        if stat > 108 {
            stat -= 1;
        } else if stat > 88 {
            stat += -random_number(6) - 2;
        } else if stat > 18 {
            stat += -random_number(15) - 5;
            if stat < 18 {
                stat = 18;
            }
        } else if stat > 3 {
            stat -= 1;
        }
        i -= 1;
    }

    stat as u8
}

fn increment_stat(adjustment: i16, current_stat: u8) -> u8 {
    let mut stat = current_stat as i32;

    let mut i: i32 = 0;
    while i < adjustment as i32 {
        if stat < 18 {
            stat += 1;
        } else if stat < 88 {
            stat += random_number(15) + 5;
        } else if stat < 108 {
            stat += random_number(6) + 2;
        } else if stat < 118 {
            stat += 1;
        }
        i += 1;
    }

    stat as u8
}

// Changes stats by given amount -JWT-
// During character creation we adjust player stats based
// on their Race and Class...with a little randomness!
fn create_modify_player_stat(stat: u8, adjustment: i16) -> u8 {
    if adjustment < 0 {
        decrement_stat(adjustment, stat)
    } else {
        increment_stat(adjustment, stat)
    }
}

// generate all stats and modify for race. needed in a separate
// module so looping of character selection would be allowed -RGM-
fn character_generate_stats_and_race() {
    let race = &CHARACTER_RACES[py().misc.race_id as usize];

    character_generate_stats();
    py().stats.max[A_STR] = create_modify_player_stat(py().stats.max[A_STR], race.str_adjustment);
    py().stats.max[A_INT] = create_modify_player_stat(py().stats.max[A_INT], race.int_adjustment);
    py().stats.max[A_WIS] = create_modify_player_stat(py().stats.max[A_WIS], race.wis_adjustment);
    py().stats.max[A_DEX] = create_modify_player_stat(py().stats.max[A_DEX], race.dex_adjustment);
    py().stats.max[A_CON] = create_modify_player_stat(py().stats.max[A_CON], race.con_adjustment);
    py().stats.max[A_CHR] = create_modify_player_stat(py().stats.max[A_CHR], race.chr_adjustment);

    py().misc.level = 1;

    for i in 0..6 {
        py().stats.current[i] = py().stats.max[i];
        player_set_and_use_stat(i);
    }

    py().misc.chance_in_search = race.search_chance_base;
    py().misc.bth = race.base_to_hit;
    py().misc.bth_with_bows = race.base_to_hit_bows;
    py().misc.fos = race.fos;
    py().misc.stealth_factor = race.stealth;
    py().misc.saving_throw = race.saving_throw_base;
    py().misc.hit_die = race.hit_points_base;
    py().misc.plusses_to_damage = player_damage_adjustment();
    py().misc.plusses_to_hit = player_to_hit_adjustment();
    py().misc.magical_ac = 0;
    py().misc.ac = player_armor_class_adjustment();
    py().misc.experience_factor = race.exp_factor_base;
    py().flags.see_infra = race.infra_vision as i16;
}

// Prints a list of the available races: Human, Elf, etc.,
// shown during the character creation screens.
fn display_character_races() {
    clear_to_bottom(20);
    put_string(tr!("Choose a race (? for Help):"), Coord::new(20, 2));

    let mut coord = Coord::new(21, 2);

    for i in 0..PLAYER_MAX_RACES {
        let description = format!("{}) {}", (b'a' + i as u8) as char, tr!(CHARACTER_RACES[i].name));
        put_string(&description, coord);

        coord.x += 15;
        if coord.x > 70 {
            coord.x = 2;
            coord.y += 1;
        }
    }
}

// Allows player to select a race -JWT-
fn character_choose_race() {
    display_character_races();

    let mut id: i32;
    loop {
        move_cursor(Coord::new(20, 30));
        let key = get_key_input();

        id = key as i32 - 97; // ASCII `a`, setting id between 0 and 7
        if id >= 0 && (id as usize) < PLAYER_MAX_RACES {
            break;
        } else if key == '?' {
            display_text_help_file(&config::files::localized(config::files::WELCOME_SCREEN));
        } else {
            terminal_bell_sound();
        }
    }
    py().misc.race_id = id as u8;

    put_string(tr!(CHARACTER_RACES[id as usize].name), Coord::new(3, 15));
}

// Will print the history of a character -JWT-
fn display_character_history() {
    put_string(tr!("Character Background"), Coord::new(14, 27));

    for i in 0..4 {
        put_string_clear_to_eol(&py().misc.history[i], Coord::new(i as i32 + 15, 10));
    }
}

// Clear the previous history strings
fn player_clear_history() {
    for entry in py().misc.history.iter_mut() {
        entry.clear();
    }
}

// Get the racial history, determines social class -RAK-
//
// Assumptions:
//   - Each race has init history beginning at (race-1)*3+1
//   - All history parts are in ascending order
fn character_get_history() {
    let mut history_id = py().misc.race_id as i32 * 3 + 1;
    let mut social_class = random_number(4);

    let mut history_block = String::new();

    let mut background_id: usize = 0;

    // Get a block of history text
    loop {
        let mut flag = false;
        while !flag {
            if CHARACTER_BACKGROUNDS[background_id].chart as i32 == history_id {
                let test_roll = random_number(100);

                while test_roll > CHARACTER_BACKGROUNDS[background_id].roll as i32 {
                    background_id += 1;
                }

                let background = &CHARACTER_BACKGROUNDS[background_id];

                history_block.push_str(tr!(background.info));
                social_class += background.bonus as i32 - 50;

                if history_id > background.next as i32 {
                    background_id = 0;
                }

                history_id = background.next as i32;
                flag = true;
            } else {
                background_id += 1;
            }
        }

        if history_id < 1 {
            break;
        }
    }

    player_clear_history();

    // Process block of history text for pretty output.
    for (line_number, line) in wrap_history_lines(&history_block).into_iter().enumerate() {
        py().misc.history[line_number] = line;
    }

    // Compute social class for player
    social_class = social_class.clamp(1, 100);

    py().misc.social_class = social_class as i16;
}

// Word-wraps a block of history text into (at most 4) lines of at most 60
// display columns, trimming runs of spaces at line boundaries. Mirrors the
// original C cursor-walking algorithm exactly, but operates on chars rather
// than raw bytes: -RAK-
//
// jdbkmoria extension: using chars (not bytes) means a multi-byte UTF-8
// character (accented French background text) can never be sliced across a
// character boundary; for pure-ASCII text this produces byte-for-byte
// identical output to the original byte-indexed version, since every char
// is one byte. Factored out of `character_get_history()` so it can be unit
// tested without touching global player state.
pub fn wrap_history_lines(history_block: &str) -> Vec<String> {
    let chars: Vec<char> = history_block.chars().collect();

    let mut lines = Vec::new();

    let mut cursor_start: i32 = 0;
    let mut cursor_end: i32 = chars.len() as i32 - 1;
    while chars[cursor_end as usize] == ' ' {
        cursor_end -= 1;
    }

    let mut new_cursor_start = 0;

    let mut flag = false;
    while !flag {
        while chars[cursor_start as usize] == ' ' {
            cursor_start += 1;
        }

        let mut current_cursor_position = cursor_end - cursor_start + 1;

        if current_cursor_position > 60 {
            current_cursor_position = 60;

            while chars[(cursor_start + current_cursor_position - 1) as usize] != ' ' {
                current_cursor_position -= 1;
            }

            new_cursor_start = cursor_start + current_cursor_position;

            while chars[(cursor_start + current_cursor_position - 1) as usize] == ' ' {
                current_cursor_position -= 1;
            }
        } else {
            flag = true;
        }

        let start = cursor_start as usize;
        let len = current_cursor_position as usize;
        lines.push(chars[start..start + len].iter().collect());

        cursor_start = new_cursor_start;
    }

    lines
}

// Gets the character's gender -JWT-
fn character_set_gender() {
    clear_to_bottom(20);
    let prompt = tr!("Choose a sex (? for Help):");
    put_string(prompt, Coord::new(20, 2));
    put_string(tr!("m) Male       f) Female"), Coord::new(21, 2));

    // jdbkmoria extension: the answer cursor sits just past the prompt,
    // whose translated length varies (upstream hardcoded column 29).
    let cursor_column = 2 + prompt.chars().count() as i32 + 1;

    loop {
        move_cursor(Coord::new(20, cursor_column));
        let key = get_key_input();
        let upper_key = key.to_ascii_uppercase();
        // jdbkmoria extension: match the m/f letters shown by the locale's
        // "m) Male       f) Female" translation (fr_CA: h/f).
        let def = crate::locale::locale();

        if upper_key == def.female_key {
            player_set_gender(false);
            put_string(tr!("Female"), Coord::new(4, 15));
            break;
        } else if upper_key == def.male_key {
            player_set_gender(true);
            put_string(tr!("Male"), Coord::new(4, 15));
            break;
        } else if key == '?' {
            display_text_help_file(&config::files::localized(config::files::WELCOME_SCREEN));
        } else {
            terminal_bell_sound();
        }
    }
}

// Computes character's age, height, and weight -JWT-
fn character_set_age_height_weight() {
    let race = &CHARACTER_RACES[py().misc.race_id as usize];

    py().misc.age = (race.base_age as i32 + random_number(race.max_age as i32)) as u16;

    let height_base;
    let height_mod;
    let weight_base;
    let weight_mod;
    if player_is_male() {
        height_base = race.male_height_base;
        height_mod = race.male_height_mod;
        weight_base = race.male_weight_base;
        weight_mod = race.male_weight_mod;
    } else {
        height_base = race.female_height_base;
        height_mod = race.female_height_mod;
        weight_base = race.female_weight_base;
        weight_mod = race.female_weight_mod;
    }

    py().misc.height = random_number_normal_distribution(height_base as i32, height_mod as i32) as u16;
    py().misc.weight = random_number_normal_distribution(weight_base as i32, weight_mod as i32) as u16;
    py().misc.disarm = race.disarm_chance_base + player_disarm_adjustment();
}

// Prints the classes for a given race: Rogue, Mage, Priest, etc.,
// shown during the character creation screens.
fn display_race_classes(race_id: u8, class_list: &mut [u8; PLAYER_MAX_CLASSES]) -> usize {
    let mut coord = Coord::new(21, 2);

    let mut class_id: usize = 0;
    let mut mask: u32 = 0x1;

    clear_to_bottom(20);
    put_string(tr!("Choose a class (? for Help):"), Coord::new(20, 2));

    for i in 0..PLAYER_MAX_CLASSES {
        if (CHARACTER_RACES[race_id as usize].classes_bit_field as u32 & mask) != 0 {
            let description = format!("{}) {}", (b'a' + class_id as u8) as char, tr!(CLASSES[i].title));
            put_string(&description, coord);
            class_list[class_id] = i as u8;

            coord.x += 15;
            if coord.x > 70 {
                coord.x = 2;
                coord.y += 1;
            }
            class_id += 1;
        }
        mask <<= 1;
    }

    class_id
}

fn generate_character_class(class_id: u8) {
    py().misc.class_id = class_id;

    let class = &CLASSES[py().misc.class_id as usize];

    clear_to_bottom(20);
    put_string(tr!(class.title), Coord::new(5, 15));

    // Adjust the stats for the class adjustment -RAK-
    py().stats.max[A_STR] = create_modify_player_stat(py().stats.max[A_STR], class.strength);
    py().stats.max[A_INT] = create_modify_player_stat(py().stats.max[A_INT], class.intelligence);
    py().stats.max[A_WIS] = create_modify_player_stat(py().stats.max[A_WIS], class.wisdom);
    py().stats.max[A_DEX] = create_modify_player_stat(py().stats.max[A_DEX], class.dexterity);
    py().stats.max[A_CON] = create_modify_player_stat(py().stats.max[A_CON], class.constitution);
    py().stats.max[A_CHR] = create_modify_player_stat(py().stats.max[A_CHR], class.charisma);

    for i in 0..6 {
        py().stats.current[i] = py().stats.max[i];
        player_set_and_use_stat(i);
    }

    // Real values
    py().misc.plusses_to_damage = player_damage_adjustment();
    py().misc.plusses_to_hit = player_to_hit_adjustment();
    py().misc.magical_ac = player_armor_class_adjustment();
    py().misc.ac = 0;

    // Displayed values
    py().misc.display_to_damage = py().misc.plusses_to_damage;
    py().misc.display_to_hit = py().misc.plusses_to_hit;
    py().misc.display_to_ac = py().misc.magical_ac;
    py().misc.display_ac = py().misc.ac + py().misc.display_to_ac;

    // now set misc stats, do this after setting stats because of playerStatAdjustmentConstitution() for hit-points
    py().misc.hit_die += class.hit_points;
    py().misc.max_hp = (player_stat_adjustment_constitution() + py().misc.hit_die as i32) as i16;
    py().misc.current_hp = py().misc.max_hp;
    py().misc.current_hp_fraction = 0;

    // Initialize hit_points array.
    // Put bounds on total possible hp, only succeed
    // if it is within 1/8 of average value.
    let min_value = (PLAYER_MAX_LEVEL as i32 * 3 / 8 * (py().misc.hit_die as i32 - 1)) + PLAYER_MAX_LEVEL as i32;
    let max_value = (PLAYER_MAX_LEVEL as i32 * 5 / 8 * (py().misc.hit_die as i32 - 1)) + PLAYER_MAX_LEVEL as i32;
    py().base_hp_levels[0] = py().misc.hit_die as u16;

    loop {
        for i in 1..PLAYER_MAX_LEVEL {
            py().base_hp_levels[i] = random_number(py().misc.hit_die as i32) as u16;
            py().base_hp_levels[i] += py().base_hp_levels[i - 1];
        }

        let last = py().base_hp_levels[PLAYER_MAX_LEVEL - 1] as i32;
        if last >= min_value && last <= max_value {
            break;
        }
    }

    py().misc.bth += class.base_to_hit as i16;
    py().misc.bth_with_bows += class.base_to_hit_with_bows as i16; // RAK
    py().misc.chance_in_search += class.searching as i16;
    py().misc.disarm += class.disarm_traps as i16;
    py().misc.fos += class.fos as i16;
    py().misc.stealth_factor += class.stealth as i16;
    py().misc.saving_throw += class.saving_throw as i16;
    py().misc.experience_factor += class.experience_factor;
}

// Gets a character class -JWT-
fn character_get_class() {
    let mut class_list = [0u8; PLAYER_MAX_CLASSES];

    let class_count = display_race_classes(py().misc.race_id, &mut class_list);

    // Reset the class ID
    py().misc.class_id = 0;

    loop {
        move_cursor(Coord::new(20, 31));
        let key = get_key_input();

        let id = key as i32 - 97; // ASCII `a`, setting id to 0-5
        if id >= 0 && (id as usize) < class_count {
            generate_character_class(class_list[id as usize]);
            break;
        } else if key == '?' {
            display_text_help_file(&config::files::localized(config::files::WELCOME_SCREEN));
        } else {
            terminal_bell_sound();
        }
    }
}

// Given a stat value, return a monetary value,
// which affects the amount of gold a player has.
fn monetary_value_calculated_from_stat(stat: u8) -> i32 {
    5 * (stat as i32 - 10)
}

fn player_calculate_start_gold() {
    let mut value = monetary_value_calculated_from_stat(py().stats.max[A_STR]);
    value += monetary_value_calculated_from_stat(py().stats.max[A_INT]);
    value += monetary_value_calculated_from_stat(py().stats.max[A_WIS]);
    value += monetary_value_calculated_from_stat(py().stats.max[A_CON]);
    value += monetary_value_calculated_from_stat(py().stats.max[A_DEX]);

    // Social Class adjustment
    let mut new_gold = py().misc.social_class as i32 * 6 + random_number(25) + 325;

    // Stat adjustment
    new_gold -= value;

    // Charisma adjustment
    new_gold += monetary_value_calculated_from_stat(py().stats.max[A_CHR]);

    // She charmed the banker into it! -CJS-
    if !player_is_male() {
        new_gold += 50;
    }

    // Minimum
    if new_gold < 80 {
        new_gold = 80;
    }

    py().misc.au = new_gold;
}

// Main Character Creation Routine -JWT-
pub fn character_create() {
    // jdbkmoria extension: a classic static screen (the caller switches
    // back to the dungeon view's own centering once gameplay starts).
    center_for_static_screen();

    print_character_information();
    character_choose_race();
    character_set_gender();

    // here we start a loop giving a player a choice of characters -RGM-
    let mut done = false;
    while !done {
        character_generate_stats_and_race();
        character_get_history();
        character_set_age_height_weight();
        display_character_history();
        print_character_vital_statistics();
        print_character_stats();

        clear_to_bottom(20);
        put_string(tr!("Hit space to re-roll or ESC to accept characteristics: "), Coord::new(20, 2));

        loop {
            let key = get_key_input();
            if key == ESCAPE {
                done = true;
                break;
            } else if key == ' ' {
                break;
            } else {
                terminal_bell_sound();
            }
        }
    }

    character_get_class();
    player_calculate_start_gold();
    print_character_stats();
    print_character_level_experience();
    print_character_abilities();
    get_character_name();

    put_string_clear_to_eol(tr!("[ press any key to continue, or Q to exit ]"), Coord::new(23, 17));
    if get_key_input() == 'Q' {
        exit_program();
    }
    erase_line(Coord::new(23, 0));
}
