// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Panel holds data about a screen panel (the dungeon display)
// Screen panels calculated from the dungeon/screen dimensions
#[derive(Debug, Clone, Copy, Default)]
pub struct Panel {
    pub row: i32,
    pub col: i32,

    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,

    pub col_prt: i32,
    pub row_prt: i32,

    pub max_rows: i16,
    pub max_cols: i16,
}

impl Panel {
    pub const fn empty() -> Self {
        Panel {
            row: 0,
            col: 0,
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            col_prt: 0,
            row_prt: 0,
            max_rows: 0,
            max_cols: 0,
        }
    }
}

// message line location
pub const MSG_LINE: i32 = 0;

// How many messages to save in the buffer -CJS-
pub const MESSAGE_HISTORY_SIZE: usize = 22;

// Column for stats
pub const STAT_COLUMN: i32 = 0;

pub const fn ctrl_key(x: char) -> char {
    ((x as u8) & 0x1F) as char
}

pub const DELETE: char = 0x7f as char;

pub const ESCAPE: char = '\x1b'; // ESCAPE character -CJS-

use crate::config;
use crate::data_player::{CLASSES, CLASS_LEVEL_ADJ, CHARACTER_RACES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dungeon::{dg, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::dungeon_tile::TILE_LIGHT_FLOOR;
use crate::game::game;
use crate::mage_spells::spell_chance_of_success;
use crate::player::{
    py, player_calculate_allowed_spells_count, player_calculate_hit_points,
    player_disarm_adjustment, player_gain_mana, player_get_gender_label, player_rank_title,
    player_stat_adjustment_wisdom_intelligence, A_INT, A_WIS, BTH_PER_PLUS_TO_HIT_ADJUST,
    CLASS_BTH, CLASS_BTHB, CLASS_DEVICE, CLASS_DISARM, CLASS_SAVE, PLAYER_MAX_LEVEL,
};
use crate::player_run::player_end_running;
use crate::types::Coord;
use crate::ui_io::{
    clear_screen, clear_to_bottom, erase_line, get_default_player_name, get_key_input,
    get_string_input, panel_put_tile, print_message, put_string, put_string_clear_to_eol,
    terminal_bell_sound,
};

static STAT_NAMES: [&str; 6] = ["STR : ", "INT : ", "WIS : ", "DEX : ", "CON : ", "CHR : "];

// print `width` spaces at the given position (the C code used a shared blank string)
fn put_blanks(width: usize, coord: Coord) {
    put_string(&" ".repeat(width), coord);
}

// Calculates current boundaries -RAK-
fn panel_bounds() {
    let panel = &mut dg().panel;
    panel.top = panel.row * (SCREEN_HEIGHT / 2);
    panel.bottom = panel.top + SCREEN_HEIGHT - 1;
    panel.row_prt = panel.top - 1;
    panel.left = panel.col * (SCREEN_WIDTH / 2);
    panel.right = panel.left + SCREEN_WIDTH - 1;
    panel.col_prt = panel.left - 13;
}

// Given an row (y) and col (x), this routine detects -RAK-
// when a move off the screen has occurred and figures new borders.
// `force` forces the panel bounds to be recalculated, useful for 'W'here.
pub fn coord_outside_panel(coord: Coord, force: bool) -> bool {
    let mut panel = Coord::new(dg().panel.row, dg().panel.col);

    if force || coord.y < dg().panel.top + 2 || coord.y > dg().panel.bottom - 2 {
        panel.y = (coord.y - SCREEN_HEIGHT / 4) / (SCREEN_HEIGHT / 2);

        if panel.y > dg().panel.max_rows as i32 {
            panel.y = dg().panel.max_rows as i32;
        } else if panel.y < 0 {
            panel.y = 0;
        }
    }

    if force || coord.x < dg().panel.left + 3 || coord.x > dg().panel.right - 3 {
        panel.x = (coord.x - SCREEN_WIDTH / 4) / (SCREEN_WIDTH / 2);
        if panel.x > dg().panel.max_cols as i32 {
            panel.x = dg().panel.max_cols as i32;
        } else if panel.x < 0 {
            panel.x = 0;
        }
    }

    if panel.y != dg().panel.row || panel.x != dg().panel.col {
        dg().panel.row = panel.y;
        dg().panel.col = panel.x;
        panel_bounds();

        // stop movement if any
        if config::options::options().find_bound {
            player_end_running();
        }

        // Yes, the coordinates are beyond the current panel boundary
        return true;
    }

    false
}

// Is the given coordinate within the screen panel boundaries -RAK-
pub fn coord_inside_panel(coord: Coord) -> bool {
    let valid_y = coord.y >= dg().panel.top && coord.y <= dg().panel.bottom;
    let valid_x = coord.x >= dg().panel.left && coord.x <= dg().panel.right;

    valid_y && valid_x
}

// Prints the map of the dungeon -RAK-
pub fn draw_dungeon_panel() {
    let mut line = 1;

    // Top to bottom
    for y in dg().panel.top..=dg().panel.bottom {
        erase_line(Coord::new(line, 13));
        line += 1;

        // Left to right
        for x in dg().panel.left..=dg().panel.right {
            let coord = Coord::new(y, x);
            let ch = crate::dungeon::cave_get_tile_symbol(coord);
            if ch != ' ' {
                panel_put_tile(ch, coord);
            }
        }
    }
}

// Draws entire screen -RAK-
pub fn draw_cave_panel() {
    clear_screen();
    print_character_stats_block();
    draw_dungeon_panel();
    print_character_current_depth();
}

// We need to reset the view of things. -CJS-
pub fn dungeon_reset_view() {
    // Check for new panel
    if coord_outside_panel(py().pos, false) {
        draw_dungeon_panel();
    }

    // Move the light source
    crate::dungeon::dungeon_move_character_light(py().pos, py().pos);

    let tile = dg().floor[py().pos.y as usize][py().pos.x as usize];

    // A room of light should be lit.
    if tile.feature_id == TILE_LIGHT_FLOOR {
        if py().flags.blind < 1 && !tile.permanent_light {
            crate::dungeon::dungeon_light_room(py().pos);
        }
        return;
    }

    // In doorway of light-room?
    if tile.perma_lit_room && py().flags.blind < 1 {
        for i in (py().pos.y - 1)..=(py().pos.y + 1) {
            for j in (py().pos.x - 1)..=(py().pos.x + 1) {
                let neighbour = dg().floor[i as usize][j as usize];
                if neighbour.feature_id == TILE_LIGHT_FLOOR && !neighbour.permanent_light {
                    crate::dungeon::dungeon_light_room(Coord::new(i, j));
                }
            }
        }
    }
}

// Converts stat num into string
pub fn stats_as_string(stat: u8) -> String {
    let percentile = stat as i32 - 18;

    if stat <= 18 {
        format!("{:6}", stat)
    } else if percentile == 100 {
        "18/100".to_string()
    } else {
        format!(" 18/{:02}", percentile)
    }
}

// Print character stat in given row, column -RAK-
pub fn display_character_stats(stat: usize) {
    let text = stats_as_string(py().stats.used[stat]);
    put_string(STAT_NAMES[stat], Coord::new(6 + stat as i32, STAT_COLUMN));
    put_string(&text, Coord::new(6 + stat as i32, STAT_COLUMN + 6));
}

// Print character info in given row, column -RAK-
// The longest title is 13 characters, so only pad to 13
fn print_character_info_in_field(info: &str, coord: Coord) {
    // blank out the current field space
    put_blanks(13, coord);

    put_string(info, coord);
}

// Print long number with header at given row, column
fn print_header_long_number(header: &str, num: i32, coord: Coord) {
    put_string(&format!("{}: {:6}", header, num), coord);
}

// Print long number (7 digits of space) with header at given row, column
fn print_header_long_number_7_spaces(header: &str, num: i32, coord: Coord) {
    put_string(&format!("{}: {:7}", header, num), coord);
}

// Print number with header at given row, column -RAK-
fn print_header_number(header: &str, num: i32, coord: Coord) {
    put_string(&format!("{}: {:6}", header, num), coord);
}

// Print long number at given row, column
fn print_long_number(num: i32, coord: Coord) {
    put_string(&format!("{:6}", num), coord);
}

// Print number at given row, column -RAK-
fn print_number(num: i32, coord: Coord) {
    put_string(&format!("{:6}", num), coord);
}

// Prints title of character -RAK-
pub fn print_character_title() {
    print_character_info_in_field(&player_rank_title(), Coord::new(4, STAT_COLUMN));
}

// Prints level -RAK-
pub fn print_character_level() {
    print_number(py().misc.level as i32, Coord::new(13, STAT_COLUMN + 6));
}

// Prints players current mana points. -RAK-
pub fn print_character_current_mana() {
    print_number(py().misc.current_mana as i32, Coord::new(15, STAT_COLUMN + 6));
}

// Prints Max hit points -RAK-
pub fn print_character_max_hit_points() {
    print_number(py().misc.max_hp as i32, Coord::new(16, STAT_COLUMN + 6));
}

// Prints players current hit points -RAK-
pub fn print_character_current_hit_points() {
    print_number(py().misc.current_hp as i32, Coord::new(17, STAT_COLUMN + 6));
}

// prints current AC -RAK-
pub fn print_character_current_armor_class() {
    print_number(py().misc.display_ac as i32, Coord::new(19, STAT_COLUMN + 6));
}

// Prints current gold -RAK-
pub fn print_character_gold_value() {
    print_long_number(py().misc.au, Coord::new(20, STAT_COLUMN + 6));
}

// Prints depth in stat area -RAK-
pub fn print_character_current_depth() {
    let depth = dg().current_level as i32 * 50;

    let depths = if depth == 0 {
        "Town level".to_string()
    } else {
        format!("{} feet", depth)
    };

    put_string_clear_to_eol(&depths, Coord::new(23, 65));
}

// Prints status of hunger -RAK-
pub fn print_character_hunger_status() {
    if (py().flags.status & config::player::status::PY_WEAK) != 0 {
        put_string("Weak  ", Coord::new(23, 0));
    } else if (py().flags.status & config::player::status::PY_HUNGRY) != 0 {
        put_string("Hungry", Coord::new(23, 0));
    } else {
        put_blanks(6, Coord::new(23, 0));
    }
}

// Prints Blind status -RAK-
pub fn print_character_blind_status() {
    if (py().flags.status & config::player::status::PY_BLIND) != 0 {
        put_string("Blind", Coord::new(23, 7));
    } else {
        put_blanks(5, Coord::new(23, 7));
    }
}

// Prints Confusion status -RAK-
pub fn print_character_confused_state() {
    if (py().flags.status & config::player::status::PY_CONFUSED) != 0 {
        put_string("Confused", Coord::new(23, 13));
    } else {
        put_blanks(8, Coord::new(23, 13));
    }
}

// Prints Fear status -RAK-
pub fn print_character_fear_state() {
    if (py().flags.status & config::player::status::PY_FEAR) != 0 {
        put_string("Afraid", Coord::new(23, 22));
    } else {
        put_blanks(6, Coord::new(23, 22));
    }
}

// Prints Poisoned status -RAK-
pub fn print_character_poisoned_state() {
    if (py().flags.status & config::player::status::PY_POISONED) != 0 {
        put_string("Poisoned", Coord::new(23, 29));
    } else {
        put_blanks(8, Coord::new(23, 29));
    }
}

// Prints Searching, Resting, Paralysis, or 'count' status -RAK-
pub fn print_character_movement_state() {
    py().flags.status &= !config::player::status::PY_REPEAT;

    if py().flags.paralysis > 1 {
        put_string("Paralysed", Coord::new(23, 38));
        return;
    }

    if (py().flags.status & config::player::status::PY_REST) != 0 {
        let rest_string = if py().flags.rest < 0 {
            "Rest *".to_string()
        } else if config::options::options().display_counts {
            format!("Rest {:<5}", py().flags.rest)
        } else {
            "Rest".to_string()
        };

        put_string(&rest_string, Coord::new(23, 38));

        return;
    }

    if game().command_count > 0 {
        let repeat_string = if config::options::options().display_counts {
            format!("Repeat {:03}", game().command_count)
        } else {
            "Repeat".to_string()
        };

        py().flags.status |= config::player::status::PY_REPEAT;

        put_string(&repeat_string, Coord::new(23, 38));

        if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
            put_string("Search", Coord::new(23, 38));
        }

        return;
    }

    if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
        put_string("Searching", Coord::new(23, 38));
        return;
    }

    // "repeat 999" is 10 characters
    put_blanks(10, Coord::new(23, 38));
}

