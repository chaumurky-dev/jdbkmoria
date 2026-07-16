// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Handle object identification and descriptions
// ...mostly string handling code

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_tables::{amulets, colors, metals, mushrooms, rocks, woods, SYLLABLES};
use crate::data_treasure::{GAME_OBJECTS, SPECIAL_ITEM_NAMES};
use crate::game::{game, random_number, seed_reset_to_old_seed, seed_set, OBJECT_IDENT_SIZE};
use crate::globals::RacyCell;
use crate::helpers::{insert_string_into_string, is_vowel};
use crate::inventory::{Inventory, ITEM_SINGLE_STACK_MIN, PLAYER_INVENTORY_SIZE};
use crate::monster::monsters;
use crate::player::py;
use crate::treasure::*;
use crate::types::Coord;
use crate::ui_io::{get_string_input, get_tile_character, print_message, put_string_clear_to_eol};

// indexes into the special name table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpecialNameIds {
    SnNull = 0,
    SnR,
    SnRa,
    SnRf,
    SnRc,
    SnRl,
    SnHa,
    SnDf,
    SnSa,
    SnSd,
    SnSe,
    SnSu,
    SnFt,
    SnFb,
    SnFreeAction,
    SnSlaying,
    SnClumsiness,
    SnWeakness,
    SnSlowDescent,
    SnSpeed,
    SnStealth,
    SnSlowness,
    SnNoise,
    SnGreatMass,
    SnIntelligence,
    SnWisdom,
    SnInfravision,
    SnMight,
    SnLordliness,
    SnMagi,
    SnBeauty,
    SnSeeing,
    SnRegeneration,
    SnStupidity,
    SnDullness,
    SnBlindness,
    SnTimidness,
    SnTeleportation,
    SnUgliness,
    SnProtection,
    SnIrritation,
    SnVulnerability,
    SnEnveloping,
    SnFire,
    SnSlayEvil,
    SnDragonSlaying,
    SnEmpty,
    SnLocked,
    SnPoisonNeedle,
    SnGasTrap,
    SnExplosionDevice,
    SnSummoningRunes,
    SnMultipleTraps,
    SnDisarmed,
    SnUnlocked,
    SnSlayAnimal,
    SnArraySize, // 56th item (size value for arrays)
}

pub const SN_ARRAY_SIZE: usize = SpecialNameIds::SnArraySize as usize;

pub const MAX_COLORS: usize = 49; // Used with potions
pub const MAX_MUSHROOMS: usize = 22; // Used with mushrooms
pub const MAX_WOODS: usize = 25; // Used with staffs
pub const MAX_METALS: usize = 25; // Used with wands
pub const MAX_ROCKS: usize = 32; // Used with rings
pub const MAX_AMULETS: usize = 11; // Used with amulets
pub const MAX_TITLES: usize = 45; // Used with scrolls
pub const MAX_SYLLABLES: usize = 153; // Used with scrolls

static MAGIC_ITEM_TITLES: RacyCell<[String; MAX_TITLES]> = RacyCell::new([const { String::new() }; MAX_TITLES]);

pub fn magic_item_titles() -> &'static mut [String; MAX_TITLES] {
    MAGIC_ITEM_TITLES.get()
}

// Identified objects flags
static OBJECTS_IDENTIFIED: RacyCell<[u8; OBJECT_IDENT_SIZE]> = RacyCell::new([0; OBJECT_IDENT_SIZE]);

pub fn objects_identified() -> &'static mut [u8; OBJECT_IDENT_SIZE] {
    OBJECTS_IDENTIFIED.get()
}

