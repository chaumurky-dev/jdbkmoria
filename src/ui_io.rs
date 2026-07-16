// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Terminal I/O code, uses the curses package

use pancurses::{Input, Window};

use crate::dungeon::dg;
use crate::game::game;
use crate::globals::RacyCell;
use crate::types::{Coord, MORIA_MESSAGE_SIZE};
use crate::ui::{ctrl_key, DELETE, ESCAPE, MESSAGE_HISTORY_SIZE, MSG_LINE};
use crate::tr;

static CURSES_ON: RacyCell<bool> = RacyCell::new(false);

// The main curses window, plus a spare window for saving the screen. -CJS-
static WINDOW: RacyCell<Option<Window>> = RacyCell::new(None);
static SAVE_SCREEN: RacyCell<Option<Window>> = RacyCell::new(None);

static EOF_FLAG: RacyCell<i32> = RacyCell::new(0); // Is used to signal EOF/HANGUP condition
static PANIC_SAVE: RacyCell<bool> = RacyCell::new(false); // True if playing from a panic save

static SCREEN_HAS_CHANGED: RacyCell<bool> = RacyCell::new(false);
static MESSAGE_READY_TO_PRINT: RacyCell<bool> = RacyCell::new(false);
static MESSAGES: RacyCell<[String; MESSAGE_HISTORY_SIZE]> =
    RacyCell::new([const { String::new() }; MESSAGE_HISTORY_SIZE]);
static LAST_MESSAGE_ID: RacyCell<i16> = RacyCell::new(0);

// jdbkmoria extension: everything drawn on screen is laid out in "local"
// coordinates (the sidebar/panel/status block, or a classic 80x24 static
// screen), and every primitive below adds one of two fixed offsets before
// the actual curses call, centering that local content within the real
// terminal. Both offsets are computed once in `terminal_initialize` from
// the real terminal size and never change afterward — only which one is
// active does, via `center_for_dungeon_view`/`center_for_static_screen`.
//
// The default is the dungeon-view offset (usually (0, 0), since the live
// game screen already auto-fits the terminal — see `terminal_initialize`)
// rather than the static offset, so a screen transition nobody remembered
// to mark as "static" just renders un-centered instead of mis-centered:
// the dungeon panel's own coordinates (already carrying their own
// per-level centering via `ui::coord_outside_panel_enlarged`'s
// row_prt/col_prt) must never be shifted by the smaller static offset, or
// the sidebar and panel — each only correct under their own offset — go
// visibly out of alignment with each other.
static DUNGEON_OFFSET: RacyCell<(i32, i32)> = RacyCell::new((0, 0));
static STATIC_OFFSET: RacyCell<(i32, i32)> = RacyCell::new((0, 0));
static USE_STATIC_OFFSET: RacyCell<bool> = RacyCell::new(false);

fn row_offset() -> i32 {
    if *USE_STATIC_OFFSET.get() { STATIC_OFFSET.get().0 } else { DUNGEON_OFFSET.get().0 }
}

fn col_offset() -> i32 {
    if *USE_STATIC_OFFSET.get() { STATIC_OFFSET.get().1 } else { DUNGEON_OFFSET.get().1 }
}

// Whether static-screen centering is currently active. A screen that can be
// entered from either context (e.g. help, reachable both mid-game and
// during character creation) should save this before switching to static
// centering and restore it afterward, rather than assuming which one to
// return to.
pub fn is_static_centered() -> bool {
    *USE_STATIC_OFFSET.get()
}

// Switches to centering a classic 80x24 screen (splash, character sheet,
// store, help, death, high scores, ...) within the terminal.
pub fn center_for_static_screen() {
    *USE_STATIC_OFFSET.get() = true;
}

// Switches back to the live dungeon screen's own centering (the default —
// see the comment on DUNGEON_OFFSET above for why).
pub fn center_for_dungeon_view() {
    *USE_STATIC_OFFSET.get() = false;
}

pub fn eof_flag() -> &'static mut i32 {
    EOF_FLAG.get()
}

