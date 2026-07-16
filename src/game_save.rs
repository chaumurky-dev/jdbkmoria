// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Save and restore games and monster memory info

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::config;
use crate::dungeon::{dg, MAX_HEIGHT, MAX_WIDTH};
use crate::{tr, tr_fmt};
use crate::game::{game, is_current_game_version, random_number, valid_game_version, LEVEL_MAX_OBJECTS};
use crate::globals::RacyCell;
use crate::helpers::get_current_unix_time;
use crate::identification::objects_identified;
use crate::inventory::{Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE};
use crate::monster::{monster_multiply_total, monsters, next_free_monster_id, Monster, MON_MAX_CREATURES, MON_TOTAL_ALLOCATIONS};
use crate::paintings::{painting_from_legacy_save, painting_from_save, painting_kind_to_u8, paintings, MAX_PAINTINGS};
use crate::player::py;
use crate::recall_data::creature_recall;
use crate::scores::HighScore;
use crate::store_data::{stores, STORE_MAX_DISCRETE_ITEMS};
use crate::treasure::missiles_counter;
use crate::types::Coord;
use crate::ui_io::{
    clear_screen, eof_flag, get_input_confirmation, get_string_input, last_message_id, messages, panic_save, print_message,
    put_qio, put_string, put_string_clear_to_eol,
};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

// This save package was brought to by                -JWT-
// and                                                -RAK-
// and has been completely rewritten for UNIX by      -JEW-
// and has been completely rewritten again by         -CJS-
// and completely rewritten again! for portability by -JEW-

// these are used for the save file, to avoid having to pass them to every procedure
static FILEPTR: RacyCell<Option<File>> = RacyCell::new(None);
static XOR_BYTE: RacyCell<u8> = RacyCell::new(0);
static FROM_SAVE_FILE: RacyCell<bool> = RacyCell::new(false); // can overwrite old save file when save
static START_TIME: RacyCell<u32> = RacyCell::new(0); // time that play started
static WRITE_ERROR: RacyCell<bool> = RacyCell::new(false);
static EOF_HIT: RacyCell<bool> = RacyCell::new(false);

// ------------------------------------------------------------------
// Shared file pointer plumbing, used by both the save-game routines
// below, and by scores.rs (for the high score file), matching the
// C code's `setFileptr(FILE*)` and shared `fileptr`/`xor_byte` globals.
// ------------------------------------------------------------------

// set the local fileptr to the score file fileptr
pub fn set_fileptr(file: File) {
    *FILEPTR.get() = Some(file);
    *EOF_HIT.get() = false;
}

// closes (drops) the current fileptr, mirroring fclose()
pub fn close_fileptr() {
    *FILEPTR.get() = None;
}

pub fn fileptr_seek(pos: SeekFrom) -> bool {
    match FILEPTR.get().as_mut() {
        Some(f) => {
            let ok = f.seek(pos).is_ok();
            if ok {
                // fseek() clears the end-of-file indicator in C stdio
                *EOF_HIT.get() = false;
            }
            ok
        }
        None => false,
    }
}

pub fn fileptr_stream_position() -> u64 {
    match FILEPTR.get().as_mut() {
        Some(f) => f.stream_position().unwrap_or(0),
        None => 0,
    }
}

pub fn eof_hit() -> bool {
    *EOF_HIT.get()
}

// get_byte reads a single byte from a file, without any xor_byte encryption
// Mirrors `(uint8_t) (getc(fileptr) & 0xFF)`; returns 0xFF at EOF, matching
// the cast of EOF (-1) to uint8_t.
fn get_byte() -> u8 {
    read_raw_byte().unwrap_or(0xFF)
}

fn read_raw_byte() -> Option<u8> {
    let f = FILEPTR.get().as_mut()?;
    let mut buf = [0u8; 1];
    match f.read(&mut buf) {
        Ok(1) => Some(buf[0]),
        _ => {
            *EOF_HIT.get() = true;
            None
        }
    }
}

fn write_raw_byte_impl(byte: u8) {
    match FILEPTR.get().as_mut() {
        Some(f) => {
            if f.write_all(&[byte]).is_err() {
                *WRITE_ERROR.get() = true;
            }
        }
        None => *WRITE_ERROR.get() = true,
    }
}

// Raw (non-xor) byte write, used directly by scores.rs to write the
// initial version header of a brand new score file (mirrors direct
// `putc()` calls in recordNewHighScore()).
pub fn write_raw_byte(byte: u8) {
    write_raw_byte_impl(byte);
}

// Raw (non-xor) byte read, used directly by scores.rs to peek at the
// version header of the score file (mirrors direct `getc()` calls).
pub fn read_raw_byte_pub() -> u8 {
    get_byte()
}

fn wr_bool(value: bool) {
    wr_byte(value as u8);
}

fn wr_byte(value: u8) {
    let xb = XOR_BYTE.get();
    *xb ^= value;
    write_raw_byte_impl(*xb);
}

fn wr_short(value: u16) {
    wr_byte((value & 0xFF) as u8);
    wr_byte(((value >> 8) & 0xFF) as u8);
}

fn wr_long(value: u32) {
    wr_byte((value & 0xFF) as u8);
    wr_byte(((value >> 8) & 0xFF) as u8);
    wr_byte(((value >> 16) & 0xFF) as u8);
    wr_byte(((value >> 24) & 0xFF) as u8);
}

fn wr_bytes(value: &[u8]) {
    for &b in value {
        wr_byte(b);
    }
}

fn wr_string(value: &str) {
    for &b in value.as_bytes() {
        wr_byte(b);
    }
    wr_byte(0);
}

fn wr_shorts(value: &[u16]) {
    for &v in value {
        wr_short(v);
    }
}

fn wr_item(item: &Inventory) {
    wr_short(item.id);
    wr_byte(item.special_name_id);
    wr_string(&item.inscription_str());
    wr_long(item.flags);
    wr_byte(item.category_id);
    wr_byte(item.sprite);
    wr_short(item.misc_use as u16);
    wr_long(item.cost as u32);
    wr_byte(item.sub_category_id);
    wr_byte(item.items_count);
    wr_short(item.weight);
    wr_short(item.to_hit as u16);
    wr_short(item.to_damage as u16);
    wr_short(item.ac as u16);
    wr_short(item.to_ac as u16);
    wr_byte(item.damage.dice);
    wr_byte(item.damage.sides);
    wr_byte(item.depth_first_found);
    wr_byte(item.identification);
}

