// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::globals::RacyCell;
use crate::inventory::Inventory;

pub const MAX_OWNERS: usize = 18; // Number of owners to choose from
pub const MAX_STORES: usize = 6; // Number of different stores
pub const STORE_MAX_DISCRETE_ITEMS: usize = 24; // Max number of discrete objects in inventory
pub const STORE_MAX_ITEM_TYPES: usize = 26; // Number of items to choose stock from
pub const COST_ADJUSTMENT: i32 = 100; // Adjust prices for buying and selling

// InventoryRecord data for a store inventory item
#[derive(Debug, Clone, Copy, Default)]
pub struct InventoryRecord {
    pub cost: i32,
    pub item: Inventory,
}

impl InventoryRecord {
    pub const fn empty() -> Self {
        InventoryRecord {
            cost: 0,
            item: Inventory::empty(),
        }
    }
}

// Store holds all the data for any given store in the game
#[derive(Debug, Clone, Copy)]
pub struct Store {
    pub turns_left_before_closing: i32,
    pub insults_counter: i16,
    pub owner_id: u8,
    pub unique_items_counter: u8,
    pub good_purchases: u16,
    pub bad_purchases: u16,
    pub inventory: [InventoryRecord; STORE_MAX_DISCRETE_ITEMS],
}

impl Store {
    pub const fn empty() -> Self {
        Store {
            turns_left_before_closing: 0,
            insults_counter: 0,
            owner_id: 0,
            unique_items_counter: 0,
            good_purchases: 0,
            bad_purchases: 0,
            inventory: [InventoryRecord::empty(); STORE_MAX_DISCRETE_ITEMS],
        }
    }
}

// Owner holds data about a given store owner
pub struct Owner {
    pub name: &'static str,
    pub max_cost: i16,
    pub max_inflate: u8,
    pub min_inflate: u8,
    pub haggles_per: u8,
    pub race: u8,
    pub max_insults: u8,
}

static STORES: RacyCell<[Store; MAX_STORES]> = RacyCell::new([Store::empty(); MAX_STORES]);

pub fn stores() -> &'static mut [Store; MAX_STORES] {
    STORES.get()
}