pub fn panic_save() -> &'static mut bool {
    PANIC_SAVE.get()
}

pub fn screen_has_changed() -> &'static mut bool {
    SCREEN_HAS_CHANGED.get()
}

pub fn message_ready_to_print() -> &'static mut bool {
    MESSAGE_READY_TO_PRINT.get()
}

pub fn messages() -> &'static mut [String; MESSAGE_HISTORY_SIZE] {
    MESSAGES.get()
}

pub fn last_message_id() -> &'static mut i16 {
    LAST_MESSAGE_ID.get()
}

fn stdscr() -> &'static Window {
    WINDOW.get().as_ref().expect("curses not initialized")
}

// Set up the terminal into a suitable state -MRC-
fn moria_terminal_initialize() {
    pancurses::raw(); // disable control characters. I.e. Ctrl-C does not work!
    pancurses::noecho(); // do not echo typed characters
    pancurses::nonl(); // disable translation return/newline for detection of return key
    // disable curses keypad translation; escape sequences are decoded by
    // parse_escape_sequence() instead, which keeps the terminal out of
    // application keypad mode so the numpad keeps sending plain digits
    stdscr().keypad(false);

    *CURSES_ON.get() = true;
}

// initializes the terminal / curses routines
//
// jdbkmoria extension: `size_override` is an optional (cols, lines) pair
// from the `-W` CLI flag. When absent, the dungeon viewport auto-fits the
// actual terminal size. Either way it's clamped into
// [SCREEN_HEIGHT/WIDTH, MAX_HEIGHT/WIDTH] by `ui::set_view_size` — see
// PORTING.md's "Extensions beyond upstream" for why SCREEN_HEIGHT/WIDTH
// themselves are never touched.
pub fn terminal_initialize(size_override: Option<(i32, i32)>) -> bool {
    // the default ESC delay can be a second on some systems,
    // let's do something about that! (must be set before initscr)
    std::env::set_var("ESCDELAY", "50");

    // jdbkmoria extension: adopt the system locale so ncursesw renders
    // multi-byte UTF-8 (accented French text, locale digit glyphs)
    // correctly. Must happen before initscr. If the environment supplies
    // no UTF-8 locale (e.g. LANG=C), fall back to C.UTF-8 so translated
    // text still displays.
    unsafe {
        let empty = std::ffi::CString::new("").unwrap();
        libc::setlocale(libc::LC_ALL, empty.as_ptr());
        let codeset = libc::nl_langinfo(libc::CODESET);
        let is_utf8 = !codeset.is_null() && std::ffi::CStr::from_ptr(codeset).to_string_lossy().eq_ignore_ascii_case("UTF-8");
        if !is_utf8 {
            let c_utf8 = std::ffi::CString::new("C.UTF-8").unwrap();
            libc::setlocale(libc::LC_ALL, c_utf8.as_ptr());
        }
    }

    let window = pancurses::initscr();

    // Check we have enough screen. -CJS-
    if window.get_max_y() < 24 || window.get_max_x() < 80 {
        pancurses::endwin();
        println!("{}", tr!("Screen too small for moria."));
        return false;
    }

    // sidebar (13 cols) + a 1-col right margin (put_string/etc. never write
    // to the last column, see ui_io.rs's put_string) + message/status lines
    // (2 rows) are fixed chrome around the dungeon viewport; the rest of
    // the terminal is available. An override is clamped to what the
    // terminal actually has, so `-W` can only shrink from the auto-fit
    // size, never draw off-screen. At exactly 80x24 this yields the
    // classic 66x22 viewport, so the default 80x24 terminal is unaffected.
    let (req_cols, req_lines) = size_override.unwrap_or((window.get_max_x(), window.get_max_y()));
    let cols = req_cols.min(window.get_max_x());
    let lines = req_lines.min(window.get_max_y());
    crate::ui::set_view_size(lines - 2, cols - 14);

    // jdbkmoria extension: center both the live game screen's own footprint
    // (only visibly different from (0, 0) when `-W` or the MAX_HEIGHT/WIDTH
    // ceiling makes it smaller than the real terminal) and, separately, a
    // classic 80x24 static screen, within the real terminal. See the
    // DUNGEON_OFFSET/STATIC_OFFSET comment above for how these are used.
    let dungeon_footprint_h = crate::ui::view_height() + 2;
    let dungeon_footprint_w = crate::ui::view_width() + 14;
    *DUNGEON_OFFSET.get() = ((window.get_max_y() - dungeon_footprint_h) / 2, (window.get_max_x() - dungeon_footprint_w) / 2);
    *STATIC_OFFSET.get() = ((window.get_max_y() - 24) / 2, (window.get_max_x() - 80) / 2);

    let save_screen = pancurses::newwin(0, 0, 0, 0);

    *WINDOW.get() = Some(window);
    *SAVE_SCREEN.get() = Some(save_screen);

    moria_terminal_initialize();

    stdscr().clear();
    stdscr().refresh();

    true
}