fn object_description(command: char) -> String {
    // every printing ASCII character is listed here, in the
    // order in which they appear in the ASCII character set.
    match command {
        ' ' => "  - An open pit.".to_string(),
        '!' => "! - A potion.".to_string(),
        '"' => "\" - An amulet, periapt, or necklace.".to_string(),
        '#' => "# - A stone wall.".to_string(),
        '$' => "$ - Treasure.".to_string(),
        '%' => {
            if !config::options::options().highlight_seams {
                "% - Not used.".to_string()
            } else {
                "% - A magma or quartz vein.".to_string()
            }
        }
        '&' => "& - Treasure chest.".to_string(),
        '\'' => "' - An open door.".to_string(),
        '(' => "( - Soft armor.".to_string(),
        ')' => ") - A shield.".to_string(),
        '*' => "* - Gems.".to_string(),
        '+' => "+ - A closed door.".to_string(),
        ',' => ", - Food or mushroom patch.".to_string(),
        '-' => "- - A wand".to_string(),
        '.' => ". - Floor.".to_string(),
        '/' => "/ - A pole weapon.".to_string(),
        '0' => "0 - A painting on a wall.".to_string(),
        '1' => "1 - Entrance to General Store.".to_string(),
        '2' => "2 - Entrance to Armory.".to_string(),
        '3' => "3 - Entrance to Weaponsmith.".to_string(),
        '4' => "4 - Entrance to Temple.".to_string(),
        '5' => "5 - Entrance to Alchemy shop.".to_string(),
        '6' => "6 - Entrance to Magic-Users store.".to_string(),
        ':' => ": - Rubble.".to_string(),
        ';' => "; - A loose rock.".to_string(),
        '<' => "< - An up staircase.".to_string(),
        '=' => "= - A ring.".to_string(),
        '>' => "> - A down staircase.".to_string(),
        '?' => "? - A scroll.".to_string(),
        '@' => py().misc.name.clone(),
        'A' => "A - Giant Ant Lion.".to_string(),
        'B' => "B - The Balrog.".to_string(),
        'C' => "C - Gelatinous Cube.".to_string(),
        'D' => "D - An Ancient Dragon (Beware).".to_string(),
        'E' => "E - Elemental.".to_string(),
        'F' => "F - Giant Fly.".to_string(),
        'G' => "G - Ghost.".to_string(),
        'H' => "H - Hobgoblin.".to_string(),
        'J' => "J - Jelly.".to_string(),
        'K' => "K - Killer Beetle.".to_string(),
        'L' => "L - Lich.".to_string(),
        'M' => "M - Mummy.".to_string(),
        'O' => "O - Ooze.".to_string(),
        'P' => "P - Giant humanoid.".to_string(),
        'Q' => "Q - Quylthulg (Pulsing Flesh Mound).".to_string(),
        'R' => "R - Reptile.".to_string(),
        'S' => "S - Giant Scorpion.".to_string(),
        'T' => "T - Troll.".to_string(),
        'U' => "U - Umber Hulk.".to_string(),
        'V' => "V - Vampire.".to_string(),
        'W' => "W - Wight or Wraith.".to_string(),
        'X' => "X - Xorn.".to_string(),
        'Y' => "Y - Yeti.".to_string(),
        '[' => "[ - Hard armor.".to_string(),
        '\\' => "\\ - A hafted weapon.".to_string(),
        ']' => "] - Misc. armor.".to_string(),
        '^' => "^ - A trap.".to_string(),
        '_' => "_ - A staff.".to_string(),
        'a' => "a - Giant Ant.".to_string(),
        'b' => "b - Giant Bat.".to_string(),
        'c' => "c - Giant Centipede.".to_string(),
        'd' => "d - Dragon.".to_string(),
        'e' => "e - Floating Eye.".to_string(),
        'f' => "f - Giant Frog.".to_string(),
        'g' => "g - Golem.".to_string(),
        'h' => "h - Harpy.".to_string(),
        'i' => "i - Icky Thing.".to_string(),
        'j' => "j - Jackal.".to_string(),
        'k' => "k - Kobold.".to_string(),
        'l' => "l - Giant Louse.".to_string(),
        'm' => "m - Mold.".to_string(),
        'n' => "n - Naga.".to_string(),
        'o' => "o - Orc or Ogre.".to_string(),
        'p' => "p - Person (Humanoid).".to_string(),
        'q' => "q - Quasit.".to_string(),
        'r' => "r - Rodent.".to_string(),
        's' => "s - Skeleton.".to_string(),
        't' => "t - Giant Tick.".to_string(),
        'w' => "w - Worm or Worm Mass.".to_string(),
        'y' => "y - Yeek.".to_string(),
        'z' => "z - Zombie.".to_string(),
        '{' => "{ - Arrow, bolt, or bullet.".to_string(),
        '|' => "| - A sword or dagger.".to_string(),
        '}' => "} - Bow, crossbow, or sling.".to_string(),
        '~' => "~ - Miscellaneous item.".to_string(),
        _ => "Not Used.".to_string(),
    }
}