fn wr_monster(monster: &Monster) {
    wr_short(monster.hp as u16);
    wr_short(monster.sleep_count as u16);
    wr_short(monster.speed as u16);
    wr_short(monster.creature_id);
    wr_byte(monster.pos.y as u8);
    wr_byte(monster.pos.x as u8);
    wr_byte(monster.distance_from_player);
    wr_bool(monster.lit);
    wr_byte(monster.stunned_amount);
    wr_byte(monster.confused_amount);
}

fn rd_bool() -> bool {
    rd_byte() != 0
}

fn rd_byte() -> u8 {
    let c = get_byte();
    let xb = XOR_BYTE.get();
    let decoded = c ^ *xb;
    *xb = c;
    decoded
}

fn rd_short() -> u16 {
    let c = get_byte();
    let mut decoded: u16 = (c ^ *XOR_BYTE.get()) as u16;
    *XOR_BYTE.get() = get_byte();
    decoded |= ((c ^ *XOR_BYTE.get()) as u16) << 8;
    decoded
}

fn rd_long() -> u32 {
    let mut c = get_byte();
    let mut decoded: u32 = (c ^ *XOR_BYTE.get()) as u32;

    *XOR_BYTE.get() = get_byte();
    decoded |= ((c ^ *XOR_BYTE.get()) as u32) << 8;

    c = get_byte();
    decoded |= ((c ^ *XOR_BYTE.get()) as u32) << 16;

    *XOR_BYTE.get() = get_byte();
    decoded |= ((c ^ *XOR_BYTE.get()) as u32) << 24;

    decoded
}

fn rd_bytes(value: &mut [u8]) {
    for b in value.iter_mut() {
        let c = get_byte();
        *b = c ^ *XOR_BYTE.get();
        *XOR_BYTE.get() = c;
    }
}