// Put the terminal in the original mode. -CJS-
pub fn terminal_restore() {
    if !*CURSES_ON.get() {
        return;
    }

    // Dump any remaining buffer
    put_qio();

    // this moves curses to bottom left corner
    let lines = stdscr().get_max_y();
    stdscr().mv(lines - 1, 0);
    stdscr().refresh();

    // exit curses
    pancurses::endwin();

    *CURSES_ON.get() = false;
}

pub fn terminal_save_screen() {
    if let Some(save) = SAVE_SCREEN.get().as_ref() {
        stdscr().overwrite(save);
    }
}

pub fn terminal_restore_screen() {
    if let Some(save) = SAVE_SCREEN.get().as_ref() {
        save.overwrite(stdscr());
        stdscr().touch();
    }
}

pub fn terminal_bell_sound() {
    put_qio();

    // The player can turn off beeps if they find them annoying.
    if crate::config::options::options().error_beep_sound {
        pancurses::beep();
    }
}

// Dump the IO buffer to terminal -RAK-
pub fn put_qio() {
    // Let inventory_execute_command() know something has changed.
    *SCREEN_HAS_CHANGED.get() = true;

    stdscr().refresh();
}

// Flush the buffer -RAK-
pub fn flush_input_buffer() {
    if *EOF_FLAG.get() != 0 {
        return;
    }

    while check_for_non_blocking_key_press(0) {}
}

// Clears screen
pub fn clear_screen() {
    if *MESSAGE_READY_TO_PRINT.get() {
        print_message(None);
    }
    stdscr().clear();
}

pub fn clear_to_bottom(row: i32) {
    stdscr().mv(row + row_offset(), col_offset());
    stdscr().clrtobot();
}

// move cursor to a given y, x position
pub fn move_cursor(coord: Coord) {
    stdscr().mv(coord.y + row_offset(), coord.x + col_offset());
}

pub fn add_char(ch: char, coord: Coord) {
    stdscr().mvaddch(coord.y + row_offset(), coord.x + col_offset(), ch);
}

// Dump IO to buffer -RAK-
pub fn put_string(out_str: &str, coord: Coord) {
    let mut coord = coord;

    // truncate the string, to make sure that it won't go past right edge of screen.
    if coord.x > 79 {
        coord.x = 79;
    }

    let max_len = (79 - coord.x) as usize;
    let truncated: String = out_str.chars().take(max_len).collect();

    stdscr().mvaddstr(coord.y + row_offset(), coord.x + col_offset(), &truncated);
}

// Outputs a line to a given y, x position -RAK-
pub fn put_string_clear_to_eol(str: &str, coord: Coord) {
    if coord.y == MSG_LINE && *MESSAGE_READY_TO_PRINT.get() {
        print_message(None);
    }

    stdscr().mv(coord.y + row_offset(), coord.x + col_offset());
    stdscr().clrtoeol();
    put_string(str, coord);
}

// Clears given line of text -RAK-
pub fn erase_line(coord: Coord) {
    if coord.y == MSG_LINE && *MESSAGE_READY_TO_PRINT.get() {
        print_message(None);
    }

    stdscr().mv(coord.y + row_offset(), coord.x + col_offset());
    stdscr().clrtoeol();
}

