// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Inventory and inventory items

use crate::config;
use crate::data_treasure::GAME_OBJECTS;
use crate::dice::Dice;
use crate::dungeon::dg;
use crate::game::{game, random_number};
use crate::identification::{
    item_append_to_inscription, item_description, item_set_colorless_as_identified,
    object_position_offset, spell_item_identified, SpecialNameIds,
};
use crate::player::py;
use crate::treasure::*;
use crate::ui_io::print_message;

// Size of inventory array (DO NOT CHANGE)
pub const PLAYER_INVENTORY_SIZE: usize = 34;

// Inventory stacking `sub_category_id`s - these never stack
// (range-documentation constants; unused in the original C++ as well)
#[allow(dead_code)]
pub const ITEM_NEVER_STACK_MIN: u8 = 0;
#[allow(dead_code)]
pub const ITEM_NEVER_STACK_MAX: u8 = 63;
// these items always stack with others of same `sub_category_id`s, always treated as
// single objects, must be power of 2;
pub const ITEM_SINGLE_STACK_MIN: u8 = 64;
pub const ITEM_SINGLE_STACK_MAX: u8 = 192; // see NOTE below
// these items stack with others only if have same `sub_category_id`s and same `misc_use`,
// they are treated as a group for wielding, etc.
pub const ITEM_GROUP_MIN: u8 = 192;
#[allow(dead_code)]
pub const ITEM_GROUP_MAX: u8 = 255;
// NOTE: items with `sub_category_id`s = 192 are treated as single objects,
// but only stack with others of same `sub_category_id`s if have the same
// `misc_use` value, only used for torches.

// Size of an inscription in the Inventory. Notice alignment, must be 4*x + 1
pub const INSCRIP_SIZE: usize = 13;

// Inventory is created for an item the player may wear about
// their person, or store in their inventory pack.
//
// Only damage, ac, and tchar are constant; level could possibly be made
// constant by changing index instead; all are used rarely.
#[derive(Debug, Clone, Copy, Default)]
pub struct Inventory {
    pub id: u16,                         // Index to object_list
    pub special_name_id: u8,             // Object special name
    pub inscription: [u8; INSCRIP_SIZE], // Object inscription
    pub flags: u32,                      // Special flags
    pub category_id: u8,                 // Category number (tval)
    pub sprite: u8,                      // Character representation - ASCII symbol (tchar)
    pub misc_use: i16,                   // Misc. use variable (p1)
    pub cost: i32,                       // Cost of item
    pub sub_category_id: u8,             // Sub-category number
    pub items_count: u8,                 // Number of items
    pub weight: u16,                     // Weight
    pub to_hit: i16,                     // Plusses to hit
    pub to_damage: i16,                  // Plusses to damage
    pub ac: i16,                         // Normal AC
    pub to_ac: i16,                      // Plusses to AC
    pub damage: Dice,                    // Damage when hits
    pub depth_first_found: u8,           // Dungeon level item first found
    pub identification: u8,              // Identify information
}

impl Inventory {
    pub const fn empty() -> Self {
        Inventory {
            id: 0,
            special_name_id: 0,
            inscription: [0; INSCRIP_SIZE],
            flags: 0,
            category_id: 0,
            sprite: 0,
            misc_use: 0,
            cost: 0,
            sub_category_id: 0,
            items_count: 0,
            weight: 0,
            to_hit: 0,
            to_damage: 0,
            ac: 0,
            to_ac: 0,
            damage: Dice::new(0, 0),
            depth_first_found: 0,
            identification: 0,
        }
    }

    // Set the item's inscription from a string, truncating to the
    // fixed inscription size (including NUL) as the C code did.
    pub fn set_inscription(&mut self, text: &str) {
        self.inscription = [0; INSCRIP_SIZE];
        for (i, b) in text.bytes().take(INSCRIP_SIZE - 1).enumerate() {
            self.inscription[i] = b;
        }
    }

    pub fn inscription_str(&self) -> String {
        let end = self.inscription.iter().position(|&b| b == 0).unwrap_or(INSCRIP_SIZE);
        String::from_utf8_lossy(&self.inscription[..end]).into_owned()
    }
}

// magic numbers for players equipment inventory array
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PlayerEquipment {
    Wield = 22, // must be first item in equipment list
    Head = 23,
    Neck = 24,
    Body = 25,
    Arm = 26,
    Hands = 27,
    Right = 28,
    Left = 29,
    Feet = 30,
    Outer = 31,
    Light = 32,
    Auxiliary = 33,
}

