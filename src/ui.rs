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

// jdbkmoria extension: the on-screen dungeon viewport size, independent of
// SCREEN_HEIGHT/SCREEN_WIDTH (which drive dungeon generation and must stay
// untouched for save/gameplay compatibility, see PORTING.md). Defaults to
// the classic size so anything that never calls `set_view_size` behaves
// exactly as upstream; `terminal_initialize` grows it to fit the terminal.
static VIEW_HEIGHT: RacyCell<i32> = RacyCell::new(SCREEN_HEIGHT);
static VIEW_WIDTH: RacyCell<i32> = RacyCell::new(SCREEN_WIDTH);

pub fn view_height() -> i32 {
    *VIEW_HEIGHT.get()
}

pub fn view_width() -> i32 {
    *VIEW_WIDTH.get()
}

// Clamps into [SCREEN_HEIGHT, MAX_HEIGHT] / [SCREEN_WIDTH, MAX_WIDTH]: never
// smaller than the classic viewport, never bigger than a full dungeon level.
pub fn set_view_size(height: i32, width: i32) {
    *VIEW_HEIGHT.get() = height.clamp(SCREEN_HEIGHT, MAX_HEIGHT);
    *VIEW_WIDTH.get() = width.clamp(SCREEN_WIDTH, MAX_WIDTH);
}

// Row of the live status line (hunger/blind/speed/depth/... fields), just
// below the dungeon panel. Row 22 (the wizard/winner indicator line just
// above it) is `status_line_row() - 1`.
pub fn status_line_row() -> i32 {
    view_height() + 1
}

pub const fn ctrl_key(x: char) -> char {
    ((x as u8) & 0x1F) as char
}

pub const DELETE: char = 0x7f as char;

pub const ESCAPE: char = '\x1b'; // ESCAPE character -CJS-