// Moves the cursor to a given interpolated y, x position -RAK-
pub fn panel_move_cursor(coord: Coord) {
    // Real coords convert to screen positions
    let y = coord.y - dg().panel.row_prt;
    let x = coord.x - dg().panel.col_prt;

    stdscr().mv(y + row_offset(), x + col_offset());
}

// Outputs a char to a given interpolated y, x position -RAK-
// sign bit of a character used to indicate standout mode. -CJS
pub fn panel_put_tile(ch: char, coord: Coord) {
    // Real coords convert to screen positions
    let y = coord.y - dg().panel.row_prt;
    let x = coord.x - dg().panel.col_prt;

    stdscr().mvaddch(y + row_offset(), x + col_offset(), ch);
}

fn current_cursor_position() -> Coord {
    Coord::new(stdscr().get_cur_y(), stdscr().get_cur_x())
}

// message_line_print_message will print a line of text to the message line (0,0).
// first clearing the line of any text!
pub fn message_line_print_message(message: &str) {
    // save current cursor position (already offset-inclusive, see
    // current_cursor_position — restoring it further below needs no
    // additional offset)
    let coord = current_cursor_position();

    // move to beginning of message line, and clear it
    stdscr().mv(row_offset(), col_offset());
    stdscr().clrtoeol();

    // truncate message if it's too long!
    let truncated: String = message.chars().take(79).collect();

    stdscr().addstr(&truncated);

    // restore cursor to old position
    stdscr().mv(coord.y, coord.x);
}

// message_line_clear will delete all text from the message line (0,0).
// The current cursor position will be maintained.
pub fn message_line_clear() {
    // save current cursor position (already offset-inclusive, see
    // current_cursor_position — restoring it further below needs no
    // additional offset)
    let coord = current_cursor_position();

    // move to beginning of message line, and clear it
    stdscr().mv(row_offset(), col_offset());
    stdscr().clrtoeol();

    // restore cursor to old position
    stdscr().mv(coord.y, coord.x);
}

// Outputs message to top line of screen
// These messages are kept for later reference.
pub fn print_message(msg: Option<&str>) {
    let mut old_len = 0;
    let mut combine_messages = false;

    if *MESSAGE_READY_TO_PRINT.get() {
        old_len = MESSAGES.get()[*LAST_MESSAGE_ID.get() as usize].chars().count() as i32 + 1;

        // If the new message and the old message are short enough,
        // we want display them together on the same line.  So we
        // don't flush the old message in this case.

        let new_len = match msg {
            Some(m) => m.chars().count() as i32,
            None => 0,
        };

        if msg.is_none() || new_len + old_len + 2 >= 73 {
            // ensure that the complete -more- message is visible.
            if old_len > 73 {
                old_len = 73;
            }

            put_string(&format!(" {}", tr!("-more-")), Coord::new(MSG_LINE, old_len));

            loop {
                let key = get_key_input();
                if key == ' ' || key == ESCAPE || key == '\n' || key == '\r' {
                    break;
                }
            }
        } else {
            combine_messages = true;
        }
    }

    if !combine_messages {
        stdscr().mv(MSG_LINE + row_offset(), col_offset());
        stdscr().clrtoeol();
    }

    // Make the null string a special case. -CJS-

    let msg = match msg {
        None => {
            *MESSAGE_READY_TO_PRINT.get() = false;
            return;
        }
        Some(m) => m,
    };

    game().command_count = 0;
    *MESSAGE_READY_TO_PRINT.get() = true;

    // If the new message and the old message are short enough,
    // display them on the same line.

    if combine_messages {
        put_string(msg, Coord::new(MSG_LINE, old_len + 2));
        let last = &mut MESSAGES.get()[*LAST_MESSAGE_ID.get() as usize];
        last.push_str("  ");
        last.push_str(msg);
    } else {
        message_line_print_message(msg);

        let id = LAST_MESSAGE_ID.get();
        *id += 1;
        if *id as usize >= MESSAGE_HISTORY_SIZE {
            *id = 0;
        }

        let mut saved: String = msg.chars().take(MORIA_MESSAGE_SIZE - 1).collect();
        saved.shrink_to_fit();
        MESSAGES.get()[*id as usize] = saved;
    }
}