// Prints the speed of a character. -CJS-
pub fn print_character_speed() {
    let mut speed = py().flags.speed;

    // Search mode.
    if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
        speed -= 1;
    }

    if speed > 1 {
        put_string("Very Slow", Coord::new(23, 49));
    } else if speed == 1 {
        put_string("Slow     ", Coord::new(23, 49));
    } else if speed == 0 {
        put_blanks(9, Coord::new(23, 49));
    } else if speed == -1 {
        put_string("Fast     ", Coord::new(23, 49));
    } else {
        put_string("Very Fast", Coord::new(23, 49));
    }
}

pub fn print_character_study_instruction() {
    py().flags.status &= !config::player::status::PY_STUDY;

    if py().flags.new_spells_to_learn == 0 {
        put_blanks(5, Coord::new(23, 59));
    } else {
        put_string("Study", Coord::new(23, 59));
    }
}

// Prints winner status on display -RAK-
pub fn print_character_winner() {
    if (game().noscore & 0x2) != 0 {
        if game().wizard_mode {
            put_string("Is wizard  ", Coord::new(22, 0));
        } else {
            put_string("Was wizard ", Coord::new(22, 0));
        }
    } else if (game().noscore & 0x1) != 0 {
        put_string("Resurrected", Coord::new(22, 0));
    } else if (game().noscore & 0x4) != 0 {
        put_string("Duplicate", Coord::new(22, 0));
    } else if game().total_winner {
        put_string("*Winner*   ", Coord::new(22, 0));
    }
}

