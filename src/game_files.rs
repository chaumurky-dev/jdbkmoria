// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Misc code to access files used by Moria

use crate::config;
use crate::data_player::{CHARACTER_RACES, CLASS_LEVEL_ADJ, CLASSES};
use crate::game::{game, sorted_objects};
use crate::game_objects::{item_get_random_object_id, popt, pusht};
use crate::globals::RacyCell;
use crate::identification::{item_append_to_inscription, item_description, item_identify_as_store_bought};
use crate::inventory::{inventory_item_copy_to, inventory_item_is_cursed, PlayerEquipment, PLAYER_INVENTORY_SIZE};
use crate::helpers::string_to_number;
use crate::player::{
    player_disarm_adjustment, player_get_gender_label, player_rank_title, player_stat_adjustment_wisdom_intelligence,
    py, A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS, BTH_PER_PLUS_TO_HIT_ADJUST, CLASS_BTH, CLASS_BTHB, CLASS_DEVICE,
    CLASS_DISARM, CLASS_SAVE, PLAYER_MAX_LEVEL,
};
use crate::treasure::TV_NOTHING;
use crate::treasure_magic::magic_treasure_magical_ability;
use crate::types::Coord;
use crate::ui::{ctrl_key, stat_rating, stats_as_string, ESCAPE};
use crate::ui_io::{
    clear_screen, get_input_confirmation, get_key_input, get_string_input, put_qio, put_string,
    put_string_clear_to_eol, terminal_restore_screen, terminal_save_screen, wait_for_continue_key,
};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

// Access the global high score file handle (the C code's `highscore_fp` global).
// The concurrent game_save/scores port has not (yet) defined a shared accessor
// for this, so it is defined here and opened by `initialize_score_file()` below.
static HIGHSCORE_FILE: RacyCell<Option<File>> = RacyCell::new(None);

pub fn highscore_file() -> &'static mut Option<File> {
    HIGHSCORE_FILE.get()
}

// initializeScoreFile
// Open the score file while we still have the setuid privileges.  Later
// when the score is being written out, you must be sure to flock the file
// so we don't have multiple people trying to write to it at the same time.
// Craig Norborg (doc)    Mon Aug 10 16:41:59 EST 1987
pub fn initialize_score_file() -> bool {
    match std::fs::OpenOptions::new().read(true).write(true).open(config::files::SCORES) {
        Ok(file) => {
            *highscore_file() = Some(file);
            true
        }
        Err(_) => false,
    }
}

// Attempt to open and print the file containing the intro splash screen text -RAK-
pub fn display_splash_screen() {
    if let Ok(file) = File::open(config::files::SPLASH_SCREEN) {
        clear_screen();

        for (i, line) in BufReader::new(file).lines().flatten().enumerate() {
            put_string(&line, Coord::new(i as i32, 0));
        }

        wait_for_continue_key(23);
    }
}

// Open and display a text help file
// File perusal, primitive, but portable -CJS-
pub fn display_text_help_file(filename: &str) {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            put_string_clear_to_eol(&format!("Can not find help file '{}'.", filename), Coord::new(0, 0));
            return;
        }
    };

    terminal_save_screen();

    let mut lines = BufReader::new(file).lines();
    let mut eof = false;

    while !eof {
        clear_screen();

        for i in 0..23 {
            match lines.next() {
                Some(Ok(text)) => put_string(&text, Coord::new(i, 0)),
                _ => eof = true,
            }
        }

        put_string_clear_to_eol("[ press any key to continue ]", Coord::new(23, 23));
        if get_key_input() == ESCAPE {
            break;
        }
    }

    terminal_restore_screen();
}

// Open and display a "death" text file
pub fn display_death_file(filename: &str) {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            put_string_clear_to_eol(&format!("Can not find help file '{}'.", filename), Coord::new(0, 0));
            return;
        }
    };

    clear_screen();

    let mut lines = BufReader::new(file).lines();

    for i in 0..23 {
        match lines.next() {
            Some(Ok(text)) => put_string(&text, Coord::new(i, 0)),
            _ => break,
        }
    }
}

