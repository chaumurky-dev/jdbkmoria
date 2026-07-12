// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Store: updating store inventory, pricing objects

use crate::config;
use crate::data_store_owners::STORE_OWNERS;
use crate::data_stores::{RACE_GOLD_ADJUSTMENTS, STORE_CHOICES};
use crate::data_treasure::GAME_OBJECTS;
use crate::game::{game, random_number};
use crate::game_objects::{popt, pusht};
use crate::identification::{item_identify_as_store_bought, item_set_colorless_as_identified, spell_item_identified, MAX_MUSHROOMS};
use crate::inventory::{inventory_item_copy_to, inventory_item_single_stackable, inventory_item_stackable, Inventory, ITEM_GROUP_MIN, ITEM_SINGLE_STACK_MIN};
use crate::player::py;
use crate::store_data::{stores, Store, MAX_STORES, STORE_MAX_DISCRETE_ITEMS, STORE_MAX_ITEM_TYPES};
use crate::treasure::*;
use crate::treasure_magic::magic_treasure_magical_ability;

// Initialize and up-keep the store's inventory. -RAK-
pub fn store_maintenance() {
    for store_id in 0..MAX_STORES {
        stores()[store_id].insults_counter = 0;

        if stores()[store_id].unique_items_counter >= config::stores::STORE_MIN_AUTO_SELL_ITEMS {
            let mut turnaround = random_number(config::stores::STORE_STOCK_TURN_AROUND as i32);
            if stores()[store_id].unique_items_counter >= config::stores::STORE_MAX_AUTO_BUY_ITEMS {
                turnaround += 1 + stores()[store_id].unique_items_counter as i32 - config::stores::STORE_MAX_AUTO_BUY_ITEMS as i32;
            }
            turnaround -= 1;
            while turnaround >= 0 {
                let idx = random_number(stores()[store_id].unique_items_counter as i32) - 1;
                store_destroy_item(store_id as i32, idx, false);
                turnaround -= 1;
            }
        }

        if stores()[store_id].unique_items_counter <= config::stores::STORE_MAX_AUTO_BUY_ITEMS {
            let mut turnaround = random_number(config::stores::STORE_STOCK_TURN_AROUND as i32);
            if stores()[store_id].unique_items_counter < config::stores::STORE_MIN_AUTO_SELL_ITEMS {
                turnaround += config::stores::STORE_MIN_AUTO_SELL_ITEMS as i32 - stores()[store_id].unique_items_counter as i32;
            }

            let max_cost = STORE_OWNERS[stores()[store_id].owner_id as usize].max_cost;

            turnaround -= 1;
            while turnaround >= 0 {
                store_item_create(store_id, max_cost);
                turnaround -= 1;
            }
        }
    }
}

// Returns the value for any given object -RAK-
pub fn store_item_value(item: &Inventory) -> i32 {
    let mut value;

    if (item.identification & config::identification::ID_DAMD) != 0 {
        // don't purchase known cursed items
        value = 0;
    } else if (item.category_id >= TV_BOW && item.category_id <= TV_SWORD) || (item.category_id >= TV_BOOTS && item.category_id <= TV_SOFT_ARMOR) {
        value = get_weapon_armor_buy_price(item);
    } else if item.category_id >= TV_SLING_AMMO && item.category_id <= TV_SPIKE {
        value = get_ammo_buy_price(item);
    } else if item.category_id == TV_SCROLL1 || item.category_id == TV_SCROLL2 || item.category_id == TV_POTION1 || item.category_id == TV_POTION2 {
        value = get_potion_scroll_buy_price(item);
    } else if item.category_id == TV_FOOD {
        value = get_food_buy_price(item);
    } else if item.category_id == TV_AMULET || item.category_id == TV_RING {
        value = get_ring_amulet_buy_price(item);
    } else if item.category_id == TV_STAFF || item.category_id == TV_WAND {
        value = get_wand_staff_buy_price(item);
    } else if item.category_id == TV_DIGGING {
        value = get_pick_shovel_buy_price(item);
    } else {
        value = item.cost;
    }

    // Multiply value by number of items if it is a group stack item.
    // Do not include torches here.
    if item.sub_category_id > ITEM_GROUP_MIN {
        value *= item.items_count as i32;
    }

    value
}

fn get_weapon_armor_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.category_id >= TV_BOW && item.category_id <= TV_SWORD {
        if item.to_hit < 0 || item.to_damage < 0 || item.to_ac < 0 {
            return 0;
        }

        return item.cost + (item.to_hit as i32 + item.to_damage as i32 + item.to_ac as i32) * 100;
    }

    if item.to_ac < 0 {
        return 0;
    }

    item.cost + item.to_ac as i32 * 100
}