// Prints character-screen info -RAK-
pub fn print_character_stats_block() {
    print_character_info_in_field(CHARACTER_RACES[py().misc.race_id as usize].name, Coord::new(2, STAT_COLUMN));
    print_character_info_in_field(CLASSES[py().misc.class_id as usize].title, Coord::new(3, STAT_COLUMN));
    print_character_info_in_field(&player_rank_title(), Coord::new(4, STAT_COLUMN));

    for i in 0..6 {
        display_character_stats(i);
    }

    print_header_number("LEV ", py().misc.level as i32, Coord::new(13, STAT_COLUMN));
    print_header_long_number("EXP ", py().misc.exp, Coord::new(14, STAT_COLUMN));
    print_header_number("MANA", py().misc.current_mana as i32, Coord::new(15, STAT_COLUMN));
    print_header_number("MHP ", py().misc.max_hp as i32, Coord::new(16, STAT_COLUMN));
    print_header_number("CHP ", py().misc.current_hp as i32, Coord::new(17, STAT_COLUMN));
    print_header_number("AC  ", py().misc.display_ac as i32, Coord::new(19, STAT_COLUMN));
    print_header_long_number("GOLD", py().misc.au, Coord::new(20, STAT_COLUMN));
    print_character_winner();

    let status = py().flags.status;

    if ((config::player::status::PY_HUNGRY | config::player::status::PY_WEAK) & status) != 0 {
        print_character_hunger_status();
    }

    if (status & config::player::status::PY_BLIND) != 0 {
        print_character_blind_status();
    }

    if (status & config::player::status::PY_CONFUSED) != 0 {
        print_character_confused_state();
    }

    if (status & config::player::status::PY_FEAR) != 0 {
        print_character_fear_state();
    }

    if (status & config::player::status::PY_POISONED) != 0 {
        print_character_poisoned_state();
    }

    if ((config::player::status::PY_SEARCH | config::player::status::PY_REST) & status) != 0 {
        print_character_movement_state();
    }

    // if speed non zero, print it, modify speed if Searching
    let speed = py().flags.speed - ((status & config::player::status::PY_SEARCH) >> 8) as i16;
    if speed != 0 {
        print_character_speed();
    }

    // display the study field
    print_character_study_instruction();
}