const WIELD: usize = PlayerEquipment::Wield as usize;

pub fn inventory_collect_all_item_flags() -> u32 {
    let mut flags = 0;

    for i in WIELD..(PlayerEquipment::Light as usize) {
        flags |= py().inventory[i].flags;
    }

    flags
}

// Destroy an item in the inventory -RAK-
pub fn inventory_destroy_item(item_id: usize) {
    let item = py().inventory[item_id];

    if item.items_count > 1 && item.sub_category_id <= ITEM_SINGLE_STACK_MAX {
        py().inventory[item_id].items_count -= 1;
        py().pack.weight -= item.weight as i16;
    } else {
        py().pack.weight -= item.weight as i16 * item.items_count as i16;

        for i in item_id..(py().pack.unique_items as usize - 1) {
            py().inventory[i] = py().inventory[i + 1];
        }

        let last = py().pack.unique_items as usize - 1;
        inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut py().inventory[last]);
        py().pack.unique_items -= 1;
    }

    py().flags.status |= config::player::status::PY_STR_WGT;
}

// Copies the object in the second argument over the first argument.
// However, the second always gets a number of one except for ammo etc.
pub fn inventory_take_one_item(to_item: &mut Inventory, from_item: &Inventory) {
    *to_item = *from_item;

    if to_item.items_count > 1 && inventory_item_single_stackable(to_item) {
        to_item.items_count = 1;
    }
}

// Drops an item from inventory to given location -RAK-
pub fn inventory_drop_item(item_id: usize, drop_all: bool) {
    let mut item_id = item_id;

    if dg().tile(py().pos).treasure_id != 0 {
        crate::dungeon::dungeon_delete_object(py().pos);
    }

    let treasure_id = crate::game_objects::popt() as usize;

    let item = py().inventory[item_id];
    game().treasure.list[treasure_id] = item;

    dg().tile_mut(py().pos).treasure_id = treasure_id as u8;

    if item_id >= WIELD {
        crate::player::player_take_off(item_id as i32, -1);
    } else {
        if drop_all || item.items_count == 1 {
            py().pack.weight -= item.weight as i16 * item.items_count as i16;
            py().pack.unique_items -= 1;

            while item_id < py().pack.unique_items as usize {
                py().inventory[item_id] = py().inventory[item_id + 1];
                item_id += 1;
            }

            let last = py().pack.unique_items as usize;
            inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut py().inventory[last]);
        } else {
            game().treasure.list[treasure_id].items_count = 1;
            py().pack.weight -= item.weight as i16;
            py().inventory[item_id].items_count -= 1;
        }

        let prt1 = item_description(&game().treasure.list[treasure_id], true);
        let prt2 = format!("Dropped {}", prt1);
        print_message(Some(&prt2));
    }

    py().flags.status |= config::player::status::PY_STR_WGT;
}

// Destroys a type of item on a given percent chance -RAK-
fn inventory_damage_item(item_type: fn(&Inventory) -> bool, chance_percentage: i32) -> i32 {
    let mut damage = 0;

    let mut i = 0;
    while i < py().pack.unique_items as usize {
        if item_type(&py().inventory[i]) && random_number(100) < chance_percentage {
            inventory_destroy_item(i);
            damage += 1;
        }
        i += 1;
    }

    damage
}

pub fn inventory_diminish_light_attack(noticed: bool) -> bool {
    let mut noticed = noticed;
    let item = &mut py().inventory[PlayerEquipment::Light as usize];

    if item.misc_use > 0 {
        item.misc_use -= 250 + random_number(250) as i16;

        if item.misc_use < 1 {
            item.misc_use = 1;
        }

        if py().flags.blind < 1 {
            print_message(Some("Your light dims."));
        } else {
            noticed = false;
        }
    } else {
        noticed = false;
    }

    noticed
}

pub fn inventory_diminish_charges_attack(creature_level: u8, monster_hp: &mut i16, noticed: bool) -> bool {
    let mut noticed = noticed;
    let item_id = (random_number(py().pack.unique_items as i32) - 1) as usize;
    let item = py().inventory[item_id];

    let has_charges = item.category_id == TV_STAFF || item.category_id == TV_WAND;

    if has_charges && item.misc_use > 0 {
        *monster_hp += creature_level as i16 * item.misc_use;
        py().inventory[item_id].misc_use = 0;
        if !spell_item_identified(&item) {
            item_append_to_inscription(&mut py().inventory[item_id], config::identification::ID_EMPTY);
        }
        print_message(Some("Energy drains from your pack!"));
    } else {
        noticed = false;
    }

    noticed
}