fn rd_string() -> String {
    let mut bytes = Vec::new();
    loop {
        let c = get_byte();
        let decoded = c ^ *XOR_BYTE.get();
        *XOR_BYTE.get() = c;
        if decoded == 0 {
            break;
        }
        bytes.push(decoded);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn rd_shorts(value: &mut [u16]) {
    for v in value.iter_mut() {
        *v = rd_short();
    }
}

fn rd_item(item: &mut Inventory) {
    item.id = rd_short();
    item.special_name_id = rd_byte();
    item.set_inscription(&rd_string());
    item.flags = rd_long();
    item.category_id = rd_byte();
    item.sprite = rd_byte();
    item.misc_use = rd_short() as i16;
    item.cost = rd_long() as i32;
    item.sub_category_id = rd_byte();
    item.items_count = rd_byte();
    item.weight = rd_short();
    item.to_hit = rd_short() as i16;
    item.to_damage = rd_short() as i16;
    item.ac = rd_short() as i16;
    item.to_ac = rd_short() as i16;
    item.damage.dice = rd_byte();
    item.damage.sides = rd_byte();
    item.depth_first_found = rd_byte();
    item.identification = rd_byte();
}

fn rd_monster(monster: &mut Monster) {
    monster.hp = rd_short() as i16;
    monster.sleep_count = rd_short() as i16;
    monster.speed = rd_short() as i16;
    monster.creature_id = rd_short();
    monster.pos.y = rd_byte() as i32;
    monster.pos.x = rd_byte() as i32;
    monster.distance_from_player = rd_byte();
    monster.lit = rd_bool();
    monster.stunned_amount = rd_byte();
    monster.confused_amount = rd_byte();
}

// functions called from scores.rs to implement the score file, sharing
// the xor_byte/fileptr encoding used by the save game format.

pub fn save_high_score(score: &HighScore) {
    // Save the encryption byte for robustness.
    wr_byte(*XOR_BYTE.get());

    wr_long(score.points as u32);
    wr_long(score.birth_date as u32);
    wr_short(score.uid as u16);
    wr_short(score.mhp as u16);
    wr_short(score.chp as u16);
    wr_byte(score.dungeon_depth);
    wr_byte(score.level);
    wr_byte(score.deepest_dungeon_depth);
    wr_byte(score.gender);
    wr_byte(score.race);
    wr_byte(score.character_class);
    wr_bytes(&score.name);
    wr_bytes(&score.died_from);
}

pub fn read_high_score(score: &mut HighScore) {
    // Read the encryption byte.
    *XOR_BYTE.get() = get_byte();

    score.points = rd_long() as i32;
    score.birth_date = rd_long() as i32;
    score.uid = rd_short() as i16;
    score.mhp = rd_short() as i16;
    score.chp = rd_short() as i16;
    score.dungeon_depth = rd_byte();
    score.level = rd_byte();
    score.deepest_dungeon_depth = rd_byte();
    score.gender = rd_byte();
    score.race = rd_byte();
    score.character_class = rd_byte();
    rd_bytes(&mut score.name);
    rd_bytes(&mut score.died_from);
}

// Set up prior to actual save, do the save, then clean up
//
// clippy::if_same_then_else fires because two of the `should_prompt` arms
// below both evaluate to `true`, but the conditions guarding them
// (a path-exists check and an interactive `get_input_confirmation()`
// prompt) are deliberately distinct side-effecting checks mirroring the
// original C control flow; merging them would change which of them runs.
#[allow(clippy::if_same_then_else)]
pub fn save_game() -> bool {
    loop {
        if save_char(&config::files::save_game()) {
            return true;
        }

        let output = tr_fmt!("Save file '{}' fails.", config::files::save_game());
        print_message(Some(&output));

        let path_exists = Path::new(&config::files::save_game()).exists();
        let mut unlink_failed = false;

        let should_prompt = if !path_exists {
            true
        } else if !get_input_confirmation(tr!("File exists. Delete old save file?")) {
            true
        } else if std::fs::remove_file(config::files::save_game()).is_err() {
            unlink_failed = true;
            true
        } else {
            false
        };

        if should_prompt {
            if unlink_failed {
                let output = tr_fmt!("Can't delete '{}'", config::files::save_game());
                print_message(Some(&output));
            }

            put_string_clear_to_eol(tr!("New Save file [ESC to give up]:"), Coord::new(0, 0));
            let mut input = String::new();
            if !get_string_input(&mut input, Coord::new(0, 31), 45) {
                return false;
            }
            if !input.is_empty() {
                config::files::set_save_game(&input);
            }
        }

        let output = tr_fmt!("Saving with '{}'...", config::files::save_game());
        put_string_clear_to_eol(&output, Coord::new(0, 0));
    }
}

fn flush_and_check() -> bool {
    let ok = !*WRITE_ERROR.get();
    let flush_ok = match FILEPTR.get().as_mut() {
        Some(f) => f.flush().is_ok(),
        None => false,
    };
    ok && flush_ok
}

fn write_save_data() -> bool {
    // clear the game.character_is_dead flag when creating a HANGUP save file,
    // so that player can see tombstone when restart
    if *eof_flag() != 0 {
        game().character_is_dead = false;
    }

    let mut l: u32 = 0;

    {
        let options = config::options::options();
        if options.run_cut_corners {
            l |= 0x1;
        }
        if options.run_examine_corners {
            l |= 0x2;
        }
        if options.run_print_self {
            l |= 0x4;
        }
        if options.find_bound {
            l |= 0x8;
        }
        if options.prompt_to_pickup {
            l |= 0x10;
        }
        if options.use_roguelike_keys {
            l |= 0x20;
        }
        if options.show_inventory_weights {
            l |= 0x40;
        }
        if options.highlight_seams {
            l |= 0x80;
        }
        if options.run_ignore_doors {
            l |= 0x100;
        }
        if options.error_beep_sound {
            l |= 0x200;
        }
        if options.display_counts {
            l |= 0x400;
        }
    }
    if game().character_is_dead {
        // Sign bit
        l |= 0x8000_0000;
    }
    if game().total_winner {
        l |= 0x4000_0000;
    }

    for i in 0..MON_MAX_CREATURES {
        let r = creature_recall()[i];
        if r.movement != 0 || r.defenses != 0 || r.kills != 0 || r.spells != 0 || r.deaths != 0 || r.attacks[0] != 0 || r.attacks[1] != 0 || r.attacks[2] != 0 || r.attacks[3] != 0 {
            wr_short(i as u16);
            wr_long(r.movement);
            wr_long(r.spells);
            wr_short(r.kills);
            wr_short(r.deaths);
            wr_short(r.defenses);
            wr_byte(r.wake);
            wr_byte(r.ignore);
            wr_bytes(&r.attacks);
        }
    }

    // sentinel to indicate no more monster info
    wr_short(0xFFFF);

    wr_long(l);

    wr_string(&py().misc.name);
    wr_bool(py().misc.gender);
    wr_long(py().misc.au as u32);
    wr_long(py().misc.max_exp as u32);
    wr_long(py().misc.exp as u32);
    wr_short(py().misc.exp_fraction);
    wr_short(py().misc.age);
    wr_short(py().misc.height);
    wr_short(py().misc.weight);
    wr_short(py().misc.level);
    wr_short(py().misc.max_dungeon_depth);
    wr_short(py().misc.chance_in_search as u16);
    wr_short(py().misc.fos as u16);
    wr_short(py().misc.bth as u16);
    wr_short(py().misc.bth_with_bows as u16);
    wr_short(py().misc.mana as u16);
    wr_short(py().misc.max_hp as u16);
    wr_short(py().misc.plusses_to_hit as u16);
    wr_short(py().misc.plusses_to_damage as u16);
    wr_short(py().misc.ac as u16);
    wr_short(py().misc.magical_ac as u16);
    wr_short(py().misc.display_to_hit as u16);
    wr_short(py().misc.display_to_damage as u16);
    wr_short(py().misc.display_ac as u16);
    wr_short(py().misc.display_to_ac as u16);
    wr_short(py().misc.disarm as u16);
    wr_short(py().misc.saving_throw as u16);
    wr_short(py().misc.social_class as u16);
    wr_short(py().misc.stealth_factor as u16);
    wr_byte(py().misc.class_id);
    wr_byte(py().misc.race_id);
    wr_byte(py().misc.hit_die);
    wr_byte(py().misc.experience_factor);
    wr_short(py().misc.current_mana as u16);
    wr_short(py().misc.current_mana_fraction);
    wr_short(py().misc.current_hp as u16);
    wr_short(py().misc.current_hp_fraction);
    for entry in py().misc.history.iter() {
        wr_string(entry);
    }

    wr_bytes(&py().stats.max);
    wr_bytes(&py().stats.current);
    {
        let modified: [u16; 6] = std::array::from_fn(|i| py().stats.modified[i] as u16);
        wr_shorts(&modified);
    }
    wr_bytes(&py().stats.used);

    wr_long(py().flags.status);
    wr_short(py().flags.rest as u16);
    wr_short(py().flags.blind as u16);
    wr_short(py().flags.paralysis as u16);
    wr_short(py().flags.confused as u16);
    wr_short(py().flags.food as u16);
    wr_short(py().flags.food_digested as u16);
    wr_short(py().flags.protection as u16);
    wr_short(py().flags.speed as u16);
    wr_short(py().flags.fast as u16);
    wr_short(py().flags.slow as u16);
    wr_short(py().flags.afraid as u16);
    wr_short(py().flags.poisoned as u16);
    wr_short(py().flags.image as u16);
    wr_short(py().flags.protect_evil as u16);
    wr_short(py().flags.invulnerability as u16);
    wr_short(py().flags.heroism as u16);
    wr_short(py().flags.super_heroism as u16);
    wr_short(py().flags.blessed as u16);
    wr_short(py().flags.heat_resistance as u16);
    wr_short(py().flags.cold_resistance as u16);
    wr_short(py().flags.detect_invisible as u16);
    wr_short(py().flags.word_of_recall as u16);
    wr_short(py().flags.see_infra as u16);
    wr_short(py().flags.timed_infra as u16);
    wr_bool(py().flags.see_invisible);
    wr_bool(py().flags.teleport);
    wr_bool(py().flags.free_action);
    wr_bool(py().flags.slow_digest);
    wr_bool(py().flags.aggravate);
    wr_bool(py().flags.resistant_to_fire);
    wr_bool(py().flags.resistant_to_cold);
    wr_bool(py().flags.resistant_to_acid);
    wr_bool(py().flags.regenerate_hp);
    wr_bool(py().flags.resistant_to_light);
    wr_bool(py().flags.free_fall);
    wr_bool(py().flags.sustain_str);
    wr_bool(py().flags.sustain_int);
    wr_bool(py().flags.sustain_wis);
    wr_bool(py().flags.sustain_con);
    wr_bool(py().flags.sustain_dex);
    wr_bool(py().flags.sustain_chr);
    wr_bool(py().flags.confuse_monster);
    wr_byte(py().flags.new_spells_to_learn);

    wr_short(*missiles_counter() as u16);
    wr_long(dg().game_turn as u32);
    wr_short(py().pack.unique_items as u16);
    for i in 0..py().pack.unique_items as usize {
        wr_item(&py().inventory[i]);
    }
    for i in (PlayerEquipment::Wield as usize)..PLAYER_INVENTORY_SIZE {
        wr_item(&py().inventory[i]);
    }
    wr_short(py().pack.weight as u16);
    wr_short(py().equipment_count as u16);
    wr_long(py().flags.spells_learnt);
    wr_long(py().flags.spells_worked);
    wr_long(py().flags.spells_forgotten);
    wr_bytes(&py().flags.spells_learned_order);
    wr_bytes(&objects_identified()[..]);
    wr_long(game().magic_seed);
    wr_long(game().town_seed);
    wr_short(*last_message_id() as u16);
    for message in messages().iter() {
        wr_string(message);
    }

    // this indicates 'cheating' if it is a one
    wr_short(*panic_save() as u16);
    wr_short(game().total_winner as u16);
    wr_short(game().noscore as u16);
    wr_shorts(&py().base_hp_levels);

    for store in stores().iter() {
        wr_long(store.turns_left_before_closing as u32);
        wr_short(store.insults_counter as u16);
        wr_byte(store.owner_id);
        wr_byte(store.unique_items_counter);
        wr_short(store.good_purchases);
        wr_short(store.bad_purchases);
        for j in 0..store.unique_items_counter as usize {
            wr_long(store.inventory[j].cost as u32);
            wr_item(&store.inventory[j].item);
        }
    }

    // save the current time in the save file
    let mut l = get_current_unix_time();

    if l < *START_TIME.get() {
        // someone is messing with the clock!,
        // assume that we have been playing for 1 day
        l = START_TIME.get().wrapping_add(86400);
    }
    wr_long(l);

    // put game.character_died_from string in save file
    wr_string(&game().character_died_from);

    // put the max_score in the save file
    let l = crate::scores::player_calculate_total_points() as u32;
    wr_long(l);

    // put the date_of_birth in the save file
    wr_long(py().misc.date_of_birth as u32);

    // only level specific info follows, this allows characters to be
    // resurrected, the dungeon level info is not needed for a resurrection
    if game().character_is_dead {
        return flush_and_check();
    }

    wr_short(dg().current_level as u16);
    wr_short(py().pos.y as u16);
    wr_short(py().pos.x as u16);
    wr_short(*monster_multiply_total() as u16);
    wr_short(dg().height as u16);
    wr_short(dg().width as u16);
    wr_short(dg().panel.max_rows as u16);
    wr_short(dg().panel.max_cols as u16);

    for i in 0..(MAX_HEIGHT as usize) {
        for j in 0..(MAX_WIDTH as usize) {
            if dg().floor[i][j].creature_id != 0 {
                wr_byte(i as u8);
                wr_byte(j as u8);
                wr_byte(dg().floor[i][j].creature_id);
            }
        }
    }

    // marks end of creature_id info
    wr_byte(0xFF);

    for i in 0..(MAX_HEIGHT as usize) {
        for j in 0..(MAX_WIDTH as usize) {
            if dg().floor[i][j].treasure_id != 0 {
                wr_byte(i as u8);
                wr_byte(j as u8);
                wr_byte(dg().floor[i][j].treasure_id);
            }
        }
    }

    // marks end of treasure_id info
    wr_byte(0xFF);

    // must set counter to zero, note that code may write out two bytes unnecessarily
    let mut count: i32 = 0;
    let mut prev_char: u8 = 0;

    for row in dg().floor.iter() {
        for tile in row.iter() {
            let char_tmp: u8 = tile.feature_id
                | ((tile.perma_lit_room as u8) << 4)
                | ((tile.field_mark as u8) << 5)
                | ((tile.permanent_light as u8) << 6)
                | ((tile.temporary_light as u8) << 7);

            if char_tmp != prev_char || count == 255 {
                wr_byte(count as u8);
                wr_byte(prev_char);
                prev_char = char_tmp;
                count = 1;
            } else {
                count += 1;
            }
        }
    }

    // save last entry
    wr_byte(count as u8);
    wr_byte(prev_char);

    wr_short(game().treasure.current_id as u16);
    for i in (config::treasure::MIN_TREASURE_LIST_ID as usize)..(game().treasure.current_id as usize) {
        wr_item(&game().treasure.list[i]);
    }
    wr_short(*next_free_monster_id() as u16);
    for i in (config::monsters::MON_MIN_INDEX_ID as usize)..(*next_free_monster_id() as usize) {
        wr_monster(&monsters()[i]);
    }

    // jdbkmoria extension: paintings, appended after all original umoria
    // data. Restore probes for this block with a raw EOF peek (the same
    // trick the dead/alive fork uses), so save files written without it
    // still load. The top two bits of the count byte mark the record
    // layout: legacy (both clear) predates the creature id, v1 (0x80 only)
    // added the creature id, and v2 (0xC0, written here) further adds the
    // loot byte for treasure a slain in-canvas creature leaves behind.
    // MAX_PAINTINGS stays below 0x40 so both flag bits stay clear of the
    // count itself.
    wr_byte(0xC0 | paintings().len() as u8);
    for painting in paintings().iter() {
        wr_byte(painting.pos.y as u8);
        wr_byte(painting.pos.x as u8);
        wr_byte(painting_kind_to_u8(painting.kind));
        wr_byte(painting.desc_id);
        wr_short(painting.creature_id);
        wr_short(painting.hp as u16);
        wr_byte(painting.items);
        wr_byte((painting.awake as u8) | ((painting.found as u8) << 1));
        wr_byte(painting.loot);
    }

    flush_and_check()
}

// Writes the save-file header (version bytes + random xor seed byte)
// followed by the full serialized game state. Assumes FILEPTR is already
// set to an open, writable file. Contains no UI/curses calls: this is the
// pure serialization core shared by `save_char()` (used by the interactive
// `save_game()`) and `save_game_state_to_file()` below (used by tests).
fn write_header_and_save_data() -> bool {
    *XOR_BYTE.get() = 0;
    wr_byte(CURRENT_VERSION_MAJOR);
    *XOR_BYTE.get() = 0;
    wr_byte(CURRENT_VERSION_MINOR);
    *XOR_BYTE.get() = 0;
    wr_byte(CURRENT_VERSION_PATCH);
    *XOR_BYTE.get() = 0;

    let char_tmp = (random_number(256) - 1) as u8;
    wr_byte(char_tmp);
    // Note that xor_byte is now equal to char_tmp

    write_save_data()
}

// Opens `filename` for writing (creating it, or truncating it if it already
// exists) and writes the full save-file header + serialized game state.
// Unlike `save_game()`/`save_char()`, this performs no curses/UI calls and
// does not touch player-disturbance/speed state; it is exposed (`pub`) so
// integration tests can exercise the save file format without a terminal.
pub fn save_game_state_to_file(filename: &str) -> bool {
    *WRITE_ERROR.get() = false;
    *FILEPTR.get() = None;

    match OpenOptions::new().write(true).create(true).truncate(true).open(filename) {
        Ok(file) => *FILEPTR.get() = Some(file),
        Err(_) => return false,
    }

    let ok = write_header_and_save_data();
    close_fileptr();
    ok
}

fn save_char(filename: &str) -> bool {
    // The C code's fopen/open were #defined to tfopen/topen (ui_io), which
    // expand a leading ~ to the user's home directory.
    let expanded = crate::ui_io::tilde(filename).unwrap_or_else(|| filename.to_string());
    let filename: &str = &expanded;
    if game().character_saved {
        return true; // Nothing to save.
    }

    put_qio();
    crate::player::player_disturb(1, 0); // Turn off resting and searching.
    crate::player::player_change_speed(-(py().pack.heaviness as i32)); // Fix the speed
    py().pack.heaviness = 0;

    *WRITE_ERROR.get() = false;
    *FILEPTR.get() = None; // Do not assume it has been init'ed

    // Try to create a brand new file first (equivalent to O_RDWR|O_CREAT|O_EXCL).
    let mut created_file = false;
    let mut open_result = OpenOptions::new().write(true).create_new(true).open(filename);

    if open_result.is_err()
        && Path::new(filename).exists()
        && (*FROM_SAVE_FILE.get() || (game().wizard_mode && get_input_confirmation(tr!("Can't make new save file. Overwrite old?"))))
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(filename) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(filename, perms);
            }
        }
        open_result = OpenOptions::new().write(true).truncate(true).open(filename);
    }

    if let Ok(file) = open_result {
        created_file = true;
        *FILEPTR.get() = Some(file);
    }

    let mut ok = false;

    if FILEPTR.get().is_some() {
        ok = write_header_and_save_data();

        close_fileptr();
    }

    if !ok {
        if created_file {
            let _ = std::fs::remove_file(filename);
        }

        let output = if created_file {
            tr_fmt!("Error writing to file '{}'", filename)
        } else {
            tr_fmt!("Can't create new file '{}'", filename)
        };
        print_message(Some(&output));

        return false;
    }

    game().character_saved = true;
    dg().game_turn = -1;

    true
}