// Prints the following information on the screen. -JWT-
pub fn print_character_information() {
    clear_screen();

    put_string("Name        :", Coord::new(2, 1));
    put_string("Race        :", Coord::new(3, 1));
    put_string("Sex         :", Coord::new(4, 1));
    put_string("Class       :", Coord::new(5, 1));

    if !game().character_generated {
        return;
    }

    let name = py().misc.name.clone();
    put_string(&name, Coord::new(2, 15));
    put_string(CHARACTER_RACES[py().misc.race_id as usize].name, Coord::new(3, 15));
    put_string(player_get_gender_label(), Coord::new(4, 15));
    put_string(CLASSES[py().misc.class_id as usize].title, Coord::new(5, 15));
}

// Prints the following information on the screen. -JWT-
pub fn print_character_stats() {
    for i in 0..6 {
        let buf = stats_as_string(py().stats.used[i]);
        put_string(STAT_NAMES[i], Coord::new(2 + i as i32, 61));
        put_string(&buf, Coord::new(2 + i as i32, 66));

        if py().stats.max[i] > py().stats.current[i] {
            let buf = stats_as_string(py().stats.max[i]);
            put_string(&buf, Coord::new(2 + i as i32, 73));
        }
    }

    print_header_number("+ To Hit    ", py().misc.display_to_hit as i32, Coord::new(9, 1));
    print_header_number("+ To Damage ", py().misc.display_to_damage as i32, Coord::new(10, 1));
    print_header_number("+ To AC     ", py().misc.display_to_ac as i32, Coord::new(11, 1));
    print_header_number("  Total AC  ", py().misc.display_ac as i32, Coord::new(12, 1));
}