use crate::config;
use crate::data_player::{CLASSES, CLASS_LEVEL_ADJ, CHARACTER_RACES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dungeon::{dg, MAX_HEIGHT, MAX_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::dungeon_tile::TILE_LIGHT_FLOOR;
use crate::game::game;
use crate::globals::RacyCell;
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
use crate::{tr, tr_fmt};

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
    // jdbkmoria extension: with an enlarged terminal, follow the player
    // directly instead of stepping between the classic discrete half-screen
    // panel positions (which only exist at SCREEN_HEIGHT/SCREEN_WIDTH size).
    if view_height() != SCREEN_HEIGHT || view_width() != SCREEN_WIDTH {
        return coord_outside_panel_enlarged(coord, force);
    }

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

// jdbkmoria extension: keeps the player at the exact center of the viewport
// whenever the view (re)centers, with the map oriented around the player —
// even when that leaves blank screen space past the map edges (a small
// level, or the player near an edge of a big one). Only recalculates once
// the player nears the edge of what's already shown (same margin the
// classic path uses) to avoid re-centering on every single step.
// Deliberately ignores `panel.max_rows`/`max_cols`/`row`/`col` (which stay
// tied to SCREEN_HEIGHT/SCREEN_WIDTH for save-file wire compatibility, see
// game_save.rs) so a stale value from a save written at a different
// terminal size can't produce an out-of-range panel here.
fn coord_outside_panel_enlarged(coord: Coord, force: bool) -> bool {
    let panel = dg().panel;

    // jdbkmoria extension: think in terms of a conceptual "window" —
    // view_height() rows by view_width() cols, centered on the player —
    // that is *not* clamped to the map. The stored panel.top/bottom/left/
    // right must stay the intersection of that window with the map, so
    // every existing consumer that iterates panel bounds and indexes
    // dg().floor (draw_dungeon_panel, the detection spells in spells.rs,
    // coord_inside_panel, etc.) keeps working unchanged. row_prt/col_prt
    // place the *window* (not the clamped panel) on screen, which is what
    // keeps the player centered even when part of the window falls off the
    // map: screen line for tile row y is `y - row_prt`, so window row
    // `wtop` lands on screen line 1 and the player lands on screen line
    // `view_height() / 2 + 1`. The window itself isn't stored, but it's
    // recoverable from row_prt/col_prt: `wtop = row_prt + 1`,
    // `wleft = col_prt + 13`.
    let wtop = panel.row_prt + 1;
    let wleft = panel.col_prt + 13;

    // A freshly generated level resets panel.top/bottom/left/right to 0
    // (see generate_cave()) before this ever runs, but leaves row_prt/
    // col_prt untouched from whatever level was previously displayed. So
    // reconstructing the window from row_prt/col_prt and re-deriving what
    // the panel bounds *should* be for that window will, in general, not
    // match the freshly zeroed top/bottom/left/right — correctly reading as
    // invalid and forcing a recompute. (In the corner case of the very
    // first level of a fresh game, row_prt/col_prt are also still zero from
    // Panel::empty(), giving wtop=1, wleft=13; panel.top==0 still doesn't
    // equal max(wtop, 0)==1, so this still correctly reads invalid.)
    let panel_valid = panel.top == wtop.max(0)
        && panel.bottom == (wtop + view_height() - 1).min(dg().height as i32 - 1)
        && panel.left == wleft.max(0)
        && panel.right == (wleft + view_width() - 1).min(dg().width as i32 - 1);

    if !force
        && panel_valid
        && coord.y >= wtop + 2
        && coord.y <= wtop + view_height() - 3
        && coord.x >= wleft + 3
        && coord.x <= wleft + view_width() - 4
    {
        return false;
    }

    let new_wtop = coord.y - view_height() / 2;
    let new_wleft = coord.x - view_width() / 2;

    if !force && panel_valid && new_wtop == wtop && new_wleft == wleft {
        return false;
    }

    let panel = &mut dg().panel;
    panel.top = new_wtop.max(0);
    panel.bottom = (new_wtop + view_height() - 1).min(dg().height as i32 - 1);
    panel.row_prt = new_wtop - 1;
    panel.left = new_wleft.max(0);
    panel.right = (new_wleft + view_width() - 1).min(dg().width as i32 - 1);
    panel.col_prt = new_wleft - 13;

    if config::options::options().find_bound {
        player_end_running();
    }

    true
}

// Is the given coordinate within the screen panel boundaries -RAK-
pub fn coord_inside_panel(coord: Coord) -> bool {
    let valid_y = coord.y >= dg().panel.top && coord.y <= dg().panel.bottom;
    let valid_x = coord.x >= dg().panel.left && coord.x <= dg().panel.right;

    valid_y && valid_x
}

// Prints the map of the dungeon -RAK-
// jdbkmoria extension: `line` used to be a manual counter starting at 1,
// mirroring the C code's separate screen-row counter (a direct match with
// the original control flow rather than a zip/enumerate rewrite) — that
// only holds when panel.top always maps to screen row 1. With the window
// centered on the player (see coord_outside_panel_enlarged) possibly
// extending past the map's edges, panel.top no longer maps to screen row 1
// in general, and the clamped panel.top..=panel.bottom range never visits
// the screen rows the window has past the map edge, so every viewport row
// (1..=view_height(), the same range panel_put_tile can address) is erased
// up front instead, to clear stale content left over from a previous,
// differently-positioned window; tiles are then drawn only for the clamped
// panel range, same as before.
pub fn draw_dungeon_panel() {
    for line in 1..=view_height() {
        erase_line(Coord::new(line, 13));
    }

    // Top to bottom
    for y in dg().panel.top..=dg().panel.bottom {
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
    // jdbkmoria extension: this is the canonical "return to the live
    // dungeon screen" redraw (from a store, a message screen, ...), so it's
    // where centering switches back from whatever static screen was
    // showing to the dungeon view's own centering (see ui_io.rs).
    crate::ui_io::center_for_dungeon_view();
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

    let tile = *dg().tile(py().pos);

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
                let neighbour = *dg().tile(Coord::new(i, j));
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
    put_string(&format!("{:<6.6}", tr!(STAT_NAMES[stat])), Coord::new(6 + stat as i32, STAT_COLUMN));
    put_string(&text, Coord::new(6 + stat as i32, STAT_COLUMN + 6));
}

// Print character info in given row, column -RAK-
// The longest title is 13 characters, so only pad to 13
fn print_character_info_in_field(info: &str, coord: Coord) {
    // blank out the current field space
    put_blanks(13, coord);

    put_string(&format!("{:.13}", info), coord);
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
        tr!("Town level").to_string()
    } else {
        tr_fmt!("{} feet", depth)
    };

    put_string_clear_to_eol(&depths, Coord::new(status_line_row(), 65));
}

// Prints status of hunger -RAK-
pub fn print_character_hunger_status() {
    if (py().flags.status & config::player::status::PY_WEAK) != 0 {
        put_string(&format!("{:<6.6}", tr!("Weak")), Coord::new(status_line_row(), 0));
    } else if (py().flags.status & config::player::status::PY_HUNGRY) != 0 {
        put_string(&format!("{:<6.6}", tr!("Hungry")), Coord::new(status_line_row(), 0));
    } else {
        put_blanks(6, Coord::new(status_line_row(), 0));
    }
}

// Prints Blind status -RAK-
pub fn print_character_blind_status() {
    if (py().flags.status & config::player::status::PY_BLIND) != 0 {
        put_string(&format!("{:<5.5}", tr!("Blind")), Coord::new(status_line_row(), 7));
    } else {
        put_blanks(5, Coord::new(status_line_row(), 7));
    }
}

// Prints Confusion status -RAK-
pub fn print_character_confused_state() {
    if (py().flags.status & config::player::status::PY_CONFUSED) != 0 {
        put_string(&format!("{:<8.8}", tr!("Confused")), Coord::new(status_line_row(), 13));
    } else {
        put_blanks(8, Coord::new(status_line_row(), 13));
    }
}

// Prints Fear status -RAK-
pub fn print_character_fear_state() {
    if (py().flags.status & config::player::status::PY_FEAR) != 0 {
        put_string(&format!("{:<6.6}", tr!("Afraid")), Coord::new(status_line_row(), 22));
    } else {
        put_blanks(6, Coord::new(status_line_row(), 22));
    }
}

// Prints Poisoned status -RAK-
pub fn print_character_poisoned_state() {
    if (py().flags.status & config::player::status::PY_POISONED) != 0 {
        put_string(&format!("{:<8.8}", tr!("Poisoned")), Coord::new(status_line_row(), 29));
    } else {
        put_blanks(8, Coord::new(status_line_row(), 29));
    }
}

// Prints Searching, Resting, Paralysis, or 'count' status -RAK-
pub fn print_character_movement_state() {
    py().flags.status &= !config::player::status::PY_REPEAT;

    if py().flags.paralysis > 1 {
        put_string(&format!("{:<9.9}", tr!("Paralysed")), Coord::new(status_line_row(), 38));
        return;
    }

    if (py().flags.status & config::player::status::PY_REST) != 0 {
        let rest_string = if py().flags.rest < 0 {
            format!("{:<6.6}", tr!("Rest *"))
        } else if config::options::options().display_counts {
            tr_fmt!("Rest {}", format!("{:<5}", py().flags.rest))
        } else {
            format!("{:<4.4}", tr!("Rest"))
        };

        put_string(&rest_string, Coord::new(status_line_row(), 38));

        return;
    }

    if game().command_count > 0 {
        let repeat_string = if config::options::options().display_counts {
            tr_fmt!("Repeat {}", format!("{:03}", game().command_count))
        } else {
            format!("{:<6.6}", tr!("Repeat"))
        };

        py().flags.status |= config::player::status::PY_REPEAT;

        put_string(&repeat_string, Coord::new(status_line_row(), 38));

        if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
            put_string(&format!("{:<6.6}", tr!("Search")), Coord::new(status_line_row(), 38));
        }

        return;
    }

    if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
        put_string(&format!("{:<9.9}", tr!("Searching")), Coord::new(status_line_row(), 38));
        return;
    }

    // "repeat 999" is 10 characters
    put_blanks(10, Coord::new(status_line_row(), 38));
}