pub fn execute_disenchant_attack() -> bool {
    let item_id = match random_number(7) {
        1 => PlayerEquipment::Wield as usize,
        2 => PlayerEquipment::Body as usize,
        3 => PlayerEquipment::Arm as usize,
        4 => PlayerEquipment::Outer as usize,
        5 => PlayerEquipment::Hands as usize,
        6 => PlayerEquipment::Head as usize,
        7 => PlayerEquipment::Feet as usize,
        _ => return false,
    };

    let mut success = false;
    let item = &mut py().inventory[item_id];

    if item.to_hit > 0 {
        item.to_hit -= random_number(2) as i16;

        // don't send it below zero
        if item.to_hit < 0 {
            item.to_hit = 0;
        }
        success = true;
    }
    if item.to_damage > 0 {
        item.to_damage -= random_number(2) as i16;

        // don't send it below zero
        if item.to_damage < 0 {
            item.to_damage = 0;
        }
        success = true;
    }
    if item.to_ac > 0 {
        item.to_ac -= random_number(2) as i16;

        // don't send it below zero
        if item.to_ac < 0 {
            item.to_ac = 0;
        }
        success = true;
    }

    success
}

// this code must be identical to the inventory_carry_item() code below
pub fn inventory_can_carry_item_count(item: &Inventory) -> bool {
    if (py().pack.unique_items as usize) < WIELD {
        return true;
    }

    if !inventory_item_stackable(item) {
        return false;
    }

    for i in 0..py().pack.unique_items as usize {
        let inv = py().inventory[i];

        let same_character = inv.category_id == item.category_id;
        let same_category = inv.sub_category_id == item.sub_category_id;

        // make sure the number field doesn't overflow
        let same_number = (inv.items_count as u16 + item.items_count as u16) < 256;

        // they always stack (sub_category_id < 192), or else they have same `misc_use`
        let same_group = item.sub_category_id < ITEM_GROUP_MIN || inv.misc_use == item.misc_use;

        // only stack if both or neither are identified
        let inventory_item_is_colorless = item_set_colorless_as_identified(inv.category_id, inv.sub_category_id, inv.identification);
        let item_is_colorless = item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification);
        let identification = inventory_item_is_colorless == item_is_colorless;

        if same_character && same_category && same_number && same_group && identification {
            return true;
        }
    }

    false
}

// return false if picking up an object would change the players speed
pub fn inventory_can_carry_item(item: &Inventory) -> bool {
    let mut limit = crate::player::player_carrying_load_limit();
    let new_weight = item.items_count as i32 * item.weight as i32 + py().pack.weight as i32;

    if limit < new_weight {
        limit = new_weight / (limit + 1);
    } else {
        limit = 0;
    }

    py().pack.heaviness as i32 == limit
}

// Add an item to players inventory.  Return the
// item position for a description if needed. -RAK-
// this code must be identical to the inventory_can_carry_item_count() code above
pub fn inventory_carry_item(new_item: &mut Inventory) -> i32 {
    let is_known = item_set_colorless_as_identified(new_item.category_id, new_item.sub_category_id, new_item.identification);
    let is_always_known = object_position_offset(new_item.category_id, new_item.sub_category_id) == -1;

    let mut slot_id: usize = 0;

    // Now, check to see if player can carry object
    while slot_id < PLAYER_INVENTORY_SIZE {
        let item = py().inventory[slot_id];

        let is_same_category = new_item.category_id == item.category_id;
        let is_same_sub_category = new_item.sub_category_id == item.sub_category_id;
        let not_too_many_items = (new_item.items_count as i32 + item.items_count as i32) < 256;

        // only stack if both or neither are identified
        let same_known_status = item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) == is_known;

        let is_stackable = inventory_item_stackable(new_item);
        let is_same_group = new_item.sub_category_id < ITEM_GROUP_MIN || item.misc_use == new_item.misc_use;

        if is_same_category && is_same_sub_category && is_stackable && not_too_many_items && is_same_group && same_known_status {
            py().inventory[slot_id].items_count += new_item.items_count;
            break;
        }

        if (is_same_category && new_item.sub_category_id < item.sub_category_id && is_always_known) || new_item.category_id > item.category_id {
            // For items which are always `is_known`, i.e. never have a 'color',
            // insert them into the inventory in sorted order.
            let unique_items = py().pack.unique_items as usize;
            let mut i = unique_items as i32 - 1;
            while i >= slot_id as i32 {
                py().inventory[i as usize + 1] = py().inventory[i as usize];
                i -= 1;
            }
            py().inventory[slot_id] = *new_item;
            py().pack.unique_items += 1;
            break;
        }

        slot_id += 1;
    }

    py().pack.weight += new_item.items_count as i16 * new_item.weight as i16;
    py().flags.status |= config::player::status::PY_STR_WGT;

    slot_id as i32
}