// Prints a list of random objects to a file. -RAK-
// Note that the objects produced is a sampling of objects
// which be expected to appear on that level.
pub fn output_random_level_objects_to_file() {
    put_string_clear_to_eol("Produce objects on what level?: ", Coord::new(0, 0));

    let mut input = String::new();
    if !get_string_input(&mut input, Coord::new(0, 32), 10) {
        return;
    }

    let mut level: i32 = 0;
    if !string_to_number(&input, &mut level) {
        return;
    }

    put_string_clear_to_eol("Produce how many objects?: ", Coord::new(0, 0));
    if !get_string_input(&mut input, Coord::new(0, 27), 10) {
        return;
    }

    let mut count: i32 = 0;
    if !string_to_number(&input, &mut count) {
        return;
    }

    if count < 1 || level < 0 || level > 1200 {
        put_string_clear_to_eol("Parameters no good.", Coord::new(0, 0));
        return;
    }

    if count > 10000 {
        count = 10000;
    }

    let small_objects = get_input_confirmation("Small objects only?");

    put_string_clear_to_eol("File name: ", Coord::new(0, 0));

    let mut filename = String::new();
    if !get_string_input(&mut filename, Coord::new(0, 11), 64) {
        return;
    }
    if filename.is_empty() {
        return;
    }

    let mut file = match File::create(&filename) {
        Ok(file) => file,
        Err(_) => {
            put_string_clear_to_eol("File could not be opened.", Coord::new(0, 0));
            return;
        }
    };

    put_string_clear_to_eol(&format!("{} random objects being produced...", count), Coord::new(0, 0));

    put_qio();

    let _ = write!(file, "*** Random Object Sampling:\n");
    let _ = write!(file, "*** {} objects\n", count);
    let _ = write!(file, "*** For Level {}\n", level);
    let _ = write!(file, "\n");
    let _ = write!(file, "\n");

    let treasure_id = popt();

    for _ in 0..count {
        let object_id = item_get_random_object_id(level, small_objects);
        let from_item_id = sorted_objects()[object_id as usize] as usize;
        inventory_item_copy_to(from_item_id, &mut game().treasure.list[treasure_id as usize]);

        magic_treasure_magical_ability(treasure_id, level);

        item_identify_as_store_bought(&mut game().treasure.list[treasure_id as usize]);

        if inventory_item_is_cursed(&game().treasure.list[treasure_id as usize]) {
            item_append_to_inscription(&mut game().treasure.list[treasure_id as usize], config::identification::ID_DAMD);
        }

        let item = game().treasure.list[treasure_id as usize];
        let description = item_description(&item, true);
        let _ = write!(file, "{} {}\n", item.depth_first_found, description);
    }

    pusht(treasure_id as u8);

    put_string_clear_to_eol("Completed.", Coord::new(0, 0));
}