// Prints the speed of a character. -CJS-
pub fn print_character_speed() {
    let mut speed = py().flags.speed;

    // Search mode.
    if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
        speed -= 1;
    }

    if speed > 1 {
        put_string(&format!("{:<9.9}", tr!("Very Slow")), Coord::new(status_line_row(), 49));
    } else if speed == 1 {
        put_string(&format!("{:<9.9}", tr!("Slow")), Coord::new(status_line_row(), 49));
    } else if speed == 0 {
        put_blanks(9, Coord::new(status_line_row(), 49));
    } else if speed == -1 {
        put_string(&format!("{:<9.9}", tr!("Fast")), Coord::new(status_line_row(), 49));
    } else {
        put_string(&format!("{:<9.9}", tr!("Very Fast")), Coord::new(status_line_row(), 49));
    }
}

pub fn print_character_study_instruction() {
    py().flags.status &= !config::player::status::PY_STUDY;

    if py().flags.new_spells_to_learn == 0 {
        put_blanks(5, Coord::new(status_line_row(), 59));
    } else {
        put_string(&format!("{:<5.5}", tr!("Study")), Coord::new(status_line_row(), 59));
    }
}

// Prints winner status on display -RAK-
pub fn print_character_winner() {
    if (game().noscore & 0x2) != 0 {
        if game().wizard_mode {
            put_string(&format!("{:<11.11}", tr!("Is wizard")), Coord::new(status_line_row() - 1, 0));
        } else {
            put_string(&format!("{:<11.11}", tr!("Was wizard")), Coord::new(status_line_row() - 1, 0));
        }
    } else if (game().noscore & 0x1) != 0 {
        put_string(&format!("{:<11.11}", tr!("Resurrected")), Coord::new(status_line_row() - 1, 0));
    } else if (game().noscore & 0x4) != 0 {
        put_string(&format!("{:<9.9}", tr!("Duplicate")), Coord::new(status_line_row() - 1, 0));
    } else if game().total_winner {
        put_string(&format!("{:<11.11}", tr!("*Winner*")), Coord::new(status_line_row() - 1, 0));
    }
}