fn get_ammo_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.to_hit < 0 || item.to_damage < 0 || item.to_ac < 0 {
        return 0;
    }

    // use 5, because missiles generally appear in groups of 20,
    // so 20 * 5 == 100, which is comparable to weapon bonus above
    item.cost + (item.to_hit as i32 + item.to_damage as i32 + item.to_ac as i32) * 5
}

fn get_potion_scroll_buy_price(item: &Inventory) -> i32 {
    if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        return 20;
    }

    item.cost
}

fn get_food_buy_price(item: &Inventory) -> i32 {
    if (item.sub_category_id as usize) < ITEM_SINGLE_STACK_MIN as usize + MAX_MUSHROOMS && !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        return 1;
    }

    item.cost
}

fn get_ring_amulet_buy_price(item: &Inventory) -> i32 {
    // player does not know what type of ring/amulet this is
    if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        return 45;
    }

    // player knows what type of ring, but does not know whether it
    // is cursed or not, if refuse to buy cursed objects here, then
    // player can use this to 'identify' cursed objects
    if !spell_item_identified(item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    item.cost
}

fn get_wand_staff_buy_price(item: &Inventory) -> i32 {
    if !item_set_colorless_as_identified(item.category_id, item.sub_category_id, item.identification) {
        if item.category_id == TV_WAND {
            return 50;
        }

        return 70;
    }

    if spell_item_identified(item) {
        return item.cost + (item.cost / 20) * item.misc_use as i32;
    }

    item.cost
}

fn get_pick_shovel_buy_price(item: &Inventory) -> i32 {
    if !spell_item_identified(item) {
        return GAME_OBJECTS[item.id as usize].cost;
    }

    if item.misc_use < 0 {
        return 0;
    }

    // some digging tools start with non-zero `misc_use` values, so only
    // multiply the plusses by 100, make sure result is positive
    let mut value = item.cost + (item.misc_use as i32 - GAME_OBJECTS[item.id as usize].misc_use as i32) * 100;

    if value < 0 {
        value = 0;
    }

    value
}

// Asking price for an item -RAK-
pub fn store_item_sell_price(store: &Store, min_price: &mut i32, max_price: &mut i32, item: &Inventory) -> i32 {
    let mut price = store_item_value(item);

    // check `item.cost` in case it is cursed, check `price` in case it is damaged
    // don't let the item get into the store inventory
    if item.cost < 1 || price < 1 {
        return 0;
    }

    let owner = &STORE_OWNERS[store.owner_id as usize];

    price = price * RACE_GOLD_ADJUSTMENTS[owner.race as usize][py().misc.race_id as usize] as i32 / 100;
    if price < 1 {
        price = 1;
    }

    *max_price = price * owner.max_inflate as i32 / 100;
    *min_price = price * owner.min_inflate as i32 / 100;

    if *min_price > *max_price {
        *min_price = *max_price;
    }

    price
}

// Check to see if they will be carrying too many objects -RAK-
pub fn store_check_player_items_count(store: &Store, item: &Inventory) -> bool {
    if (store.unique_items_counter as usize) < STORE_MAX_DISCRETE_ITEMS {
        return true;
    }

    if !inventory_item_stackable(item) {
        return false;
    }

    let mut store_check = false;

    for i in 0..store.unique_items_counter as usize {
        let store_item = store.inventory[i].item;

        // note: items with sub_category_id of gte ITEM_SINGLE_STACK_MAX only stack
        // if their `sub_category_id`s match
        if store_item.category_id == item.category_id
            && store_item.sub_category_id == item.sub_category_id
            && (store_item.items_count as i32 + item.items_count as i32) < 256
            && (item.sub_category_id < ITEM_GROUP_MIN || store_item.misc_use == item.misc_use)
        {
            store_check = true;
        }
    }

    store_check
}

// Insert INVEN_MAX at given location
fn store_item_insert(store_id: usize, pos: usize, i_cost: i32, item: &Inventory) {
    let count = stores()[store_id].unique_items_counter as usize;

    for i in (pos..count).rev() {
        stores()[store_id].inventory[i + 1] = stores()[store_id].inventory[i];
    }

    stores()[store_id].inventory[pos].item = *item;
    stores()[store_id].inventory[pos].cost = -i_cost;
    stores()[store_id].unique_items_counter += 1;
}