// Finds range of item in inventory list -RAK-
pub fn inventory_find_range(item_id_start: i32, item_id_end: i32, j: &mut i32, k: &mut i32) -> bool {
    *j = -1;
    *k = -1;

    let mut at_end_of_range = false;

    for i in 0..py().pack.unique_items as usize {
        let item_id = py().inventory[i].category_id as i32;

        if !at_end_of_range {
            if item_id == item_id_start || item_id == item_id_end {
                at_end_of_range = true;
                *j = i as i32;
            }
        } else if item_id != item_id_start && item_id != item_id_end {
            *k = i as i32 - 1;
            break;
        }
    }

    if at_end_of_range && *k == -1 {
        *k = py().pack.unique_items as i32 - 1;
    }

    at_end_of_range
}

pub fn inventory_item_copy_to(from_item_id: usize, to_item: &mut Inventory) {
    let from = &GAME_OBJECTS[from_item_id];

    to_item.id = from_item_id as u16;
    to_item.special_name_id = SpecialNameIds::SnNull as u8;
    to_item.inscription = [0; INSCRIP_SIZE];
    to_item.flags = from.flags;
    to_item.category_id = from.category_id;
    to_item.sprite = from.sprite;
    to_item.misc_use = from.misc_use;
    to_item.cost = from.cost;
    to_item.sub_category_id = from.sub_category_id;
    to_item.items_count = from.items_count;
    to_item.weight = from.weight;
    to_item.to_hit = from.to_hit;
    to_item.to_damage = from.to_damage;
    to_item.ac = from.ac;
    to_item.to_ac = from.to_ac;
    to_item.damage = from.damage;
    to_item.depth_first_found = from.depth_first_found;
    to_item.identification = 0;
}

// Checks if an item is stackable, only as a singles object.
pub fn inventory_item_single_stackable(item: &Inventory) -> bool {
    item.sub_category_id >= ITEM_SINGLE_STACK_MIN && item.sub_category_id <= ITEM_SINGLE_STACK_MAX
}

// Checks if an item is stackable; either singles objects or group items.
pub fn inventory_item_stackable(item: &Inventory) -> bool {
    item.sub_category_id >= ITEM_SINGLE_STACK_MIN
}

pub fn inventory_item_is_cursed(item: &Inventory) -> bool {
    (item.flags & config::treasure::flags::TR_CURSED) != 0
}

pub fn inventory_item_remove_curse(item: &mut Inventory) {
    item.flags &= !config::treasure::flags::TR_CURSED;
}

// AC gets worse -RAK-
// Note: This routine affects magical AC bonuses so
// that stores can detect the damage.
fn damage_minus_ac(typ_dam: u32) -> bool {
    let mut items: Vec<usize> = Vec::with_capacity(6);

    for id in [
        PlayerEquipment::Body,
        PlayerEquipment::Arm,
        PlayerEquipment::Outer,
        PlayerEquipment::Hands,
        PlayerEquipment::Head,
        // also affect boots
        PlayerEquipment::Feet,
    ] {
        if py().inventory[id as usize].category_id != TV_NOTHING {
            items.push(id as usize);
        }
    }

    let mut minus = false;

    if items.is_empty() {
        return minus;
    }

    let item_id = items[(random_number(items.len() as i32) - 1) as usize];

    if (py().inventory[item_id].flags & typ_dam) != 0 {
        minus = true;

        let description = item_description(&py().inventory[item_id], false);
        let msg = format!("Your {} resists damage!", description);
        print_message(Some(&msg));
    } else if py().inventory[item_id].ac + py().inventory[item_id].to_ac > 0 {
        minus = true;

        let description = item_description(&py().inventory[item_id], false);
        let msg = format!("Your {} is damaged!", description);
        print_message(Some(&msg));

        py().inventory[item_id].to_ac -= 1;
        crate::player::player_recalculate_bonuses();
    }

    minus
}