// Prints character-screen info -RAK-
pub fn print_character_stats_block() {
    print_character_info_in_field(tr!(CHARACTER_RACES[py().misc.race_id as usize].name), Coord::new(2, STAT_COLUMN));
    print_character_info_in_field(tr!(CLASSES[py().misc.class_id as usize].title), Coord::new(3, STAT_COLUMN));
    print_character_info_in_field(&player_rank_title(), Coord::new(4, STAT_COLUMN));

    for i in 0..6 {
        display_character_stats(i);
    }

    print_header_number(&format!("{:<4.4}", tr!("LEV ")), py().misc.level as i32, Coord::new(13, STAT_COLUMN));
    print_header_long_number(&format!("{:<4.4}", tr!("EXP ")), py().misc.exp, Coord::new(14, STAT_COLUMN));
    print_header_number(&format!("{:<4.4}", tr!("MANA")), py().misc.current_mana as i32, Coord::new(15, STAT_COLUMN));
    print_header_number(&format!("{:<4.4}", tr!("MHP ")), py().misc.max_hp as i32, Coord::new(16, STAT_COLUMN));
    print_header_number(&format!("{:<4.4}", tr!("CHP ")), py().misc.current_hp as i32, Coord::new(17, STAT_COLUMN));
    print_header_number(&format!("{:<4.4}", tr!("AC  ")), py().misc.display_ac as i32, Coord::new(19, STAT_COLUMN));
    print_header_long_number(&format!("{:<4.4}", tr!("GOLD")), py().misc.au, Coord::new(20, STAT_COLUMN));
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

    put_string(&format!("{:<13.13}", tr!("Name        :")), Coord::new(2, 1));
    put_string(&format!("{:<13.13}", tr!("Race        :")), Coord::new(3, 1));
    put_string(&format!("{:<13.13}", tr!("Sex         :")), Coord::new(4, 1));
    put_string(&format!("{:<13.13}", tr!("Class       :")), Coord::new(5, 1));

    if !game().character_generated {
        return;
    }

    let name = py().misc.name.clone();
    put_string(&name, Coord::new(2, 15));
    put_string(tr!(CHARACTER_RACES[py().misc.race_id as usize].name), Coord::new(3, 15));
    put_string(player_get_gender_label(), Coord::new(4, 15));
    put_string(tr!(CLASSES[py().misc.class_id as usize].title), Coord::new(5, 15));
}