// Returns a rating of x depending on y -JWT-
pub fn stat_rating(coord: Coord) -> &'static str {
    match coord.x / coord.y {
        -3 | -2 | -1 => "Very Bad",
        0 | 1 => "Bad",
        2 => "Poor",
        3 | 4 => "Fair",
        5 => "Good",
        6 => "Very Good",
        7 | 8 => "Excellent",
        _ => "Superb",
    }
}

// Prints age, height, weight, and SC -JWT-
pub fn print_character_vital_statistics() {
    print_header_number("Age          ", py().misc.age as i32, Coord::new(2, 38));
    print_header_number("Height       ", py().misc.height as i32, Coord::new(3, 38));
    print_header_number("Weight       ", py().misc.weight as i32, Coord::new(4, 38));
    print_header_number("Social Class ", py().misc.social_class as i32, Coord::new(5, 38));
}

// Prints the following information on the screen. -JWT-
pub fn print_character_level_experience() {
    print_header_long_number_7_spaces("Level      ", py().misc.level as i32, Coord::new(9, 28));
    print_header_long_number_7_spaces("Experience ", py().misc.exp, Coord::new(10, 28));
    print_header_long_number_7_spaces("Max Exp    ", py().misc.max_exp, Coord::new(11, 28));

    if py().misc.level as usize >= PLAYER_MAX_LEVEL {
        put_string_clear_to_eol("Exp to Adv.: *******", Coord::new(12, 28));
    } else {
        print_header_long_number_7_spaces(
            "Exp to Adv.",
            (py().base_exp_levels[py().misc.level as usize - 1] as i64 * py().misc.experience_factor as i64 / 100) as i32,
            Coord::new(12, 28),
        );
    }

    print_header_long_number_7_spaces("Gold       ", py().misc.au, Coord::new(13, 28));
    print_header_number("Max Hit Points ", py().misc.max_hp as i32, Coord::new(9, 52));
    print_header_number("Cur Hit Points ", py().misc.current_hp as i32, Coord::new(10, 52));
    print_header_number("Max Mana       ", py().misc.mana as i32, Coord::new(11, 52));
    print_header_number("Cur Mana       ", py().misc.current_mana as i32, Coord::new(12, 52));
}