// Functions to emulate the original Pascal sets
pub fn set_null(_item: &Inventory) -> bool {
    false
}

fn set_corrodable_items(item: &Inventory) -> bool {
    matches!(item.category_id, TV_SWORD | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_WAND)
}

fn set_flammable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_SOFT_ARMOR => {
            // Items of (RF) should not be destroyed.
            (item.flags & config::treasure::flags::TR_RES_FIRE) == 0
        }
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 => true,
        _ => false,
    }
}

fn set_acid_affected_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_MISC | TV_CHEST => true,
        TV_BOLT | TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_SOFT_ARMOR => {
            (item.flags & config::treasure::flags::TR_RES_ACID) == 0
        }
        _ => false,
    }
}

pub fn set_frost_destroyable_items(item: &Inventory) -> bool {
    item.category_id == TV_POTION1 || item.category_id == TV_POTION2 || item.category_id == TV_FLASK
}

pub fn set_lightning_destroyable_items(item: &Inventory) -> bool {
    item.category_id == TV_RING || item.category_id == TV_WAND || item.category_id == TV_SPIKE
}

pub fn set_acid_destroyable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_HELM | TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => {
            (item.flags & config::treasure::flags::TR_RES_ACID) == 0
        }
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 | TV_FOOD | TV_OPEN_DOOR | TV_CLOSED_DOOR => true,
        _ => false,
    }
}

pub fn set_fire_destroyable_items(item: &Inventory) -> bool {
    match item.category_id {
        TV_ARROW | TV_BOW | TV_HAFTED | TV_POLEARM | TV_BOOTS | TV_GLOVES | TV_CLOAK | TV_SOFT_ARMOR => {
            (item.flags & config::treasure::flags::TR_RES_FIRE) == 0
        }
        TV_STAFF | TV_SCROLL1 | TV_SCROLL2 | TV_POTION1 | TV_POTION2 | TV_FLASK | TV_FOOD | TV_OPEN_DOOR | TV_CLOSED_DOOR => true,
        _ => false,
    }
}

// Corrode the unsuspecting person's armor -RAK-
pub fn damage_corroding_gas(creature_name: &str) {
    if !damage_minus_ac(config::treasure::flags::TR_RES_ACID) {
        crate::player::player_takes_hit(random_number(8), creature_name);
    }

    if inventory_damage_item(set_corrodable_items, 5) > 0 {
        print_message(Some("There is an acrid smell coming from your pack."));
    }
}

// Poison gas the idiot. -RAK-
pub fn damage_poisoned_gas(damage: i32, creature_name: &str) {
    crate::player::player_takes_hit(damage, creature_name);

    py().flags.poisoned += 12 + random_number(damage) as i16;
}

// Burn the fool up. -RAK-
pub fn damage_fire(damage: i32, creature_name: &str) {
    let mut damage = damage;

    if py().flags.resistant_to_fire {
        damage /= 3;
    }

    if py().flags.heat_resistance > 0 {
        damage /= 3;
    }

    crate::player::player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_flammable_items, 3) > 0 {
        print_message(Some("There is smoke coming from your pack!"));
    }
}

// Freeze them to death. -RAK-
pub fn damage_cold(damage: i32, creature_name: &str) {
    let mut damage = damage;

    if py().flags.resistant_to_cold {
        damage /= 3;
    }

    if py().flags.cold_resistance > 0 {
        damage /= 3;
    }

    crate::player::player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_frost_destroyable_items, 5) > 0 {
        print_message(Some("Something shatters inside your pack!"));
    }
}

// Lightning bolt the sucker away. -RAK-
pub fn damage_lightning_bolt(damage: i32, creature_name: &str) {
    let mut damage = damage;

    if py().flags.resistant_to_light {
        damage /= 3;
    }

    crate::player::player_takes_hit(damage, creature_name);

    if inventory_damage_item(set_lightning_destroyable_items, 3) > 0 {
        print_message(Some("There are sparks coming from your pack!"));
    }
}

// Throw acid on the hapless victim -RAK-
pub fn damage_acid(damage: i32, creature_name: &str) {
    let mut flag = 0;

    if damage_minus_ac(config::treasure::flags::TR_RES_ACID) {
        flag = 1;
    }

    if py().flags.resistant_to_acid {
        flag += 2;
    }

    crate::player::player_takes_hit(damage / (flag + 1), creature_name);

    if inventory_damage_item(set_acid_affected_items, 3) > 0 {
        print_message(Some("There is an acrid smell coming from your pack!"));
    }
}