// Write character sheet to the file
fn write_character_sheet_to_file(char_file: &mut File) -> io::Result<()> {
    put_string_clear_to_eol("Writing character sheet...", Coord::new(0, 0));
    put_qio();

    let colon = ":";
    let blank = " ";

    write!(char_file, "{}\n\n", ctrl_key('L'))?;

    write!(char_file, " Name{:>9} {:<23}", colon, py().misc.name)?;
    write!(char_file, " Age{:>11} {:>6}", colon, py().misc.age)?;
    write!(char_file, "   STR : {}\n", stats_as_string(py().stats.used[A_STR]))?;

    write!(char_file, " Race{:>9} {:<23}", colon, CHARACTER_RACES[py().misc.race_id as usize].name)?;
    write!(char_file, " Height{:>8} {:>6}", colon, py().misc.height)?;
    write!(char_file, "   INT : {}\n", stats_as_string(py().stats.used[A_INT]))?;

    write!(char_file, " Sex{:>10} {:<23}", colon, player_get_gender_label())?;
    write!(char_file, " Weight{:>8} {:>6}", colon, py().misc.weight)?;
    write!(char_file, "   WIS : {}\n", stats_as_string(py().stats.used[A_WIS]))?;

    write!(char_file, " Class{:>8} {:<23}", colon, CLASSES[py().misc.class_id as usize].title)?;
    write!(char_file, " Social Class : {:>6}", py().misc.social_class)?;
    write!(char_file, "   DEX : {}\n", stats_as_string(py().stats.used[A_DEX]))?;

    write!(char_file, " Title{:>8} {:<23}", colon, player_rank_title())?;
    write!(char_file, "{:>22}", blank)?;
    write!(char_file, "   CON : {}\n", stats_as_string(py().stats.used[A_CON]))?;

    write!(char_file, "{:>34}", blank)?;
    write!(char_file, "{:>26}", blank)?;
    write!(char_file, "   CHR : {}\n\n", stats_as_string(py().stats.used[A_CHR]))?;

    write!(char_file, " + To Hit    : {:>6}", py().misc.display_to_hit)?;
    write!(char_file, "{:>7}Level      : {:>7}", blank, py().misc.level)?;
    write!(char_file, "    Max Hit Points : {:>6}\n", py().misc.max_hp)?;

    write!(char_file, " + To Damage : {:>6}", py().misc.display_to_damage)?;
    write!(char_file, "{:>7}Experience : {:>7}", blank, py().misc.exp)?;
    write!(char_file, "    Cur Hit Points : {:>6}\n", py().misc.current_hp)?;

    write!(char_file, " + To AC     : {:>6}", py().misc.display_to_ac)?;
    write!(char_file, "{:>7}Max Exp    : {:>7}", blank, py().misc.max_exp)?;
    write!(char_file, "    Max Mana{:>8} {:>6}\n", colon, py().misc.mana)?;

    write!(char_file, "   Total AC  : {:>6}", py().misc.display_ac)?;
    if py().misc.level as usize >= PLAYER_MAX_LEVEL {
        write!(char_file, "{:>7}Exp to Adv : *******", blank)?;
    } else {
        let exp_to_adv =
            (py().base_exp_levels[py().misc.level as usize - 1] as i64 * py().misc.experience_factor as i64 / 100) as i32;
        write!(char_file, "{:>7}Exp to Adv : {:>7}", blank, exp_to_adv)?;
    }
    write!(char_file, "    Cur Mana{:>8} {:>6}\n", colon, py().misc.current_mana)?;

    write!(char_file, "{:>28}Gold{:>8} {:>7}\n\n", blank, colon, py().misc.au)?;

    let misc = &py().misc;
    let class_id = misc.class_id as usize;
    let level = misc.level as i32;

    let xbth = misc.bth as i32 + misc.plusses_to_hit as i32 * BTH_PER_PLUS_TO_HIT_ADJUST + (CLASS_LEVEL_ADJ[class_id][CLASS_BTH] as i32 * level);
    let xbthb =
        misc.bth_with_bows as i32 + misc.plusses_to_hit as i32 * BTH_PER_PLUS_TO_HIT_ADJUST + (CLASS_LEVEL_ADJ[class_id][CLASS_BTHB] as i32 * level);

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

    write!(char_file, "(Miscellaneous Abilities)\n\n")?;
    write!(char_file, " Fighting    : {:<10}", stat_rating(Coord::new(12, xbth)))?;
    write!(char_file, "   Stealth     : {:<10}", stat_rating(Coord::new(1, xstl)))?;
    write!(char_file, "   Perception  : {}\n", stat_rating(Coord::new(3, xfos)))?;
    write!(char_file, " Bows/Throw  : {:<10}", stat_rating(Coord::new(12, xbthb)))?;
    write!(char_file, "   Disarming   : {:<10}", stat_rating(Coord::new(8, xdis)))?;
    write!(char_file, "   Searching   : {}\n", stat_rating(Coord::new(6, xsrh)))?;
    write!(char_file, " Saving Throw: {:<10}", stat_rating(Coord::new(6, xsave)))?;
    write!(char_file, "   Magic Device: {:<10}", stat_rating(Coord::new(6, xdev)))?;
    write!(char_file, "   Infra-Vision: {}\n\n", xinfra)?;

    // Write out the character's history
    write!(char_file, "Character Background\n")?;
    for entry in &py().misc.history {
        write!(char_file, " {}\n", entry)?;
    }

    Ok(())
}