// Print a message so as not to interrupt a counted command. -CJS-
pub fn print_message_no_command_interrupt(msg: &str) {
    // Save command count value
    let count = game().command_count;

    print_message(Some(msg));

    // Restore count value
    game().command_count = count;
}

// A single keypress pushed back after peeking past an ESC that turned
// out not to start an escape sequence.
static PUSHED_BACK_KEY: RacyCell<Option<char>> = RacyCell::new(None);

// get_key_input() returns arrow/keypad keys as characters in the Unicode
// private use area, which never arrives as ordinary terminal input:
// value = base + direction (1-9, numpad layout), with separate bases for
// plain (walk) and shifted (run) keys.
const KEYPAD_WALK_BASE: u32 = 0xE000;
const KEYPAD_RUN_BASE: u32 = 0xE010;

// Decode a keypad character from get_key_input() into
// (direction 1-9, shift held).
pub fn keypad_direction(key: char) -> Option<(i32, bool)> {
    match key as u32 {
        v @ 0xE001..=0xE009 => Some(((v - KEYPAD_WALK_BASE) as i32, false)),
        v @ 0xE011..=0xE019 => Some(((v - KEYPAD_RUN_BASE) as i32, true)),
        _ => None,
    }
}

// Read one character of an escape sequence. The terminal sends a whole
// sequence at once, so a short timeout is enough to tell a lone ESC
// keypress from the start of a sequence.
fn get_sequence_char() -> Option<char> {
    stdscr().timeout(50);
    let input = stdscr().getch();
    stdscr().timeout(-1);

    match input {
        Some(Input::Character(ch)) => Some(ch),
        _ => None,
    }
}

enum EscapeKey {
    Escape,    // a lone ESC keypress
    Key(char), // a decoded arrow/keypad key
    Unknown,   // an unrecognized sequence, to be ignored
}

// Curses keypad() translation is left off (see moria_terminal_initialize),
// so arrow and keypad keys arrive as raw escape sequences. Parse the two
// xterm-style forms: ESC [ params final ("CSI") and ESC O final ("SS3").
fn parse_escape_sequence() -> EscapeKey {
    let Some(intro) = get_sequence_char() else {
        return EscapeKey::Escape;
    };

    if intro != '[' && intro != 'O' {
        // Not a sequence: keep the key for the next read.
        *PUSHED_BACK_KEY.get() = Some(intro);
        return EscapeKey::Escape;
    }

    // Accumulate parameter characters up to the final byte (0x40-0x7E).
    let mut params = String::new();
    let key_char = loop {
        let Some(ch) = get_sequence_char() else {
            return EscapeKey::Unknown; // sequence cut short
        };
        if ('@'..='~').contains(&ch) {
            break ch;
        }
        if params.len() >= 8 {
            return EscapeKey::Unknown; // implausibly long: bail out
        }
        params.push(ch);
    };

    match decode_keypad_sequence(key_char, &params) {
        Some(key) => EscapeKey::Key(key),
        None => EscapeKey::Unknown,
    }
}

// Map a parsed sequence onto a keypad direction character. Letter finals
// are the arrow/home/end/keypad keys (ESC [ A, ESC O A, ESC [ 1;2A, ...);
// a '~' final is the legacy encoding with the key number as the first
// parameter (ESC [ 5 ~ = page up, ESC [ 5;2 ~ = shift page up, ...).
fn decode_keypad_sequence(key_char: char, params: &str) -> Option<char> {
    let mut params = params.split(';');
    let first: u32 = params.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let modifier: u32 = params.next().and_then(|p| p.parse().ok()).unwrap_or(1);

    let direction = match key_char {
        'A' => 8, // up
        'B' => 2, // down
        'C' => 6, // right
        'D' => 4, // left
        'H' => 7, // home
        'F' => 1, // end
        'E' => 5, // keypad center
        '~' => match first {
            1 | 7 => 7, // home
            4 | 8 => 1, // end
            5 => 9,     // page up
            6 => 3,     // page down
            _ => return None,
        },
        _ => return None,
    };

    // The modifier parameter encodes the held modifier keys as a bitmask
    // plus one, with shift as the low bit.
    let shift = modifier.saturating_sub(1) & 1 == 1;

    let base = if shift { KEYPAD_RUN_BASE } else { KEYPAD_WALK_BASE };
    char::from_u32(base + direction)
}