fn try_open_for_read(filename: &str) -> Option<File> {
    // tfopen_read applies the same ~ expansion the C tfopen performed
    if let Ok(f) = crate::ui_io::tfopen_read(filename) {
        return Some(f);
    }

    // Allow restoring a file belonging to someone else, if we can delete it.
    // Hence first try to read without doing a chmod.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(filename) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o400);
            let _ = std::fs::set_permissions(filename, perms);
        }
    }

    File::open(filename).ok()
}

// Certain checks are omitted for the wizard. -CJS-
pub fn load_game(generate: &mut bool) -> bool {
    *generate = true;

    // Not required for Mac, because the file name is obtained through a dialog.
    // There is no way for a nonexistent file to be specified. -BS-
    if !Path::new(&config::files::save_game()).exists() {
        print_message(Some(tr!("Save file does not exist.")));
        return false; // Don't bother with messages here. File absent.
    }

    clear_screen();

    let filename_msg = tr_fmt!("Save file '{}' present. Attempting restore.", config::files::save_game());
    put_string(&filename_msg, Coord::new(23, 0));

    // FIXME: check this if/else logic! -- MRC
    if dg().game_turn >= 0 {
        print_message(Some(tr!("IMPOSSIBLE! Attempt to restore while still alive!")));
    } else if let Some(file) = try_open_for_read(&config::files::save_game()) {
        dg().game_turn = -1;

        if let Some(result) = restore_from_file(file, generate, true) {
            return result;
        }
        // ok was false: "Error during reading of file." was already printed.
    } else {
        print_message(Some(tr!("Can't open file for reading.")));
    }

    dg().game_turn = -1;
    put_string_clear_to_eol(tr!("Please try again without that save file."), Coord::new(1, 0));

    // We have messages for the player to read, this will ask for a keypress
    print_message(None);

    crate::game::exit_program();
}