pub fn identify_game_object() {
    // an ASCII character representing the item/monster tile, e.g. `+` = Door.
    let mut item_id = '\0';

    if !get_tile_character("Enter character to be identified :", &mut item_id) {
        return;
    }

    put_string_clear_to_eol(&object_description(item_id), Coord::new(0, 0));
    crate::recall::recall_monster_attributes(item_id);
}

// Initialize all Potions, wands, staves, scrolls, etc.
pub fn magic_initialize_item_names() {
    seed_set(game().magic_seed);

    // The first 3 entries for colors are fixed, (slime & apple juice, water)
    for i in 3..MAX_COLORS {
        let id = (random_number(MAX_COLORS as i32 - 3) + 2) as usize;
        colors().swap(i, id);
    }

    for i in 0..MAX_WOODS {
        let id = (random_number(MAX_WOODS as i32) - 1) as usize;
        woods().swap(i, id);
    }

    for i in 0..MAX_METALS {
        let id = (random_number(MAX_METALS as i32) - 1) as usize;
        metals().swap(i, id);
    }

    for i in 0..MAX_ROCKS {
        let id = (random_number(MAX_ROCKS as i32) - 1) as usize;
        rocks().swap(i, id);
    }

    for i in 0..MAX_AMULETS {
        let id = (random_number(MAX_AMULETS as i32) - 1) as usize;
        amulets().swap(i, id);
    }

    for i in 0..MAX_MUSHROOMS {
        let id = (random_number(MAX_MUSHROOMS as i32) - 1) as usize;
        mushrooms().swap(i, id);
    }

    for item_title in magic_item_titles().iter_mut() {
        let mut title = String::new();
        let k = random_number(2) + 1;

        for i in 0..k {
            for _ in 0..random_number(2) {
                title.push_str(SYLLABLES[(random_number(MAX_SYLLABLES as i32) - 1) as usize]);
            }
            if i < k - 1 {
                title.push(' ');
            }
        }

        // titles are truncated to 9 characters, or 8 if
        // that would leave a trailing space
        let bytes = title.as_bytes();
        if bytes.len() > 8 && bytes[8] == b' ' {
            title.truncate(8);
        } else {
            title.truncate(9);
        }

        *item_title = title;
    }

    seed_reset_to_old_seed();
}

