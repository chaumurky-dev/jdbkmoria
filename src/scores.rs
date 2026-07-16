// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Handle reading, writing, and displaying of high scores.

use std::fs::{File, OpenOptions};
use std::io::SeekFrom;

use crate::config;
use crate::data_player::{CHARACTER_RACES, CLASSES};
use crate::dungeon::dg;
use crate::game::{game, valid_game_version};
use crate::game_save::{close_fileptr, eof_hit, fileptr_seek, fileptr_stream_position, read_high_score, read_raw_byte_pub, save_high_score, set_fileptr, write_raw_byte};
use crate::player::{player_is_male, py, PLAYER_NAME_SIZE};
use crate::store_inventory::store_item_value;
use crate::types::Coord;
use crate::ui::ESCAPE;
use crate::ui_io::{clear_screen, erase_line, get_key_input, panic_save, print_message, put_string_clear_to_eol};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};
use crate::{tr, tr_fmt};

// High score file pointer -- modeled as the shared module-internal fileptr
// state in `game_save.rs`, matching the C code's `setFileptr(highscore_fp)`
// scheme (see game_save::set_fileptr / close_fileptr).

// HighScore is a score object used for saving to the high score file
// This structure is 64 bytes in size
#[derive(Debug, Clone)]
pub struct HighScore {
    pub points: i32,
    pub birth_date: i32,
    pub uid: i16,
    pub mhp: i16,
    pub chp: i16,
    pub dungeon_depth: u8,
    pub level: u8,
    pub deepest_dungeon_depth: u8,
    pub gender: u8,
    pub race: u8,
    pub character_class: u8,
    pub name: [u8; PLAYER_NAME_SIZE],
    pub died_from: [u8; 25],
}

impl HighScore {
    pub const fn empty() -> Self {
        HighScore {
            points: 0,
            birth_date: 0,
            uid: 0,
            mhp: 0,
            chp: 0,
            dungeon_depth: 0,
            level: 0,
            deepest_dungeon_depth: 0,
            gender: 0,
            race: 0,
            character_class: 0,
            name: [0; PLAYER_NAME_SIZE],
            died_from: [0; 25],
        }
    }
}

// Number of entries allowed in the score file.
pub const MAX_HIGH_SCORE_ENTRIES: u16 = 1000;

// On-disk byte length of a single high score record: the 1 byte encryption
// "robustness" byte written/read by save_high_score()/read_high_score(),
// plus the field bytes: points(4) + birth_date(4) + uid(2) + mhp(2) + chp(2)
// + dungeon_depth(1) + level(1) + deepest_dungeon_depth(1) + gender(1) +
// race(1) + character_class(1) + name(PLAYER_NAME_SIZE) + died_from(25).
// This must match the exact byte count emitted by `save_high_score()` -
// it is used to seek backwards by one record when shuffling entries.
const HIGH_SCORE_RECORD_SIZE: i64 = 1 + 4 + 4 + 2 + 2 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + PLAYER_NAME_SIZE as i64 + 25;

fn high_score_gender_label() -> u8 {
    if player_is_male() {
        b'M'
    } else {
        b'F'
    }
}

// Copies a string into a fixed-size, zero-padded byte buffer, mirroring the
// C code's `strcpy()` into a zero-initialized `HighScore_t` struct member.
//
// jdbkmoria extension: translated died_from/name text may contain multi-byte
// UTF-8 sequences, so the byte count that fits the buffer is walked back to
// the nearest char boundary before copying — a plain byte-index truncation
// could split a multi-byte character, leaving an invalid UTF-8 tail that
// `cstr()` would then discard entirely via `from_utf8().unwrap_or("")`.
fn copy_bytes_zero_padded(dest: &mut [u8], src: &str) {
    for b in dest.iter_mut() {
        *b = 0;
    }
    let max_len = dest.len().saturating_sub(1);
    let mut end = src.len().min(max_len);
    while end > 0 && !src.is_char_boundary(end) {
        end -= 1;
    }
    dest[..end].copy_from_slice(&src.as_bytes()[..end]);
}