fn equipment_placement_description(item_id: usize) -> &'static str {
    match item_id {
        x if x == PlayerEquipment::Wield as usize => "You are wielding",
        x if x == PlayerEquipment::Head as usize => "Worn on head",
        x if x == PlayerEquipment::Neck as usize => "Worn around neck",
        x if x == PlayerEquipment::Body as usize => "Worn on body",
        x if x == PlayerEquipment::Arm as usize => "Worn on shield arm",
        x if x == PlayerEquipment::Hands as usize => "Worn on hands",
        x if x == PlayerEquipment::Right as usize => "Right ring finger",
        x if x == PlayerEquipment::Left as usize => "Left  ring finger",
        x if x == PlayerEquipment::Feet as usize => "Worn on feet",
        x if x == PlayerEquipment::Outer as usize => "Worn about body",
        x if x == PlayerEquipment::Light as usize => "Light source is",
        x if x == PlayerEquipment::Auxiliary as usize => "Secondary weapon",
        _ => "*Unknown value*",
    }
}

// Write out the equipment list.
fn write_equipment_list_to_file(equip_file: &mut File) -> io::Result<()> {
    write!(equip_file, "\n  [Character's Equipment List]\n\n")?;

    if py().equipment_count == 0 {
        write!(equip_file, "  Character has no equipment in use.\n")?;
        return Ok(());
    }

    let mut item_slot_id: u8 = 0;

    for i in (PlayerEquipment::Wield as usize)..PLAYER_INVENTORY_SIZE {
        if py().inventory[i].category_id == TV_NOTHING {
            continue;
        }

        let description = item_description(&py().inventory[i], true);
        write!(equip_file, "  {}) {:<19}: {}\n", (b'a' + item_slot_id) as char, equipment_placement_description(i), description)?;

        item_slot_id += 1;
    }

    write!(equip_file, "{}\n\n", ctrl_key('L'))?;

    Ok(())
}

// Write out the character's inventory.
fn write_inventory_to_file(inv_file: &mut File) -> io::Result<()> {
    write!(inv_file, "  [General Inventory List]\n\n")?;

    if py().pack.unique_items == 0 {
        write!(inv_file, "  Character has no objects in inventory.\n")?;
        return Ok(());
    }

    for i in 0..py().pack.unique_items as usize {
        let description = item_description(&py().inventory[i], true);
        write!(inv_file, "{}) {}\n", (b'a' + i as u8) as char, description)?;
    }

    write!(inv_file, "{}", ctrl_key('L'))?;

    Ok(())
}

// Print the character to a file or device -RAK-
pub fn output_player_character_to_file(filename: &str) -> bool {
    let file = match std::fs::OpenOptions::new().write(true).create_new(true).open(filename) {
        Ok(file) => Some(file),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if get_input_confirmation(&format!("Replace existing file {}?", filename)) {
                File::create(filename).ok()
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let mut file = match file {
        Some(file) => file,
        None => {
            crate::ui_io::print_message(Some(&format!("Can't open file {}:", filename)));
            return false;
        }
    };

    let _ = write_character_sheet_to_file(&mut file);
    let _ = write_equipment_list_to_file(&mut file);
    let _ = write_inventory_to_file(&mut file);

    put_string_clear_to_eol("Completed.", Coord::new(0, 0));

    true
}