// Prints the following information on the screen. -JWT-
pub fn print_character_stats() {
    for i in 0..6 {
        let buf = stats_as_string(py().stats.used[i]);
        put_string(&format!("{:<6.6}", tr!(STAT_NAMES[i])), Coord::new(2 + i as i32, 61));
        put_string(&buf, Coord::new(2 + i as i32, 66));

        if py().stats.max[i] > py().stats.current[i] {
            let buf = stats_as_string(py().stats.max[i]);
            put_string(&buf, Coord::new(2 + i as i32, 73));
        }
    }

    print_header_number(&format!("{:<12.12}", tr!("+ To Hit    ")), py().misc.display_to_hit as i32, Coord::new(9, 1));
    print_header_number(&format!("{:<12.12}", tr!("+ To Damage ")), py().misc.display_to_damage as i32, Coord::new(10, 1));
    print_header_number(&format!("{:<12.12}", tr!("+ To AC     ")), py().misc.display_to_ac as i32, Coord::new(11, 1));
    print_header_number(&format!("{:<12.12}", tr!("  Total AC  ")), py().misc.display_ac as i32, Coord::new(12, 1));
}

// Returns a rating of x depending on y -JWT-
pub fn stat_rating(coord: Coord) -> &'static str {
    match coord.x / coord.y {
        -3..=-1 => tr!("Very Bad"),
        0 | 1 => tr!("Bad"),
        2 => tr!("Poor"),
        3 | 4 => tr!("Fair"),
        5 => tr!("Good"),
        6 => tr!("Very Good"),
        7 | 8 => tr!("Excellent"),
        _ => tr!("Superb"),
    }
}

// Prints age, height, weight, and SC -JWT-
pub fn print_character_vital_statistics() {
    print_header_number(&format!("{:<13.13}", tr!("Age          ")), py().misc.age as i32, Coord::new(2, 38));
    print_header_number(&format!("{:<13.13}", tr!("Height       ")), py().misc.height as i32, Coord::new(3, 38));
    print_header_number(&format!("{:<13.13}", tr!("Weight       ")), py().misc.weight as i32, Coord::new(4, 38));
    print_header_number(&format!("{:<13.13}", tr!("Social Class ")), py().misc.social_class as i32, Coord::new(5, 38));
}