// Add the item in INVEN_MAX to stores inventory. -RAK-
pub fn store_carry_item(store_id: i32, index_id: &mut i32, item: &mut Inventory) {
    *index_id = -1;

    let store_id = store_id as usize;
    let store = stores()[store_id];

    let mut dummy = 0;
    let mut item_cost = 0;
    if store_item_sell_price(&store, &mut dummy, &mut item_cost, item) < 1 {
        return;
    }

    let mut item_id: usize = 0;
    let item_num = item.items_count;
    let item_category = item.category_id;
    let item_sub_catagory = item.sub_category_id;

    let mut flag = false;
    loop {
        let mut store_item = stores()[store_id].inventory[item_id].item;

        if item_category == store_item.category_id {
            if item_sub_catagory == store_item.sub_category_id // Adds to other item
                && item_sub_catagory >= ITEM_SINGLE_STACK_MIN
                && (item_sub_catagory < ITEM_GROUP_MIN || store_item.misc_use == item.misc_use)
            {
                *index_id = item_id as i32;
                store_item.items_count += item_num;

                // must set new cost for group items, do this only for items
                // strictly greater than group_min, not for torches, this
                // must be recalculated for entire group
                if item_sub_catagory > ITEM_GROUP_MIN {
                    store_item_sell_price(&stores()[store_id], &mut dummy, &mut item_cost, &store_item);
                    stores()[store_id].inventory[item_id].cost = -item_cost;
                } else if store_item.items_count > 24 {
                    // must let group objects (except torches) stack over 24
                    // since there may be more than 24 in the group
                    store_item.items_count = 24;
                }
                stores()[store_id].inventory[item_id].item = store_item;
                flag = true;
            }
        } else if item_category > store_item.category_id {
            // Insert into list
            store_item_insert(store_id, item_id, item_cost, item);
            flag = true;
            *index_id = item_id as i32;
        }
        item_id += 1;

        if flag || item_id >= stores()[store_id].unique_items_counter as usize {
            break;
        }
    }

    // Becomes last item in list
    if !flag {
        let count = stores()[store_id].unique_items_counter as usize;
        store_item_insert(store_id, count, item_cost, item);
        *index_id = stores()[store_id].unique_items_counter as i32 - 1;
    }
}

// Destroy an item in the stores inventory.  Note that if
// `only_one_of` is false, an entire slot is destroyed -RAK-
pub fn store_destroy_item(store_id: i32, item_id: i32, only_one_of: bool) {
    let store_id = store_id as usize;
    let item_id = item_id as usize;

    let store_item = stores()[store_id].inventory[item_id].item;

    // for single stackable objects, only destroy one half on average,
    // this will help ensure that general store and alchemist have
    // reasonable selection of objects
    let number: u8 = if inventory_item_single_stackable(&store_item) {
        if only_one_of {
            1
        } else {
            random_number(store_item.items_count as i32) as u8
        }
    } else {
        store_item.items_count
    };

    if number != store_item.items_count {
        stores()[store_id].inventory[item_id].item.items_count -= number;
    } else {
        let count = stores()[store_id].unique_items_counter as usize;
        for i in item_id..count - 1 {
            stores()[store_id].inventory[i] = stores()[store_id].inventory[i + 1];
        }
        let mut nothing = Inventory::empty();
        inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut nothing);
        stores()[store_id].inventory[count - 1].item = nothing;
        stores()[store_id].inventory[count - 1].cost = 0;
        stores()[store_id].unique_items_counter -= 1;
    }
}

// Creates an item and inserts it into store's inven -RAK-
fn store_item_create(store_id: usize, max_cost: i16) {
    let free_id = popt();

    for _tries in 0..=3 {
        let id = STORE_CHOICES[store_id][(random_number(STORE_MAX_ITEM_TYPES as i32) - 1) as usize];
        inventory_item_copy_to(id as usize, &mut game().treasure.list[free_id as usize]);
        magic_treasure_magical_ability(free_id, config::treasure::LEVEL_TOWN_OBJECTS as i32);

        let item = game().treasure.list[free_id as usize];

        if store_check_player_items_count(&stores()[store_id], &item) {
            // Item must be good: cost > 0.
            if item.cost > 0 && item.cost < max_cost as i32 {
                // equivalent to calling spellIdentifyItem(),
                // except will not change the objects_identified array.
                item_identify_as_store_bought(&mut game().treasure.list[free_id as usize]);

                let mut dummy = 0;
                let mut carried_item = game().treasure.list[free_id as usize];
                store_carry_item(store_id as i32, &mut dummy, &mut carried_item);

                break;
            }
        }
    }

    pusht(free_id as u8);
}