// Reads a NUL-terminated string out of a fixed-size byte buffer.
fn cstr(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

// Strips a leading "a "/"an " article from the death description.
//
// NOTE: this faithfully ports a quirk in the original C++: it only checks
// whether the string starts with the letter 'a', not specifically for the
// article "a " or "an " - so a `character_died_from` string that happens to
// start with a literal 'a' that is not an article (e.g. "acid") would (as in
// the original) have its leading character(s) incorrectly stripped. Ported
// as-is for behavioral fidelity with the reference implementation.
fn strip_died_from_article(died_from: &str) -> String {
    let bytes = died_from.as_bytes();
    let mut idx = 0;

    if idx < bytes.len() && bytes[idx] == b'a' {
        idx += 1;
        if idx < bytes.len() && bytes[idx] == b'n' {
            idx += 1;
        }
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
    }

    String::from_utf8_lossy(&bytes[idx..]).into_owned()
}

// under unix, only allow one gender/race/class combo per person,
// on single user system, allow any number of entries, but try to
// prevent multiple entries per character by checking for case when
// birthdate/gender/race/class are the same, and game.character_died_from
// of score file entry is "(saved)"
fn is_duplicate_entry(new_entry: &HighScore, old_entry: &HighScore) -> bool {
    ((new_entry.uid != 0 && new_entry.uid == old_entry.uid) || (new_entry.uid == 0 && cstr(&old_entry.died_from) == "(saved)" && new_entry.birth_date == old_entry.birth_date))
        && new_entry.gender == old_entry.gender
        && new_entry.race == old_entry.race
        && new_entry.character_class == old_entry.character_class
}

// Enters a players name on the top twenty list -JWT-
pub fn record_new_high_score() {
    clear_screen();

    if game().noscore != 0 {
        return;
    }

    if *panic_save() {
        print_message(Some(tr!("Sorry, scores for games restored from panic save files are not saved.")));
        return;
    }

    let mut new_entry = HighScore {
        points: player_calculate_total_points(),
        birth_date: py().misc.date_of_birth,
        uid: 0, // NOTE: do we not want to use `getuid()`? -MRC-
        mhp: py().misc.max_hp,
        chp: py().misc.current_hp,
        dungeon_depth: dg().current_level as u8,
        level: py().misc.level as u8,
        deepest_dungeon_depth: py().misc.max_dungeon_depth as u8,
        gender: high_score_gender_label(),
        race: py().misc.race_id,
        character_class: py().misc.class_id,
        name: [0u8; PLAYER_NAME_SIZE],
        died_from: [0u8; 25],
    };
    copy_bytes_zero_padded(&mut new_entry.name, &py().misc.name);

    let died_from = strip_died_from_article(&game().character_died_from);
    copy_bytes_zero_padded(&mut new_entry.died_from, &died_from);

    let file = match OpenOptions::new().read(true).write(true).open(config::files::SCORES) {
        Ok(f) => f,
        Err(_) => {
            print_message(Some(&tr_fmt!("Error opening score file '{}'.", config::files::SCORES)));
            print_message(None);
            return;
        }
    };

    // Search file to find where to insert this character, if uid != 0 and
    // find same uid/gender/race/class combo then exit without saving this score.
    // Seek to the beginning of the file just to be safe.
    set_fileptr(file);
    fileptr_seek(SeekFrom::Start(0));

    // Read version numbers from the score file, and check for validity.
    let version_maj = read_raw_byte_pub();
    let version_min = read_raw_byte_pub();
    let patch_level = read_raw_byte_pub();

    // If this is a new score file, it should be empty.
    // Write the current version numbers to the score file.
    if eof_hit() {
        // Seek to the beginning of the file just to be safe.
        fileptr_seek(SeekFrom::Start(0));

        write_raw_byte(CURRENT_VERSION_MAJOR);
        write_raw_byte(CURRENT_VERSION_MINOR);
        write_raw_byte(CURRENT_VERSION_PATCH);
    } else if !valid_game_version(version_maj, version_min, patch_level) {
        // No need to print a message, a subsequent call to
        // show_scores_screen() will print a message.
        close_fileptr();
        return;
    }

    // set the shared fileptr's xor state; already installed via set_fileptr()

    let mut old_entry = HighScore::empty();

    let mut i: i32 = 0;
    let mut curpos = fileptr_stream_position();
    read_high_score(&mut old_entry);

    while !eof_hit() {
        if new_entry.points >= old_entry.points {
            break;
        }

        if is_duplicate_entry(&new_entry, &old_entry) {
            close_fileptr();
            return;
        }

        // only allow one thousand scores in the score file
        i += 1;
        if i >= MAX_HIGH_SCORE_ENTRIES as i32 {
            close_fileptr();
            return;
        }

        curpos = fileptr_stream_position();
        read_high_score(&mut old_entry);
    }

    if eof_hit() {
        // write out new_entry at end of file
        fileptr_seek(SeekFrom::Start(curpos));

        save_high_score(&new_entry);
    } else {
        let mut entry = new_entry.clone();

        while !eof_hit() {
            fileptr_seek(SeekFrom::Current(-HIGH_SCORE_RECORD_SIZE));

            save_high_score(&entry);

            if is_duplicate_entry(&new_entry, &old_entry) {
                break;
            }
            entry = old_entry.clone();

            curpos = fileptr_stream_position();
            read_high_score(&mut old_entry);
        }
        if eof_hit() {
            fileptr_seek(SeekFrom::Start(curpos));

            save_high_score(&entry);
        }
    }

    close_fileptr();
}

pub fn show_scores_screen() {
    let file = match File::open(config::files::SCORES) {
        Ok(f) => f,
        Err(_) => {
            print_message(Some(&tr_fmt!("Error opening score file '{}'.", config::files::SCORES)));
            print_message(None);
            return;
        }
    };

    set_fileptr(file);
    fileptr_seek(SeekFrom::Start(0));

    // Read version numbers from the score file, and check for validity.
    let version_maj = read_raw_byte_pub();
    let version_min = read_raw_byte_pub();
    let patch_level = read_raw_byte_pub();

    // If score data present, check if a valid game version
    if !eof_hit() && !valid_game_version(version_maj, version_min, patch_level) {
        print_message(Some(tr!("Sorry. This score file is from a different version of umoria.")));
        print_message(None);
        close_fileptr();
        return;
    }

    let mut score = HighScore::empty();
    read_high_score(&mut score);

    let mut rank = 1;

    while !eof_hit() {
        let mut i = 1;
        clear_screen();

        // Put twenty scores on each page, on lines 2 through 21.
        while !eof_hit() && i < 21 {
            let race_name = tr!(CHARACTER_RACES[score.race as usize].name);
            let class_title = tr!(CLASSES[score.character_class as usize].title);

            let msg = format!(
                "{:<4}{:>8} {:<19.19} {} {:<10.10} {:<7.7}{:>3} {:<22.22}",
                rank,
                score.points,
                cstr(&score.name),
                score.gender as char,
                race_name,
                class_title,
                score.level,
                cstr(&score.died_from),
            );

            i += 1;
            put_string_clear_to_eol(&msg, Coord::new(i, 0));
            rank += 1;
            read_high_score(&mut score);
        }
        // Header is built field-by-field (rather than as one translated
        // line) so that each column label stays clamped to the same
        // printed width as the data row's format spec above, regardless of
        // how long the translated word is -- jdbkmoria extension.
        let header = format!(
            "{:<4.4}{:>8.8} {:<16.16}{:>5.5} {:<10.10} {:<7.7}{:>3.3} {}",
            tr!("Rank"),
            tr!("Points"),
            tr!("Name"),
            tr!("Sex"),
            tr!("Race"),
            tr!("Class"),
            tr!("Lvl"),
            tr!("Killed By"),
        );
        put_string_clear_to_eol(&header, Coord::new(0, 0));
        erase_line(Coord::new(1, 0));
        put_string_clear_to_eol(tr!("[ press any key to continue ]"), Coord::new(23, 23));
        if get_key_input() == ESCAPE {
            break;
        }
    }

    close_fileptr();
}

// Calculates the total number of points earned -JWT-
pub fn player_calculate_total_points() -> i32 {
    let mut total = py().misc.max_exp + 100 * py().misc.max_dungeon_depth as i32;
    total += py().misc.au / 100;

    for item in py().inventory.iter() {
        total += store_item_value(item);
    }

    total += dg().current_level as i32 * 50;

    // Don't ever let the score decrease from one save to the next.
    if py().max_score > total {
        return py().max_score;
    }

    total
}