// Opens `filename` and restores the game state from it, with no curses/UI
// calls (the shared `interactive` core below is run with `interactive:
// false`). This is the non-interactive counterpart to `load_game()`,
// exposed (`pub`) so integration tests can exercise the restore path
// without a terminal.
pub fn load_game_state_from_file(filename: &str, generate: &mut bool) -> Option<bool> {
    let file = File::open(filename).ok()?;
    restore_from_file(file, generate, false)
}

// Shared restore core for `load_game()` (interactive = true) and
// `load_game_state_from_file()` (interactive = false, used by tests). The
// `interactive` flag only gates progress/error messages printed via curses;
// it never changes which data is read, in what order, or any game-logic
// call (matching the interactive path exactly when `interactive` is true).
fn restore_from_file(file: File, generate: &mut bool, interactive: bool) -> Option<bool> {
    set_fileptr(file);

    if interactive {
        put_string_clear_to_eol(tr!("Restoring Memory..."), Coord::new(0, 0));
        put_qio();
    }

    // Note: setting these xor_byte is correct!
    *XOR_BYTE.get() = 0;
    let version_maj = rd_byte();
    *XOR_BYTE.get() = 0;
    let version_min = rd_byte();
    *XOR_BYTE.get() = 0;
    let patch_level = rd_byte();

    *XOR_BYTE.get() = get_byte();

    if !valid_game_version(version_maj, version_min, patch_level) {
        if interactive {
            put_string_clear_to_eol(tr!("Sorry. This save file is from a different version of umoria."), Coord::new(2, 0));
        }
        close_fileptr();
        if interactive {
            print_message(Some(tr!("Error during reading of file.")));
        }
        return None;
    }

    let mut time_saved: u32 = 0;

    // jdbkmoria extension: start from a clean painting registry; the level
    // block below repopulates it for living characters.
    paintings().clear();

    let ok = 'restore: {
        let mut uint_16_t_tmp = rd_short();
        while uint_16_t_tmp != 0xFFFF {
            if uint_16_t_tmp as usize >= MON_MAX_CREATURES {
                break 'restore false;
            }
            let memory = &mut creature_recall()[uint_16_t_tmp as usize];
            memory.movement = rd_long();
            memory.spells = rd_long();
            memory.kills = rd_short();
            memory.deaths = rd_short();
            memory.defenses = rd_short();
            memory.wake = rd_byte();
            memory.ignore = rd_byte();
            rd_bytes(&mut memory.attacks);
            uint_16_t_tmp = rd_short();
        }

        let mut l = rd_long();

        {
            let options = config::options::options();
            options.run_cut_corners = (l & 0x1) != 0;
            options.run_examine_corners = (l & 0x2) != 0;
            options.run_print_self = (l & 0x4) != 0;
            options.find_bound = (l & 0x8) != 0;
            options.prompt_to_pickup = (l & 0x10) != 0;
            options.use_roguelike_keys = (l & 0x20) != 0;
            options.show_inventory_weights = (l & 0x40) != 0;
            options.highlight_seams = (l & 0x80) != 0;
            options.run_ignore_doors = (l & 0x100) != 0;
            options.error_beep_sound = (l & 0x200) != 0;
            options.display_counts = (l & 0x400) != 0;
        }

        // Don't allow resurrection of game.total_winner characters.  It causes
        // problems because the character level is out of the allowed range.
        if game().to_be_wizard && (l & 0x4000_0000) != 0 {
            print_message(Some(tr!("Sorry, this character is retired from moria.")));
            print_message(Some(tr!("You can not resurrect a retired character.")));
        } else if game().to_be_wizard && (l & 0x8000_0000) != 0 && get_input_confirmation(tr!("Resurrect a dead character?")) {
            l &= !0x8000_0000u32;
        }

        if (l & 0x8000_0000) == 0 {
            py().misc.name = rd_string();
            py().misc.gender = rd_bool();
            py().misc.au = rd_long() as i32;
            py().misc.max_exp = rd_long() as i32;
            py().misc.exp = rd_long() as i32;
            py().misc.exp_fraction = rd_short();
            py().misc.age = rd_short();
            py().misc.height = rd_short();
            py().misc.weight = rd_short();
            py().misc.level = rd_short();
            py().misc.max_dungeon_depth = rd_short();
            py().misc.chance_in_search = rd_short() as i16;
            py().misc.fos = rd_short() as i16;
            py().misc.bth = rd_short() as i16;
            py().misc.bth_with_bows = rd_short() as i16;
            py().misc.mana = rd_short() as i16;
            py().misc.max_hp = rd_short() as i16;
            py().misc.plusses_to_hit = rd_short() as i16;
            py().misc.plusses_to_damage = rd_short() as i16;
            py().misc.ac = rd_short() as i16;
            py().misc.magical_ac = rd_short() as i16;
            py().misc.display_to_hit = rd_short() as i16;
            py().misc.display_to_damage = rd_short() as i16;
            py().misc.display_ac = rd_short() as i16;
            py().misc.display_to_ac = rd_short() as i16;
            py().misc.disarm = rd_short() as i16;
            py().misc.saving_throw = rd_short() as i16;
            py().misc.social_class = rd_short() as i16;
            py().misc.stealth_factor = rd_short() as i16;
            py().misc.class_id = rd_byte();
            py().misc.race_id = rd_byte();
            py().misc.hit_die = rd_byte();
            py().misc.experience_factor = rd_byte();
            py().misc.current_mana = rd_short() as i16;
            py().misc.current_mana_fraction = rd_short();
            py().misc.current_hp = rd_short() as i16;
            py().misc.current_hp_fraction = rd_short();
            for entry in py().misc.history.iter_mut() {
                *entry = rd_string();
            }

            rd_bytes(&mut py().stats.max);
            rd_bytes(&mut py().stats.current);
            {
                let mut modified: [u16; 6] = [0; 6];
                rd_shorts(&mut modified);
                for i in 0..6 {
                    py().stats.modified[i] = modified[i] as i16;
                }
            }
            rd_bytes(&mut py().stats.used);

            py().flags.status = rd_long();
            py().flags.rest = rd_short() as i16;
            py().flags.blind = rd_short() as i16;
            py().flags.paralysis = rd_short() as i16;
            py().flags.confused = rd_short() as i16;
            py().flags.food = rd_short() as i16;
            py().flags.food_digested = rd_short() as i16;
            py().flags.protection = rd_short() as i16;
            py().flags.speed = rd_short() as i16;
            py().flags.fast = rd_short() as i16;
            py().flags.slow = rd_short() as i16;
            py().flags.afraid = rd_short() as i16;
            py().flags.poisoned = rd_short() as i16;
            py().flags.image = rd_short() as i16;
            py().flags.protect_evil = rd_short() as i16;
            py().flags.invulnerability = rd_short() as i16;
            py().flags.heroism = rd_short() as i16;
            py().flags.super_heroism = rd_short() as i16;
            py().flags.blessed = rd_short() as i16;
            py().flags.heat_resistance = rd_short() as i16;
            py().flags.cold_resistance = rd_short() as i16;
            py().flags.detect_invisible = rd_short() as i16;
            py().flags.word_of_recall = rd_short() as i16;
            py().flags.see_infra = rd_short() as i16;
            py().flags.timed_infra = rd_short() as i16;
            py().flags.see_invisible = rd_bool();
            py().flags.teleport = rd_bool();
            py().flags.free_action = rd_bool();
            py().flags.slow_digest = rd_bool();
            py().flags.aggravate = rd_bool();
            py().flags.resistant_to_fire = rd_bool();
            py().flags.resistant_to_cold = rd_bool();
            py().flags.resistant_to_acid = rd_bool();
            py().flags.regenerate_hp = rd_bool();
            py().flags.resistant_to_light = rd_bool();
            py().flags.free_fall = rd_bool();
            py().flags.sustain_str = rd_bool();
            py().flags.sustain_int = rd_bool();
            py().flags.sustain_wis = rd_bool();
            py().flags.sustain_con = rd_bool();
            py().flags.sustain_dex = rd_bool();
            py().flags.sustain_chr = rd_bool();
            py().flags.confuse_monster = rd_bool();
            py().flags.new_spells_to_learn = rd_byte();

            *missiles_counter() = rd_short() as i16;
            dg().game_turn = rd_long() as i32;
            py().pack.unique_items = rd_short() as i16;
            if py().pack.unique_items as usize > PlayerEquipment::Wield as usize {
                break 'restore false;
            }
            for i in 0..py().pack.unique_items as usize {
                rd_item(&mut py().inventory[i]);
            }
            for i in (PlayerEquipment::Wield as usize)..PLAYER_INVENTORY_SIZE {
                rd_item(&mut py().inventory[i]);
            }
            py().pack.weight = rd_short() as i16;
            py().equipment_count = rd_short() as i16;
            py().flags.spells_learnt = rd_long();
            py().flags.spells_worked = rd_long();
            py().flags.spells_forgotten = rd_long();
            rd_bytes(&mut py().flags.spells_learned_order);
            rd_bytes(&mut objects_identified()[..]);
            game().magic_seed = rd_long();
            game().town_seed = rd_long();
            *last_message_id() = rd_short() as i16;
            for message in messages().iter_mut() {
                *message = rd_string();
            }

            let panic_save_short = rd_short();
            let total_winner_short = rd_short();
            *panic_save() = panic_save_short != 0;
            game().total_winner = total_winner_short != 0;

            game().noscore = rd_short() as i16;
            rd_shorts(&mut py().base_hp_levels);

            for store in stores().iter_mut() {
                store.turns_left_before_closing = rd_long() as i32;
                store.insults_counter = rd_short() as i16;
                store.owner_id = rd_byte();
                store.unique_items_counter = rd_byte();
                store.good_purchases = rd_short();
                store.bad_purchases = rd_short();
                if store.unique_items_counter as usize > STORE_MAX_DISCRETE_ITEMS {
                    break 'restore false;
                }
                for j in 0..store.unique_items_counter as usize {
                    store.inventory[j].cost = rd_long() as i32;
                    rd_item(&mut store.inventory[j].item);
                }
            }

            time_saved = rd_long();
            game().character_died_from = rd_string();
            py().max_score = rd_long() as i32;
            py().misc.date_of_birth = rd_long() as i32;
        }

        // Peek at the next raw byte (mirrors `c = getc(fileptr)`); do not
        // decode it through the xor chain yet, we may need to "un-read" it.
        let c = read_raw_byte();
        if c.is_none() || (l & 0x8000_0000) != 0 {
            if (l & 0x8000_0000) == 0 {
                if !game().to_be_wizard || dg().game_turn < 0 {
                    break 'restore false;
                }
                if interactive {
                    put_string_clear_to_eol(tr!("Attempting a resurrection!"), Coord::new(0, 0));
                }
                if py().misc.current_hp < 0 {
                    py().misc.current_hp = 0;
                    py().misc.current_hp_fraction = 0;
                }

                // don't let them starve to death immediately
                if py().flags.food < 0 {
                    py().flags.food = 0;
                }

                // don't let them immediately die of poison again
                if py().flags.poisoned > 1 {
                    py().flags.poisoned = 1;
                }

                dg().current_level = 0; // Resurrect on the town level.
                game().character_generated = true;

                // set `noscore` to indicate a resurrection, and don't enter wizard mode
                game().to_be_wizard = false;
                game().noscore |= 0x1;
            } else {
                // Make sure that this message is seen, since it is a bit
                // more interesting than the other messages.
                if interactive {
                    print_message(Some(tr!("Restoring Memory of a departed spirit...")));
                }
                dg().game_turn = -1;
            }
            if interactive {
                put_qio();
            }
            break 'restore true;
        }

        // un-read the peeked byte (mirrors `ungetc(c, fileptr)`)
        if !fileptr_seek(SeekFrom::Current(-1)) {
            break 'restore false;
        }

        if interactive {
            put_string_clear_to_eol(tr!("Restoring Character..."), Coord::new(0, 0));
            put_qio();
        }

        // only level specific info should follow,
        // not present for dead characters

        dg().current_level = rd_short() as i16;
        py().pos.y = rd_short() as i32;
        py().pos.x = rd_short() as i32;
        *monster_multiply_total() = rd_short() as i16;
        dg().height = rd_short() as i16;
        dg().width = rd_short() as i16;
        dg().panel.max_rows = rd_short() as i16;
        dg().panel.max_cols = rd_short() as i16;

        // read in the creature ptr info
        let mut char_tmp = rd_byte();
        while char_tmp != 0xFF {
            let ychar = char_tmp;
            let xchar = rd_byte();
            char_tmp = rd_byte();
            if xchar as i32 > MAX_WIDTH || ychar as i32 > MAX_HEIGHT {
                break 'restore false;
            }
            dg().tile_mut(Coord::new(ychar as i32, xchar as i32)).creature_id = char_tmp;
            char_tmp = rd_byte();
        }

        // read in the treasure ptr info
        let mut char_tmp = rd_byte();
        while char_tmp != 0xFF {
            let ychar = char_tmp;
            let xchar = rd_byte();
            char_tmp = rd_byte();
            if xchar as i32 > MAX_WIDTH || ychar as i32 > MAX_HEIGHT {
                break 'restore false;
            }
            dg().tile_mut(Coord::new(ychar as i32, xchar as i32)).treasure_id = char_tmp;
            char_tmp = rd_byte();
        }

        // read in the rest of the cave info
        let total = (MAX_HEIGHT as usize) * (MAX_WIDTH as usize);
        let mut pos: usize = 0;
        while pos != total {
            let count = rd_byte();
            let char_tmp = rd_byte();
            for _ in 0..count {
                if pos >= total {
                    break 'restore false;
                }
                let row = pos / (MAX_WIDTH as usize);
                let col = pos % (MAX_WIDTH as usize);
                let tile = &mut dg().floor[row][col];
                tile.feature_id = char_tmp & 0xF;
                tile.perma_lit_room = ((char_tmp >> 4) & 0x1) != 0;
                tile.field_mark = ((char_tmp >> 5) & 0x1) != 0;
                tile.permanent_light = ((char_tmp >> 6) & 0x1) != 0;
                tile.temporary_light = ((char_tmp >> 7) & 0x1) != 0;
                pos += 1;
            }
        }

        game().treasure.current_id = rd_short() as i16;
        if game().treasure.current_id as usize > LEVEL_MAX_OBJECTS {
            break 'restore false;
        }
        for i in (config::treasure::MIN_TREASURE_LIST_ID as usize)..(game().treasure.current_id as usize) {
            rd_item(&mut game().treasure.list[i]);
        }
        *next_free_monster_id() = rd_short() as i16;
        if *next_free_monster_id() as usize > MON_TOTAL_ALLOCATIONS {
            break 'restore false;
        }
        for i in (config::monsters::MON_MIN_INDEX_ID as usize)..(*next_free_monster_id() as usize) {
            rd_monster(&mut monsters()[i]);
        }

        *generate = false; // We have restored a cave - no need to generate.

        if eof_hit() {
            break 'restore false;
        }

        // jdbkmoria extension: paintings. Older save files end at the monster
        // data, so peek for more bytes the same way the dead/alive fork
        // does; plain EOF here just means "no paintings".
        paintings().clear();
        if read_raw_byte().is_some() {
            if !fileptr_seek(SeekFrom::Current(-1)) {
                break 'restore false;
            }

            // The top two bits of the count byte mark the record layout:
            // 0xC0 is the current (v2) layout, which adds a loot byte after
            // the flags; 0x80 alone is v1 (creature id, no loot byte, and no
            // Swarm kind -- it didn't exist yet); neither bit set is the
            // original legacy layout from before monster paintings carried
            // a creature id at all.
            let header = rd_byte();
            let is_v2 = header & 0xC0 == 0xC0;
            let is_v1 = !is_v2 && (header & 0x80) != 0;
            let painting_count = if is_v2 { (header & 0x3F) as usize } else { (header & 0x7F) as usize };
            if painting_count > MAX_PAINTINGS {
                break 'restore false;
            }

            for _ in 0..painting_count {
                let y = rd_byte() as i32;
                let x = rd_byte() as i32;
                let kind_byte = rd_byte();
                let desc_id = rd_byte();
                let creature_id = if is_v1 || is_v2 { rd_short() } else { 0 };
                let hp = rd_short() as i16;
                let items = rd_byte();
                let flags = rd_byte();
                let loot = if is_v2 { rd_byte() } else { 0 };

                if y >= MAX_HEIGHT || x >= MAX_WIDTH || eof_hit() {
                    break 'restore false;
                }

                let painting = if is_v1 || is_v2 {
                    painting_from_save(Coord::new(y, x), kind_byte, desc_id, creature_id, hp, items, flags, loot)
                } else {
                    painting_from_legacy_save(Coord::new(y, x), kind_byte, desc_id, hp, items, flags)
                };

                match painting {
                    Some(painting) => paintings().push(painting),
                    None => break 'restore false,
                }
            }
        }

        if dg().game_turn < 0 {
            false
        } else {
            // don't overwrite the killed by string if character is dead
            // jdbkmoria extension note: "(alive and well)" is left
            // untranslated -- it's part of the character_died_from sentinel
            // family (see also the matching literal in game_run.rs) which
            // flows into the save file and score board, outside this
            // module's ownership.
            if py().misc.current_hp >= 0 {
                game().character_died_from = "(alive and well)".to_string();
            }
            game().character_generated = true;
            true
        }
    };

    if !ok {
        close_fileptr();
        if interactive {
            print_message(Some(tr!("Error during reading of file.")));
        }
        return None;
    }

    close_fileptr();

    // let the user overwrite the old save file when save/quit
    *FROM_SAVE_FILE.get() = true;

    if *panic_save() {
        if interactive {
            print_message(Some(tr!("This game is from a panic save.  Score will not be added to scoreboard.")));
        }
    } else {
        // NOTE: faithfully ports `(!game.noscore) & 0x04` from the original
        // C++. Since `!game.noscore` is a logical negation (always 0 or 1),
        // ANDing with 0x04 is always zero - this branch is unreachable dead
        // code in the original umoria source. Kept for byte-for-byte/logic
        // fidelity with the reference implementation.
        let logical_not_noscore: i16 = if game().noscore == 0 { 1 } else { 0 };
        if (logical_not_noscore & 0x4) != 0 {
            if interactive {
                print_message(Some(tr!("This character is already on the scoreboard; it will not be scored again.")));
            }
            game().noscore |= 0x4;
        }
    }

    if dg().game_turn >= 0 {
        // Only if a full restoration.
        py().weapon_is_heavy = false;
        py().pack.heaviness = 0;
        crate::player::player_strength();

        // rotate store inventory, depending on how old the save file
        // is foreach day old (rounded up), call storeMaintenance
        // calculate age in seconds
        *START_TIME.get() = get_current_unix_time();

        let age = if *START_TIME.get() < time_saved { 0 } else { *START_TIME.get() - time_saved };

        let mut age_days = (age + 43200) / 86400; // age in days
        if age_days > 10 {
            age_days = 10; // in case save file is very old
        }

        for _ in 0..age_days {
            crate::store_inventory::store_maintenance();
        }
    }

    if interactive && game().noscore != 0 {
        print_message(Some(tr!("This save file cannot be used to get on the score board.")));
    }
    if interactive && valid_game_version(version_maj, version_min, patch_level) && !is_current_game_version(version_maj, version_min, patch_level) {
        let msg = tr_fmt!(
            "Save file version {}.{} accepted on game version {}.{}.",
            version_maj, version_min, CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR
        );
        print_message(Some(&msg));
    }

    // if false: only restored options and monster memory.
    Some(dg().game_turn >= 0)
}