// Returns a single character input from the terminal. -CJS-
//
// This silently consumes ^R to redraw the screen and reset the
// terminal, so that this operation can always be performed at
// any input prompt. get_key_input() never returns ^R.
//
// Arrow/keypad escape sequences are decoded into the private-use
// characters described above keypad_direction().
pub fn get_key_input() -> char {
    put_qio(); // Dump IO buffer
    game().command_count = 0; // Just to be safe -CJS-

    loop {
        let input = match PUSHED_BACK_KEY.get().take() {
            Some(key) => Some(Input::Character(key)),
            None => stdscr().getch(),
        };

        match input {
            None => {
                // EOF: avoid infinite loops while trying to call get_key_input()
                // for a -more- prompt.
                *MESSAGE_READY_TO_PRINT.get() = false;

                *EOF_FLAG.get() += 1;

                stdscr().refresh();

                if !game().character_generated || game().character_saved {
                    crate::game_death::end_game();
                }

                crate::player::player_disturb(1, 0);

                if *EOF_FLAG.get() > 100 {
                    // just in case, to make sure that the process eventually dies
                    *PANIC_SAVE.get() = true;

                    game().character_died_from = "(end of input: panic saved)".to_string();
                    if !crate::game_save::save_game() {
                        game().character_died_from = "panic: unexpected eof".to_string();
                        game().character_is_dead = true;
                    }
                    crate::game_death::end_game();
                }
                return ESCAPE;
            }
            Some(Input::Character(ch)) => {
                if ch == ESCAPE {
                    match parse_escape_sequence() {
                        EscapeKey::Escape => return ESCAPE,
                        EscapeKey::Key(key) => return key,
                        EscapeKey::Unknown => continue,
                    }
                }

                if ch != ctrl_key('R') {
                    return ch;
                }

                stdscr().refresh();
                moria_terminal_initialize();
            }
            // ignore non-character input (function keys, resize events, ...)
            Some(_) => {}
        }
    }
}

// Prompts (optional) and returns ord value of input char
// Function returns false if <ESCAPE> is input
pub fn get_command(prompt: &str, command: &mut char) -> bool {
    if !prompt.is_empty() {
        put_string_clear_to_eol(prompt, Coord::new(0, 0));
    }
    *command = get_key_input();

    message_line_clear();

    *command != ESCAPE
}

// NOTE: currently this just wraps the get_command() function, but better defines the different usages. -MRC-
pub fn get_tile_character(prompt: &str, command: &mut char) -> bool {
    get_command(prompt, command)
}

// NOTE: currently this just wraps the get_command() function, but better defines the different usages. -MRC-
pub fn get_menu_item_id(prompt: &str, command: &mut char) -> bool {
    get_command(prompt, command)
}