// Prints the following information on the screen. -JWT-
pub fn print_character_level_experience() {
    print_header_long_number_7_spaces(&format!("{:<11.11}", tr!("Level      ")), py().misc.level as i32, Coord::new(9, 28));
    print_header_long_number_7_spaces(&format!("{:<11.11}", tr!("Experience ")), py().misc.exp, Coord::new(10, 28));
    print_header_long_number_7_spaces(&format!("{:<11.11}", tr!("Max Exp    ")), py().misc.max_exp, Coord::new(11, 28));

    if py().misc.level as usize >= PLAYER_MAX_LEVEL {
        let header = format!("{:<11.11}", tr!("Exp to Adv."));
        put_string_clear_to_eol(&format!("{}: *******", header), Coord::new(12, 28));
    } else {
        print_header_long_number_7_spaces(
            &format!("{:<11.11}", tr!("Exp to Adv.")),
            (py().base_exp_levels[py().misc.level as usize - 1] as i64 * py().misc.experience_factor as i64 / 100) as i32,
            Coord::new(12, 28),
        );
    }

    print_header_long_number_7_spaces(&format!("{:<11.11}", tr!("Gold       ")), py().misc.au, Coord::new(13, 28));
    print_header_number(&format!("{:<15.15}", tr!("Max Hit Points ")), py().misc.max_hp as i32, Coord::new(9, 52));
    print_header_number(&format!("{:<15.15}", tr!("Cur Hit Points ")), py().misc.current_hp as i32, Coord::new(10, 52));
    print_header_number(&format!("{:<15.15}", tr!("Max Mana       ")), py().misc.mana as i32, Coord::new(11, 52));
    print_header_number(&format!("{:<15.15}", tr!("Cur Mana       ")), py().misc.current_mana as i32, Coord::new(12, 52));
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

    let xinfra = tr_fmt!("{} feet", py().flags.see_infra * 10);

    put_string(tr!("(Miscellaneous Abilities)"), Coord::new(15, 25));
    put_string(&format!("{:<13.13}", tr!("Fighting    :")), Coord::new(16, 1));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(12, xbth))), Coord::new(16, 15));
    put_string(&format!("{:<13.13}", tr!("Bows/Throw  :")), Coord::new(17, 1));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(12, xbthb))), Coord::new(17, 15));
    put_string(&format!("{:<13.13}", tr!("Saving Throw:")), Coord::new(18, 1));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(6, xsave))), Coord::new(18, 15));

    put_string(&format!("{:<13.13}", tr!("Stealth     :")), Coord::new(16, 28));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(1, xstl))), Coord::new(16, 42));
    put_string(&format!("{:<13.13}", tr!("Disarming   :")), Coord::new(17, 28));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(8, xdis))), Coord::new(17, 42));
    put_string(&format!("{:<13.13}", tr!("Magic Device:")), Coord::new(18, 28));
    put_string(&format!("{:<13.13}", stat_rating(Coord::new(6, xdev))), Coord::new(18, 42));

    put_string(&format!("{:<13.13}", tr!("Perception  :")), Coord::new(16, 55));
    put_string(&format!("{:.11}", stat_rating(Coord::new(3, xfos))), Coord::new(16, 69));
    put_string(&format!("{:<13.13}", tr!("Searching   :")), Coord::new(17, 55));
    put_string(&format!("{:.11}", stat_rating(Coord::new(6, xsrh))), Coord::new(17, 69));
    put_string(&format!("{:<13.13}", tr!("Infra-Vision:")), Coord::new(18, 55));
    put_string(&format!("{:.11}", xinfra), Coord::new(18, 69));
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
    put_string_clear_to_eol(tr!("Enter your player's name  [press <RETURN> when finished]"), Coord::new(21, 2));

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
        put_string_clear_to_eol(tr!("<f>ile character description. <c>hange character name."), Coord::new(21, 2));

        match get_key_input() {
            'c' => {
                get_character_name();
                flag = true;
            }
            'f' => {
                put_string_clear_to_eol(tr!("File name:"), Coord::new(0, 0));

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
    put_string(tr!("Name"), Coord::new(1, col + 5));
    put_string(tr!("Lv Mana Fail"), Coord::new(1, col + 35));

    // only show the first 22 choices
    let number_of_choices = std::cmp::min(number_of_choices, 22);

    for i in 0..number_of_choices as usize {
        let spell_id = spell_ids[i];
        let spell = &MAGIC_SPELLS[py().misc.class_id as usize - 1][spell_id as usize];

        let p = if !comment {
            ""
        } else if (py().flags.spells_forgotten & (1u32 << spell_id)) != 0 {
            tr!(" forgotten")
        } else if (py().flags.spells_learnt & (1u32 << spell_id)) == 0 {
            tr!(" unknown")
        } else if (py().flags.spells_worked & (1u32 << spell_id)) == 0 {
            tr!(" untried")
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
            "  {}) {:<30.30}{:2} {:4} {:3}%{}",
            spell_char,
            tr!(SPELL_NAMES[(spell_id + consecutive_offset as i32) as usize]),
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

    let msg = tr_fmt!("Welcome to level {}.", py().misc.level);
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