pub fn object_position_offset(category_id: u8, sub_category_id: u8) -> i16 {
    match category_id {
        TV_AMULET => 0,
        TV_RING => 1,
        TV_STAFF => 2,
        TV_WAND => 3,
        TV_SCROLL1 | TV_SCROLL2 => 4,
        TV_POTION1 | TV_POTION2 => 5,
        TV_FOOD => {
            if ((sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as usize) < MAX_MUSHROOMS {
                6
            } else {
                -1
            }
        }
        _ => -1,
    }
}

fn clear_object_tried_flag(id: i16) {
    objects_identified()[id as usize] &= !config::identification::OD_TRIED;
}

fn set_object_tried_flag(id: i16) {
    objects_identified()[id as usize] |= config::identification::OD_TRIED;
}

fn is_object_known(id: i16) -> bool {
    (objects_identified()[id as usize] & config::identification::OD_KNOWN1) != 0
}

// Remove "Secret" symbol for identity of object
pub fn item_set_as_identified(category_id: u8, sub_category_id: u8) {
    let mut id = object_position_offset(category_id, sub_category_id);

    if id < 0 {
        return;
    }

    id <<= 6;
    id += (sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as i16;

    objects_identified()[id as usize] |= config::identification::OD_KNOWN1;

    // clear the tried flag, since it is now known
    clear_object_tried_flag(id);
}

// Remove an automatically generated inscription. -CJS-
fn unsample(item: &mut Inventory) {
    // this also used to clear ID_DAMD flag, but I think it should remain set
    item.identification &= !(config::identification::ID_MAGIK | config::identification::ID_EMPTY);

    let mut id = object_position_offset(item.category_id, item.sub_category_id);

    if id < 0 {
        return;
    }

    id <<= 6;
    id += (item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as i16;

    // clear the tried flag, since it is now known
    clear_object_tried_flag(id);
}

// Remove "Secret" symbol for identity of plusses
pub fn spell_item_identify_and_remove_random_inscription(item: &mut Inventory) {
    unsample(item);
    item.identification |= config::identification::ID_KNOWN2;
}

pub fn spell_item_identified(item: &Inventory) -> bool {
    (item.identification & config::identification::ID_KNOWN2) != 0
}

pub fn spell_item_remove_identification(item: &mut Inventory) {
    item.identification &= !config::identification::ID_KNOWN2;
}

pub fn item_identification_clear_empty(item: &mut Inventory) {
    item.identification &= !config::identification::ID_EMPTY;
}

pub fn item_identify_as_store_bought(item: &mut Inventory) {
    item.identification |= config::identification::ID_STORE_BOUGHT;
    spell_item_identify_and_remove_random_inscription(item);
}

fn item_store_bought(identification: u8) -> bool {
    (identification & config::identification::ID_STORE_BOUGHT) != 0
}

// Items which don't have a 'color' are always known / item_set_as_identified(),
// so that they can be carried in order in the inventory.
pub fn item_set_colorless_as_identified(category_id: u8, sub_category_id: u8, identification: u8) -> bool {
    let mut id = object_position_offset(category_id, sub_category_id);

    if id < 0 {
        return config::identification::OD_KNOWN1 != 0;
    }
    if item_store_bought(identification) {
        return config::identification::OD_KNOWN1 != 0;
    }

    id <<= 6;
    id += (sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as i16;

    is_object_known(id)
}

// Somethings been sampled -CJS-
pub fn item_set_as_tried(item: &Inventory) {
    let mut id = object_position_offset(item.category_id, item.sub_category_id);

    if id < 0 {
        return;
    }

    id <<= 6;
    id += (item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as i16;

    set_object_tried_flag(id);
}

// Somethings been identified.
// Extra complexity by CJS so that it can merge store/dungeon objects when appropriate.
pub fn item_identify(item_id: &mut usize) {
    // note: the C code takes (item, item_id) where item aliases py.inventory[item_id];
    // here we work through the player's inventory directly.
    let item = py().inventory[*item_id];

    if crate::inventory::inventory_item_is_cursed(&item) {
        item_append_to_inscription(&mut py().inventory[*item_id], config::identification::ID_DAMD);
    }

    if item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        return;
    }

    item_set_as_identified(item.category_id, item.sub_category_id);

    // no merging possible
    if !crate::inventory::inventory_item_single_stackable(&item) {
        return;
    }

    let mut i: usize = 0;
    while i < py().pack.unique_items as usize {
        let t_ptr = py().inventory[i];

        let matching_cat = t_ptr.category_id == item.category_id;
        let matching_sub_cat = t_ptr.sub_category_id == item.sub_category_id;
        let total_items_count = t_ptr.items_count as i32 + item.items_count as i32;

        if matching_cat && matching_sub_cat && i != *item_id && total_items_count < 256 {
            let mut i_mut = i;

            // make *item_id the smaller number
            if *item_id > i {
                std::mem::swap(item_id, &mut i_mut);
            }

            print_message(Some("You combine similar objects from the shop and dungeon."));

            py().inventory[*item_id].items_count += py().inventory[i_mut].items_count;
            py().pack.unique_items -= 1;

            let mut j = i_mut;
            while j < py().pack.unique_items as usize {
                py().inventory[j] = py().inventory[j + 1];
                j += 1;
            }

            let mut nothing = Inventory::empty();
            crate::inventory::inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut nothing);
            py().inventory[j] = nothing;

            // the C loop variable took on the swapped index before continuing
            i = i_mut;
        }
        i += 1;
    }
}

// If an object has lost magical properties,
// remove the appropriate portion of the name. -CJS-
pub fn item_remove_magic_naming(item: &mut Inventory) {
    item.special_name_id = SpecialNameIds::SnNull as u8;
}

pub fn bow_damage_value(misc_use: i16) -> i32 {
    if misc_use == 1 || misc_use == 2 {
        return 2;
    }
    if misc_use == 3 || misc_use == 5 {
        return 3;
    }
    if misc_use == 4 || misc_use == 6 {
        return 4;
    }
    -1
}

// determines how the `item.misc_use` field is printed
#[derive(PartialEq, Eq, Clone, Copy)]
enum ItemMiscUse {
    Ignored,
    Charges,
    Plusses,
    Light,
    Flags,
    ZPlusses,
}

// Returns the `description` for an inventory item.
// The `add_prefix` param indicates that an article must be added.
pub fn item_description(item: &Inventory, add_prefix: bool) -> String {
    let indexx = (item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as usize;

    // base name, modifier string
    let mut basenm: String = GAME_OBJECTS[item.id as usize].name.to_string();
    let mut modstr: Option<&str> = None;

    let mut damstr = String::new();

    let mut append_name = false;
    let modify = !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification);
    let mut misc_type = ItemMiscUse::Ignored;

    match item.category_id {
        TV_MISC | TV_CHEST => {}
        TV_SLING_AMMO | TV_BOLT | TV_ARROW => {
            damstr = format!(" ({}d{})", item.damage.dice, item.damage.sides);
        }
        TV_LIGHT => {
            misc_type = ItemMiscUse::Light;
        }
        TV_SPIKE => {}
        TV_BOW => {
            damstr = format!(" (x{})", bow_damage_value(item.misc_use));
        }
        TV_HAFTED | TV_POLEARM | TV_SWORD => {
            damstr = format!(" ({}d{})", item.damage.dice, item.damage.sides);
            misc_type = ItemMiscUse::Flags;
        }
        TV_DIGGING => {
            misc_type = ItemMiscUse::ZPlusses;
            // NOTE: the original prints sides twice (not dice), kept for fidelity
            damstr = format!(" ({}d{})", item.damage.sides, item.damage.sides);
        }
        TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => {}
        TV_AMULET => {
            if modify {
                basenm = "& %s Amulet".to_string();
                modstr = Some(amulets()[indexx]);
            } else {
                basenm = "& Amulet".to_string();
                append_name = true;
            }
            misc_type = ItemMiscUse::Plusses;
        }
        TV_RING => {
            if modify {
                basenm = "& %s Ring".to_string();
                modstr = Some(rocks()[indexx]);
            } else {
                basenm = "& Ring".to_string();
                append_name = true;
            }
            misc_type = ItemMiscUse::Plusses;
        }
        TV_STAFF => {
            if modify {
                basenm = "& %s Staff".to_string();
                modstr = Some(woods()[indexx]);
            } else {
                basenm = "& Staff".to_string();
                append_name = true;
            }
            misc_type = ItemMiscUse::Charges;
        }
        TV_WAND => {
            if modify {
                basenm = "& %s Wand".to_string();
                modstr = Some(metals()[indexx]);
            } else {
                basenm = "& Wand".to_string();
                append_name = true;
            }
            misc_type = ItemMiscUse::Charges;
        }
        TV_SCROLL1 | TV_SCROLL2 => {
            if modify {
                basenm = "& Scroll~ titled \"%s\"".to_string();
                modstr = Some(&magic_item_titles()[indexx]);
            } else {
                basenm = "& Scroll~".to_string();
                append_name = true;
            }
        }
        TV_POTION1 | TV_POTION2 => {
            if modify {
                basenm = "& %s Potion~".to_string();
                modstr = Some(colors()[indexx]);
            } else {
                basenm = "& Potion~".to_string();
                append_name = true;
            }
        }
        TV_FLASK => {}
        TV_FOOD => {
            if modify {
                if indexx <= 15 {
                    basenm = "& %s Mushroom~".to_string();
                } else if indexx <= 20 {
                    basenm = "& Hairy %s Mold~".to_string();
                }
                if indexx <= 20 {
                    modstr = Some(mushrooms()[indexx]);
                }
            } else {
                append_name = true;
                if indexx <= 15 {
                    basenm = "& Mushroom~".to_string();
                } else if indexx <= 20 {
                    basenm = "& Hairy Mold~".to_string();
                } else {
                    // Ordinary food does not have a name appended.
                    append_name = false;
                }
            }
        }
        TV_MAGIC_BOOK => {
            modstr = None;
            let name = basenm;
            basenm = format!("& Book~ of Magic Spells {}", name);
        }
        TV_PRAYER_BOOK => {
            modstr = None;
            let name = basenm;
            basenm = format!("& Holy Book~ of Prayers {}", name);
        }
        TV_OPEN_DOOR | TV_CLOSED_DOOR | TV_SECRET_DOOR | TV_RUBBLE => {}
        TV_GOLD | TV_INVIS_TRAP | TV_VIS_TRAP | TV_UP_STAIR | TV_DOWN_STAIR => {
            return format!("{}.", GAME_OBJECTS[item.id as usize].name);
        }
        TV_STORE_DOOR => {
            return format!("the entrance to the {}.", GAME_OBJECTS[item.id as usize].name);
        }
        _ => {
            return "Error in objdes()".to_string();
        }
    }

    let mut tmp_val: String = match modstr {
        Some(modstr) => basenm.replace("%s", modstr),
        None => basenm,
    };

    if append_name {
        tmp_val.push_str(" of ");
        tmp_val.push_str(GAME_OBJECTS[item.id as usize].name);
    }

    if item.items_count != 1 {
        insert_string_into_string(&mut tmp_val, "ch~", "ches");
        insert_string_into_string(&mut tmp_val, "~", "s");
    } else {
        insert_string_into_string(&mut tmp_val, "~", "");
    }

    if !add_prefix {
        if tmp_val.starts_with("some") {
            return tmp_val[5..].to_string();
        }
        if tmp_val.starts_with('&') {
            // eliminate the '& ' at the beginning
            return tmp_val[2..].to_string();
        }
        return tmp_val;
    }

    if item.special_name_id != SpecialNameIds::SnNull as u8 && spell_item_identified(item) {
        tmp_val.push(' ');
        tmp_val.push_str(SPECIAL_ITEM_NAMES[item.special_name_id as usize]);
    }

    if !damstr.is_empty() {
        tmp_val.push_str(&damstr);
    }

    if spell_item_identified(item) {
        let abs_to_hit = item.to_hit.abs();
        let abs_to_damage = item.to_damage.abs();

        if (item.identification & config::identification::ID_SHOW_HIT_DAM) != 0 {
            tmp_val.push_str(&format!(
                " ({}{},{}{})",
                if item.to_hit < 0 { '-' } else { '+' },
                abs_to_hit,
                if item.to_damage < 0 { '-' } else { '+' },
                abs_to_damage
            ));
        } else if item.to_hit != 0 {
            tmp_val.push_str(&format!(" ({}{})", if item.to_hit < 0 { '-' } else { '+' }, abs_to_hit));
        } else if item.to_damage != 0 {
            tmp_val.push_str(&format!(" ({}{})", if item.to_damage < 0 { '-' } else { '+' }, abs_to_damage));
        }
    }

    // Crowns have a zero base AC, so make a special test for them.
    let abs_to_ac = item.to_ac.abs();
    if item.ac != 0 || item.category_id == TV_HELM {
        tmp_val.push_str(&format!(" [{}", item.ac));
        if spell_item_identified(item) {
            // originally used %+d, but several machines don't support it
            tmp_val.push_str(&format!(",{}{}", if item.to_ac < 0 { '-' } else { '+' }, abs_to_ac));
        }
        tmp_val.push(']');
    } else if item.to_ac != 0 && spell_item_identified(item) {
        // originally used %+d, but several machines don't support it
        tmp_val.push_str(&format!(" [{}{}]", if item.to_ac < 0 { '-' } else { '+' }, abs_to_ac));
    }

    // override defaults, check for `misc_type` flags in the `item.identification` field
    if (item.identification & config::identification::ID_NO_SHOW_P1) != 0 {
        misc_type = ItemMiscUse::Ignored;
    } else if (item.identification & config::identification::ID_SHOW_P1) != 0 {
        misc_type = ItemMiscUse::ZPlusses;
    }

    let mut tmp_str = String::new();

    if misc_type == ItemMiscUse::Light {
        tmp_str = format!(" with {} turns of light", item.misc_use);
    } else if misc_type == ItemMiscUse::Ignored {
        // NOOP
    } else if spell_item_identified(item) {
        let abs_misc_use = item.misc_use.abs();

        if misc_type == ItemMiscUse::ZPlusses {
            // originally used %+d, but several machines don't support it
            tmp_str = format!(" ({}{})", if item.misc_use < 0 { '-' } else { '+' }, abs_misc_use);
        } else if misc_type == ItemMiscUse::Charges {
            tmp_str = format!(" ({} charges)", item.misc_use);
        } else if item.misc_use != 0 {
            if misc_type == ItemMiscUse::Plusses {
                tmp_str = format!(" ({}{})", if item.misc_use < 0 { '-' } else { '+' }, abs_misc_use);
            } else if misc_type == ItemMiscUse::Flags {
                if (item.flags & config::treasure::flags::TR_STR) != 0 {
                    tmp_str = format!(" ({}{} to STR)", if item.misc_use < 0 { '-' } else { '+' }, abs_misc_use);
                } else if (item.flags & config::treasure::flags::TR_STEALTH) != 0 {
                    tmp_str = format!(" ({}{} to stealth)", if item.misc_use < 0 { '-' } else { '+' }, abs_misc_use);
                }
            }
        }
    }
    tmp_val.push_str(&tmp_str);

    // ampersand is always the first character
    let mut description: String;
    if let Some(stripped) = tmp_val.strip_prefix('&') {
        // use `stripped`, so that & does not appear in output
        if item.items_count > 1 {
            description = format!("{}{}", item.items_count, stripped);
        } else if item.items_count < 1 {
            description = format!("no more{}", stripped);
        } else if is_vowel(tmp_val.chars().nth(2).unwrap_or(' ')) {
            description = format!("an{}", stripped);
        } else {
            description = format!("a{}", stripped);
        }
    } else if item.items_count < 1 {
        // handle 'no more' case specially

        // check for "some" at start
        if tmp_val.starts_with("some") {
            description = format!("no more {}", &tmp_val[5..]);
        } else {
            // here if no article
            description = format!("no more {}", tmp_val);
        }
    } else {
        description = tmp_val;
    }

    let mut tmp_str = String::new();

    let offset = object_position_offset(item.category_id, item.sub_category_id);
    if offset >= 0 {
        let mut indexx = offset << 6;
        indexx += (item.sub_category_id & (ITEM_SINGLE_STACK_MIN - 1)) as i16;

        // don't print tried string for store bought items
        if (objects_identified()[indexx as usize] & config::identification::OD_TRIED) != 0 && !item_store_bought(item.identification) {
            tmp_str.push_str("tried ");
        }
    }

    if (item.identification & (config::identification::ID_MAGIK | config::identification::ID_EMPTY | config::identification::ID_DAMD)) != 0 {
        if (item.identification & config::identification::ID_MAGIK) != 0 {
            tmp_str.push_str("magik ");
        }
        if (item.identification & config::identification::ID_EMPTY) != 0 {
            tmp_str.push_str("empty ");
        }
        if (item.identification & config::identification::ID_DAMD) != 0 {
            tmp_str.push_str("damned ");
        }
    }

    if item.inscription[0] != 0 {
        tmp_str.push_str(&item.inscription_str());
    } else if !tmp_str.is_empty() {
        // remove the extra blank at the end
        tmp_str.pop();
    }

    if !tmp_str.is_empty() {
        description.push_str(&format!(" {{{}}}", tmp_str));
    }

    description.push('.');

    description
}

// Describe number of remaining charges. -RAK-
pub fn item_charges_remaining_description(item_id: usize) {
    if !spell_item_identified(&py().inventory[item_id]) {
        return;
    }

    let rem_num = py().inventory[item_id].misc_use;

    let out_val = format!("You have {} charges remaining.", rem_num);
    print_message(Some(&out_val));
}

// Describe amount of item remaining. -RAK-
pub fn item_type_remaining_count_description(item_id: usize) {
    py().inventory[item_id].items_count -= 1;

    let tmp_str = item_description(&py().inventory[item_id], true);

    py().inventory[item_id].items_count += 1;

    // the string already has a dot at the end.
    let out_val = format!("You have {}", tmp_str);
    print_message(Some(&out_val));
}

// Add a comment to an object description. -CJS-
pub fn item_inscribe() {
    if py().pack.unique_items == 0 && py().equipment_count == 0 {
        print_message(Some("You are not carrying anything to inscribe."));
        return;
    }

    let mut item_id: i32 = 0;
    if !crate::ui_inventory::inventory_get_input_for_item_id(&mut item_id, "Which one? ", 0, PLAYER_INVENTORY_SIZE as i32, None, None) {
        return;
    }

    let msg = item_description(&py().inventory[item_id as usize], true);

    let inscription_msg = format!("Inscribing {}", msg);
    print_message(Some(&inscription_msg));

    let prompt: String = if py().inventory[item_id as usize].inscription[0] != 0 {
        format!("Replace {} New inscription:", py().inventory[item_id as usize].inscription_str())
    } else {
        "Inscription: ".to_string()
    };

    let mut msg_len = 78 - msg.chars().count() as i32;
    if msg_len > 12 {
        msg_len = 12;
    }

    put_string_clear_to_eol(&prompt, Coord::new(0, 0));

    let mut inscription = String::new();
    if get_string_input(&mut inscription, Coord::new(0, prompt.chars().count() as i32), msg_len) {
        item_replace_inscription(&mut py().inventory[item_id as usize], &inscription);
    }
}

// Append an additional comment to an object description. -CJS-
pub fn item_append_to_inscription(item: &mut Inventory, item_ident_type: u8) {
    item.identification |= item_ident_type;
}

// Replace any existing comment in an object description with a new one. -CJS-
pub fn item_replace_inscription(item: &mut Inventory, inscription: &str) {
    item.set_inscription(inscription);
}

pub fn object_blocked_by_monster(monster_id: usize) {
    let monster = &monsters()[monster_id];
    let name = CREATURES_LIST[monster.creature_id as usize].name;

    let description = if monster.lit {
        format!("The {}", name)
    } else {
        "Something".to_string()
    };

    let msg = format!("{} is in your way!", description);
    print_message(Some(&msg));
}