// Prints ratings on certain abilities -RAK-
pub fn print_character_abilities() {
    clear_to_bottom(14);

    let misc = &py().misc;
    let class_id = misc.class_id as usize;
    let level = misc.level as i32;

    let xbth = misc.bth as i32 + misc.plusses_to_hit as i32 * BTH_PER_PLUS_TO_HIT_ADJUST + (CLASS_LEVEL_ADJ[class_id][CLASS_BTH] as i32 * level);
    let xbthb = misc.bth_with_bows as i32 + misc.plusses_to_hit as i32 * BTH_PER_PLUS_TO_HIT_ADJUST + (CLASS_LEVEL_ADJ[class_id][CLASS_BTHB] as i32 * level);

    // this results in a range from 0 to 29
    let mut xfos = 40 - misc.fos as i32;
    if xfos < 0 {
        xfos = 0;
    }

    let xsrh = misc.chance_in_search as i32;

    // this results in a range from 0 to 9
    let xstl = misc.stealth_factor as i32 + 1;
    let xdis = misc.disarm as i32 + 2 * player_disarm_adjustment() as i32 + player_stat_adjustment_wisdom_intelligence(A_INT)
        + (CLASS_LEVEL_ADJ[class_id][CLASS_DISARM] as i32 * level / 3);
    let xsave = misc.saving_throw as i32 + player_stat_adjustment_wisdom_intelligence(A_WIS)
        + (CLASS_LEVEL_ADJ[class_id][CLASS_SAVE] as i32 * level / 3);
    let xdev = misc.saving_throw as i32 + player_stat_adjustment_wisdom_intelligence(A_INT)
        + (CLASS_LEVEL_ADJ[class_id][CLASS_DEVICE] as i32 * level / 3);

    let xinfra = format!("{} feet", py().flags.see_infra * 10);

    put_string("(Miscellaneous Abilities)", Coord::new(15, 25));
    put_string("Fighting    :", Coord::new(16, 1));
    put_string(stat_rating(Coord::new(12, xbth)), Coord::new(16, 15));
    put_string("Bows/Throw  :", Coord::new(17, 1));
    put_string(stat_rating(Coord::new(12, xbthb)), Coord::new(17, 15));
    put_string("Saving Throw:", Coord::new(18, 1));
    put_string(stat_rating(Coord::new(6, xsave)), Coord::new(18, 15));

    put_string("Stealth     :", Coord::new(16, 28));
    put_string(stat_rating(Coord::new(1, xstl)), Coord::new(16, 42));
    put_string("Disarming   :", Coord::new(17, 28));
    put_string(stat_rating(Coord::new(8, xdis)), Coord::new(17, 42));
    put_string("Magic Device:", Coord::new(18, 28));
    put_string(stat_rating(Coord::new(6, xdev)), Coord::new(18, 42));

    put_string("Perception  :", Coord::new(16, 55));
    put_string(stat_rating(Coord::new(3, xfos)), Coord::new(16, 69));
    put_string("Searching   :", Coord::new(17, 55));
    put_string(stat_rating(Coord::new(6, xsrh)), Coord::new(17, 69));
    put_string("Infra-Vision:", Coord::new(18, 55));
    put_string(&xinfra, Coord::new(18, 69));
}

// Used to display the character on the screen. -RAK-
pub fn print_character() {
    print_character_information();
    print_character_vital_statistics();
    print_character_stats();
    print_character_level_experience();
    print_character_abilities();
}

// Gets a name for the character -JWT-
pub fn get_character_name() {
    put_string_clear_to_eol("Enter your player's name  [press <RETURN> when finished]", Coord::new(21, 2));

    put_blanks(23, Coord::new(2, 15));

    let mut name = String::new();
    if !get_string_input(&mut name, Coord::new(2, 15), 23) || name.is_empty() {
        name = get_default_player_name();
        put_string(&name, Coord::new(2, 15));
    }
    py().misc.name = name;

    clear_to_bottom(20);
}

// Changes the name of the character -JWT-
pub fn change_character_name() {
    let mut flag = false;

    print_character();

    while !flag {
        put_string_clear_to_eol("<f>ile character description. <c>hange character name.", Coord::new(21, 2));

        match get_key_input() {
            'c' => {
                get_character_name();
                flag = true;
            }
            'f' => {
                put_string_clear_to_eol("File name:", Coord::new(0, 0));

                let mut temp = String::new();
                if get_string_input(&mut temp, Coord::new(0, 10), 60) && !temp.is_empty() {
                    if crate::game_files::output_player_character_to_file(&temp) {
                        flag = true;
                    }
                }
            }
            ESCAPE | ' ' | '\n' | '\r' => {
                flag = true;
            }
            _ => {
                terminal_bell_sound();
            }
        }
    }
}