// Gets a string terminated by <RETURN>
// Function returns false if <ESCAPE> is input
pub fn get_string_input(in_str: &mut String, coord: Coord, slen: i32) -> bool {
    let mut coord = coord;

    stdscr().mv(coord.y + row_offset(), coord.x + col_offset());

    for _ in 0..slen {
        stdscr().addch(' ');
    }

    stdscr().mv(coord.y + row_offset(), coord.x + col_offset());

    let start_col = coord.x;
    let mut end_col = coord.x + slen - 1;

    if end_col > 79 {
        end_col = 79;
    }

    let mut buffer = String::new();

    let mut flag = false;
    let mut aborted = false;

    while !flag && !aborted {
        let key = get_key_input();
        match key {
            ESCAPE => aborted = true,
            k if k == ctrl_key('J') || k == ctrl_key('M') => flag = true,
            k if k == DELETE || k == ctrl_key('H') => {
                if coord.x > start_col {
                    coord.x -= 1;
                    put_string(" ", coord);
                    move_cursor(coord);
                    buffer.pop();
                }
            }
            key => {
                if !key.is_ascii_graphic() && key != ' ' || coord.x > end_col {
                    terminal_bell_sound();
                } else {
                    stdscr().mvaddch(coord.y + row_offset(), coord.x + col_offset(), key);
                    buffer.push(key);
                    coord.x += 1;
                }
            }
        }
    }

    if aborted {
        return false;
    }

    // Remove trailing blanks
    while buffer.ends_with(' ') {
        buffer.pop();
    }

    *in_str = buffer;

    true
}

// Used to verify a choice - user gets the chance to abort choice. -CJS-
pub fn get_input_confirmation(prompt: &str) -> bool {
    get_input_confirmation_with_abort(0, prompt) == 1
}

// Used to verify a choice, with the option of aborting (useful for "drop all items")
// and with the option of setting the column for displaying the prompt.
pub fn get_input_confirmation_with_abort(column: i32, prompt: &str) -> i32 {
    put_string_clear_to_eol(prompt, Coord::new(0, column));

    let x = stdscr().get_cur_x();

    if x > 73 + col_offset() {
        stdscr().mv(row_offset(), 73 + col_offset());
    }

    stdscr().addstr(format!(" {}", tr!("[y/n]")));

    let mut key = ' ';
    while key == ' ' {
        key = get_key_input();
    }

    message_line_clear();

    // jdbkmoria extension: match the yes/no letters shown by the locale's
    // "[y/n]" translation (e.g. fr_CA displays "[o/n]" for "oui"/"non").
    let def = crate::locale::locale();
    let key = key.to_ascii_uppercase();
    if key == def.no_key {
        0
    } else if key == def.yes_key {
        1
    } else {
        -1
    }
}

// Pauses for user response before returning -RAK-
pub fn wait_for_continue_key(line_number: i32) {
    put_string_clear_to_eol(tr!("[ press any key to continue ]"), Coord::new(line_number, 23));
    get_key_input();
    erase_line(Coord::new(line_number, 0));
}

// Provides for a timeout on input. Does a non-blocking read, consuming the data if
// any, and then returns true if data was read, false otherwise.
pub fn check_for_non_blocking_key_press(microseconds: i32) -> bool {
    let timeout_ms = std::cmp::max(microseconds / 1000, 8);

    stdscr().timeout(timeout_ms);
    let result = stdscr().getch();
    stdscr().timeout(-1);

    result.is_some()
}

// Find a default user name from the system.
pub fn get_default_player_name() -> String {
    // Gotta have some name
    let default_name = "X".to_string();

    for var in ["USER", "LOGNAME", "USERNAME"] {
        if let Ok(name) = std::env::var(var) {
            if !name.is_empty() {
                return name;
            }
        }
    }

    default_name
}

// expands a tilde at the beginning of a file name to the user's home directory
pub fn tilde(file: &str) -> Option<String> {
    if let Some(rest) = file.strip_prefix('~') {
        // `~user` form: only the current user's home is supported here
        let (user, remainder) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, ""),
        };

        if user.is_empty() {
            if let Ok(home) = std::env::var("HOME") {
                return Some(format!("{}{}", home, remainder));
            }
        }

        return None;
    }

    Some(file.to_string())
}

// open a file just as does File::open, but allow a leading ~ to specify a home directory
pub fn tfopen_read(file: &str) -> std::io::Result<std::fs::File> {
    match tilde(file) {
        Some(expanded) => std::fs::File::open(expanded),
        None => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
    }
}

// Check user permissions on Unix based systems.
// The original relinquished setuid privileges; running setuid is not
// supported by this port, so there is nothing to do here.
pub fn check_file_permissions() -> bool {
    true
}