// Print list of spells -RAK-
// if non_consecutive is  -1: spells numbered consecutively from 'a' to 'a'+num
//                       >=0: spells numbered by offset from non_consecutive
pub fn display_spells_list(spell_ids: &[i32], number_of_choices: i32, comment: bool, non_consecutive: i32) {
    let col = if comment { 22 } else { 31 };

    let consecutive_offset = if CLASSES[py().misc.class_id as usize].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        config::spells::NAME_OFFSET_SPELLS
    } else {
        config::spells::NAME_OFFSET_PRAYERS
    };

    erase_line(Coord::new(1, col));
    put_string("Name", Coord::new(1, col + 5));
    put_string("Lv Mana Fail", Coord::new(1, col + 35));

    // only show the first 22 choices
    let number_of_choices = std::cmp::min(number_of_choices, 22);

    for i in 0..number_of_choices as usize {
        let spell_id = spell_ids[i];
        let spell = &MAGIC_SPELLS[py().misc.class_id as usize - 1][spell_id as usize];

        let p = if !comment {
            ""
        } else if (py().flags.spells_forgotten & (1u32 << spell_id)) != 0 {
            " forgotten"
        } else if (py().flags.spells_learnt & (1u32 << spell_id)) == 0 {
            " unknown"
        } else if (py().flags.spells_worked & (1u32 << spell_id)) == 0 {
            " untried"
        } else {
            ""
        };

        // determine whether or not to leave holes in character choices, non_consecutive -1
        // when learning spells, consecutive_offset>=0 when asking which spell to cast.
        let spell_char = if non_consecutive == -1 {
            (b'a' + i as u8) as char
        } else {
            (b'a' as i32 + spell_id - non_consecutive) as u8 as char
        };

        let out_val = format!(
            "  {}) {:<30}{:2} {:4} {:3}%{}",
            spell_char,
            SPELL_NAMES[(spell_id + consecutive_offset as i32) as usize],
            spell.level_required,
            spell.mana_required,
            spell_chance_of_success(spell_id),
            p
        );
        put_string_clear_to_eol(&out_val, Coord::new(2 + i as i32, col));
    }
}

// Increases hit points and level -RAK-
fn player_gain_level() {
    py().misc.level += 1;

    let msg = format!("Welcome to level {}.", py().misc.level);
    print_message(Some(&msg));

    player_calculate_hit_points();

    let new_exp = (py().base_exp_levels[py().misc.level as usize - 1] as i64 * py().misc.experience_factor as i64 / 100) as i32;

    if py().misc.exp > new_exp {
        // lose some of the 'extra' exp when gaining several levels at once
        let dif_exp = py().misc.exp - new_exp;
        py().misc.exp = new_exp + (dif_exp / 2);
    }

    print_character_level();
    print_character_title();

    let class_to_use_mage_spells = CLASSES[py().misc.class_id as usize].class_to_use_mage_spells;

    if class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        player_calculate_allowed_spells_count(A_INT);
        player_gain_mana(A_INT);
    } else if class_to_use_mage_spells == config::spells::SPELL_TYPE_PRIEST {
        player_calculate_allowed_spells_count(A_WIS);
        player_gain_mana(A_WIS);
    }
}

// Prints experience -RAK-
pub fn display_character_experience() {
    if py().misc.exp > config::player::PLAYER_MAX_EXP {
        py().misc.exp = config::player::PLAYER_MAX_EXP;
    }

    while (py().misc.level as usize) < PLAYER_MAX_LEVEL
        && (py().base_exp_levels[py().misc.level as usize - 1] as i64 * py().misc.experience_factor as i64 / 100) as i32 <= py().misc.exp
    {
        player_gain_level();
    }

    if py().misc.exp > py().misc.max_exp {
        py().misc.max_exp = py().misc.exp;
    }

    print_long_number(py().misc.exp, Coord::new(14, STAT_COLUMN + 6));
}
