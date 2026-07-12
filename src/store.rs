// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Store: entering, command interpreter, buying, selling

use crate::config;
use crate::data_store_owners::{
    SPEECH_BUYING_HAGGLE, SPEECH_BUYING_HAGGLE_FINAL, SPEECH_GET_OUT_OF_MY_STORE, SPEECH_HAGGLING_TRY_AGAIN, SPEECH_INSULTED_HAGGLING_DONE, SPEECH_SALE_ACCEPTED,
    SPEECH_SELLING_HAGGLE, SPEECH_SELLING_HAGGLE_FINAL, SPEECH_SORRY, STORE_OWNERS,
};
use crate::dungeon::dg;
use crate::game::{game, random_number};
use crate::globals::RacyCell;
use crate::helpers::{insert_number_into_string, string_to_number};
use crate::identification::{item_description, item_identify, spell_item_identify_and_remove_random_inscription};
use crate::inventory::{
    inventory_can_carry_item_count, inventory_carry_item, inventory_destroy_item, inventory_item_single_stackable, inventory_take_one_item, Inventory, PlayerEquipment,
};
use crate::player::{player_strength, py, A_CHR};
use crate::player_stats::player_stat_adjustment_charisma;
use crate::store_data::{stores, Owner, Store, MAX_OWNERS, MAX_STORES};
use crate::store_inventory::{store_carry_item, store_check_player_items_count, store_destroy_item, store_item_sell_price, store_item_value};
use crate::treasure::*;
use crate::types::Coord;
use crate::ui::draw_cave_panel;
use crate::ui_inventory::{inventory_execute_command, inventory_get_input_for_item_id};
use crate::ui_io::{
    clear_screen, erase_line, get_command, get_menu_item_id, get_string_input, message_line_clear, message_ready_to_print, move_cursor, print_message, put_string,
    put_string_clear_to_eol, terminal_bell_sound,
};

const WIELD: usize = PlayerEquipment::Wield as usize;

// Save the store's last increment value.
static STORE_LAST_INCREMENT: RacyCell<i16> = RacyCell::new(0);

fn store_last_increment() -> &'static mut i16 {
    STORE_LAST_INCREMENT.get()
}

// The status of the customer bid.
// Note: a received bid may still result in a rejected offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BidState {
    Received, // the bid was received successfully
    Rejected, // the bid was rejected, or cancelled by the customer
    Offended, // customer tried to sell an undesirable item
    Insulted, // the store owner was insulted too many times by the bid
}

// Initializes the stores with owners -RAK-
pub fn store_initialize_owners() {
    let count = (MAX_OWNERS / MAX_STORES) as i32;

    for store_id in 0..MAX_STORES {
        stores()[store_id].owner_id = (MAX_STORES * (random_number(count) as usize - 1) + store_id) as u8;
        stores()[store_id].insults_counter = 0;
        stores()[store_id].turns_left_before_closing = 0;
        stores()[store_id].unique_items_counter = 0;
        stores()[store_id].good_purchases = 0;
        stores()[store_id].bad_purchases = 0;

        for item in stores()[store_id].inventory.iter_mut() {
            crate::inventory::inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut item.item);
            item.cost = 0;
        }
    }
}

// Comments vary. -RAK-
// Comment one : Finished haggling
fn print_speech_finished_haggling() {
    print_message(Some(SPEECH_SALE_ACCEPTED[(random_number(14) - 1) as usize]));
}

// %A1 is offer, %A2 is asking.
fn print_speech_selling_haggle(offer: i32, asking: i32, final_offer: i32) {
    let mut comment = if final_offer > 0 {
        SPEECH_SELLING_HAGGLE_FINAL[(random_number(3) - 1) as usize].to_string()
    } else {
        SPEECH_SELLING_HAGGLE[(random_number(16) - 1) as usize].to_string()
    };

    insert_number_into_string(&mut comment, "%A1", offer, false);
    insert_number_into_string(&mut comment, "%A2", asking, false);
    print_message(Some(&comment));
}

fn print_speech_buying_haggle(offer: i32, asking: i32, final_offer: i32) {
    let mut comment = if final_offer > 0 {
        SPEECH_BUYING_HAGGLE_FINAL[(random_number(3) - 1) as usize].to_string()
    } else {
        SPEECH_BUYING_HAGGLE[(random_number(15) - 1) as usize].to_string()
    };

    insert_number_into_string(&mut comment, "%A1", offer, false);
    insert_number_into_string(&mut comment, "%A2", asking, false);
    print_message(Some(&comment));
}

// Kick 'da bum out. -RAK-
fn print_speech_get_out_of_my_store() {
    let comment = (random_number(5) - 1) as usize;
    print_message(Some(SPEECH_INSULTED_HAGGLING_DONE[comment]));
    print_message(Some(SPEECH_GET_OUT_OF_MY_STORE[comment]));
}

fn print_speech_try_again() {
    print_message(Some(SPEECH_HAGGLING_TRY_AGAIN[(random_number(10) - 1) as usize]));
}

fn print_speech_sorry() {
    print_message(Some(SPEECH_SORRY[(random_number(5) - 1) as usize]));
}

// Displays the set of commands -RAK-
fn display_store_commands() {
    put_string_clear_to_eol("You may:", Coord::new(20, 0));
    put_string_clear_to_eol(" p) Purchase an item.           b) Browse store's inventory.", Coord::new(21, 0));
    put_string_clear_to_eol(" s) Sell an item.               i/e/t/w/x) Inventory/Equipment Lists.", Coord::new(22, 0));
    put_string_clear_to_eol("ESC) Exit from Building.        ^R) Redraw the screen.", Coord::new(23, 0));
}

// Displays the set of commands -RAK-
fn display_store_haggle_commands(haggle_type: i32) {
    if haggle_type == -1 {
        put_string_clear_to_eol("Specify an asking-price in gold pieces.", Coord::new(21, 0));
    } else {
        put_string_clear_to_eol("Specify an offer in gold pieces.", Coord::new(21, 0));
    }

    put_string_clear_to_eol("ESC) Quit Haggling.", Coord::new(22, 0));
    erase_line(Coord::new(23, 0)); // clear last line
}

// Displays a store's inventory -RAK-
fn display_store_inventory(store_id: usize, item_pos_start: i32) {
    let store = stores()[store_id];

    let mut item_pos_end = ((item_pos_start / 12) + 1) * 12;
    if item_pos_end > store.unique_items_counter as i32 {
        item_pos_end = store.unique_items_counter as i32;
    }

    let mut item_pos_start = item_pos_start;
    let mut item_line_num = item_pos_start % 12;

    while item_pos_start < item_pos_end {
        // work on a local copy: the original temporarily overwrote items_count
        // to force single-item stacks to describe as one item, then restored
        // it; since this is a copy there's nothing to restore.
        let mut item = store.inventory[item_pos_start as usize].item;

        if inventory_item_single_stackable(&item) {
            item.items_count = 1;
        }

        let description = item_description(&item, true);

        let msg = format!("{}) {}", (b'a' + item_line_num as u8) as char, description);
        put_string_clear_to_eol(&msg, Coord::new(item_line_num + 5, 0));

        let cost = store.inventory[item_pos_start as usize].cost;

        let msg = if cost <= 0 {
            let mut value = -cost;
            value = value * player_stat_adjustment_charisma() / 100;
            if value <= 0 {
                value = 1;
            }
            format!("{:9}", value)
        } else {
            format!("{:9} [Fixed]", cost)
        };

        put_string_clear_to_eol(&msg, Coord::new(item_line_num + 5, 59));
        item_pos_start += 1;
        item_line_num += 1;
    }

    if item_line_num < 12 {
        for i in 0..(11 - item_line_num + 1) {
            // clear remaining lines
            erase_line(Coord::new(i + item_line_num + 5, 0));
        }
    }

    if store.unique_items_counter > 12 {
        put_string("- cont. -", Coord::new(17, 60));
    } else {
        erase_line(Coord::new(17, 60));
    }
}

// Re-displays only a single cost -RAK-
fn display_single_cost(store_id: usize, item_id: usize) {
    let cost = stores()[store_id].inventory[item_id].cost;

    let msg = if cost < 0 {
        let mut c = -cost;
        c = c * player_stat_adjustment_charisma() / 100;
        format!("{}", c)
    } else {
        format!("{:9} [Fixed]", cost)
    };
    put_string_clear_to_eol(&msg, Coord::new((item_id as i32 % 12) + 5, 59));
}

// Displays players gold -RAK-
fn display_player_remaining_gold() {
    let msg = format!("Gold Remaining : {}", py().misc.au);
    put_string_clear_to_eol(&msg, Coord::new(18, 17));
}

// Displays store -RAK-
fn display_store(store_id: usize, owner_name: &str, current_top_item_id: i32) {
    clear_screen();
    put_string(owner_name, Coord::new(3, 9));
    put_string("Item", Coord::new(4, 3));
    put_string("Asking Price", Coord::new(4, 60));
    display_player_remaining_gold();
    display_store_commands();
    display_store_inventory(store_id, current_top_item_id);
}

// Get the ID of a store item and return it's value -RAK-
// Returns true if the item was found.
fn store_get_item_id(item_id: &mut i32, prompt: &str, item_pos_start: i32, item_pos_end: i32) -> bool {
    *item_id = -1;
    let mut item_found = false;

    let msg = format!(
        "(Items {}-{}, ESC to exit) {}",
        (b'a' + item_pos_start as u8) as char,
        (b'a' + item_pos_end as u8) as char,
        prompt
    );

    let mut key_char = '\0';
    while get_menu_item_id(&msg, &mut key_char) {
        let key_val = key_char as i32 - 'a' as i32;
        if key_val >= item_pos_start && key_val <= item_pos_end {
            item_found = true;
            *item_id = key_val;
            break;
        }
        terminal_bell_sound();
    }
    message_line_clear();

    item_found
}

// Increase the insult counter and get angry if too many -RAK-
fn store_increase_insults(store_id: usize) -> bool {
    stores()[store_id].insults_counter += 1;

    let insults_counter = stores()[store_id].insults_counter;
    let owner_id = stores()[store_id].owner_id;

    if insults_counter <= STORE_OWNERS[owner_id as usize].max_insults as i16 {
        return false;
    }

    // customer angered the store owner with too many insults!
    print_speech_get_out_of_my_store();
    stores()[store_id].insults_counter = 0;
    stores()[store_id].bad_purchases += 1;
    stores()[store_id].turns_left_before_closing = dg().game_turn + 2500 + random_number(2500);

    true
}

// Decrease insults -RAK-
fn store_decrease_insults(store_id: usize) {
    if stores()[store_id].insults_counter != 0 {
        stores()[store_id].insults_counter -= 1;
    }
}

// Have insulted while haggling -RAK-
// Returns true if the store owner was angered.
fn store_haggle_insults(store_id: usize) -> bool {
    if store_increase_insults(store_id) {
        return true;
    }

    print_speech_try_again();

    // keep insult separate from rest of haggle
    print_message(None);

    false
}

// Returns true if the customer made a valid offer
fn store_get_haggle(prompt: &str, new_offer: &mut i32, offer_count: i32) -> bool {
    let mut valid_offer = true;

    if offer_count == 0 {
        *store_last_increment() = 0;
    }

    let mut increment = false;
    let mut adjustment: i32 = 0;

    let start_len = prompt.chars().count() as i32;
    let mut prompt_len = start_len;

    // Get a customers new offer
    while valid_offer && adjustment == 0 {
        put_string_clear_to_eol(prompt, Coord::new(0, 0));

        if offer_count != 0 && *store_last_increment() != 0 {
            let abs_store_last_increment = (*store_last_increment() as i32).abs();
            let sign = if *store_last_increment() < 0 { '-' } else { '+' };

            let last_offer_str = format!("[{}{}] ", sign, abs_store_last_increment);
            put_string_clear_to_eol(&last_offer_str, Coord::new(0, start_len));

            prompt_len = start_len + last_offer_str.chars().count() as i32;
        }

        let mut msg = String::new();
        if !get_string_input(&mut msg, Coord::new(0, prompt_len), 40) {
            // customer aborted, i.e. pressed escape
            valid_offer = false;
        }

        let p = msg.trim_start();
        if p.starts_with('+') || p.starts_with('-') {
            increment = true;
        }

        if offer_count != 0 && increment {
            string_to_number(&msg, &mut adjustment);

            // Don't accept a zero here.  Turn off increment if it was zero
            // because a zero will not exit.  This can be zero if the user
            // did not type a number after the +/- sign.
            if adjustment == 0 {
                increment = false;
            } else {
                *store_last_increment() = adjustment as i16;
            }
        } else if offer_count != 0 && msg.is_empty() {
            adjustment = *store_last_increment() as i32;
            increment = true;
        } else {
            string_to_number(&msg, &mut adjustment);
        }

        // don't allow incremental haggling, if player has not made an offer yet
        if valid_offer && offer_count == 0 && increment {
            print_message(Some("You haven't even made your first offer yet!"));
            adjustment = 0;
            increment = false;
        }
    }

    if valid_offer {
        if increment {
            *new_offer += adjustment;
        } else {
            *new_offer = adjustment;
        }
    } else {
        message_line_clear();
    }

    valid_offer
}

fn store_receive_offer(store_id: usize, prompt: &str, new_offer: &mut i32, last_offer: i32, offer_count: i32, factor: i32) -> BidState {
    let mut status = BidState::Received;

    let mut done = false;
    while !done {
        if store_get_haggle(prompt, new_offer, offer_count) {
            // customer submitted valid offer
            if *new_offer * factor >= last_offer * factor {
                done = true;
            } else if store_haggle_insults(store_id) {
                // customer angered the store owner!
                status = BidState::Insulted;
                done = true;
            } else {
                // new_offer rejected, reset new_offer so that incremental
                // haggling works correctly
                *new_offer = last_offer;
            }
        } else {
            // customer aborted offer
            status = BidState::Rejected;
            done = true;
        }
    }

    status
}

fn store_purchase_customer_adjustment(min_sell: &mut i32, max_sell: &mut i32) {
    let charisma = player_stat_adjustment_charisma();

    *max_sell = *max_sell * charisma / 100;
    if *max_sell <= 0 {
        *max_sell = 1;
    }

    *min_sell = *min_sell * charisma / 100;
    if *min_sell <= 0 {
        *min_sell = 1;
    }
}

// Haggling routine -RAK-
fn store_purchase_haggle(store_id: usize, price: &mut i32, item: &Inventory) -> BidState {
    let mut status = BidState::Received;

    let mut new_price: i32 = 0;

    let store = stores()[store_id];
    let owner = &STORE_OWNERS[store.owner_id as usize];

    let mut max_sell: i32 = 0;
    let mut min_sell: i32 = 0;
    let cost = store_item_sell_price(&store, &mut min_sell, &mut max_sell, item);

    store_purchase_customer_adjustment(&mut min_sell, &mut max_sell);

    // cast max_inflate to signed so that subtraction works correctly
    let mut max_buy = cost * (200 - owner.max_inflate as i32) / 100;
    if max_buy <= 0 {
        max_buy = 1;
    }

    display_store_haggle_commands(1);

    let final_asking_price = min_sell;
    let mut current_asking_price = max_sell;

    let mut comment = "Asking".to_string();
    let mut accepted_without_haggle = false;
    let mut offers_count = 0; // this prevents incremental haggling on first try

    // go right to final price if player has bargained well
    if store_no_need_to_bargain(&store, final_asking_price) {
        print_message(Some("After a long bargaining session, you agree upon the price."));
        current_asking_price = min_sell;
        comment = "Final offer".to_string();
        accepted_without_haggle = true;

        // Set up automatic increment, so that a return will accept the final price.
        *store_last_increment() = min_sell as i16;
        offers_count = 1;
    }

    let min_offer = max_buy;
    let mut last_offer = min_offer;
    let mut new_offer: i32 = 0;

    let min_per = owner.haggles_per as i32;
    let max_per_base = min_per * 3;

    let mut final_flag = 0;

    let mut rejected = false;

    while !rejected {
        let mut bidding_open;
        loop {
            bidding_open = true;

            let msg = format!("{} :  {}", comment, current_asking_price);
            put_string(&msg, Coord::new(1, 0));

            status = store_receive_offer(store_id, "What do you offer? ", &mut new_offer, last_offer, offers_count, 1);

            if status != BidState::Received {
                rejected = true;
            } else {
                // review the received bid

                if new_offer > current_asking_price {
                    print_speech_sorry();

                    // rejected, reset new_offer for incremental haggling
                    new_offer = last_offer;

                    // If the automatic increment is large enough to overflow,
                    // then the player must have made a mistake.  Clear it
                    // because it is useless.
                    if last_offer + (*store_last_increment() as i32) > current_asking_price {
                        *store_last_increment() = 0;
                    }
                } else if new_offer == current_asking_price {
                    rejected = true;
                    new_price = new_offer;
                } else {
                    bidding_open = false;
                }
            }

            if rejected || !bidding_open {
                break;
            }
        }

        if !rejected {
            let mut adjustment = (new_offer - last_offer) * 100 / (current_asking_price - last_offer);

            if adjustment < min_per {
                rejected = store_haggle_insults(store_id);
                if rejected {
                    status = BidState::Insulted;
                }
            } else if adjustment > max_per_base {
                adjustment = adjustment * 75 / 100;
                if adjustment < max_per_base {
                    adjustment = max_per_base;
                }
            }

            adjustment = ((current_asking_price - new_offer) * (adjustment + random_number(5) - 3) / 100) + 1;

            // don't let the price go up
            if adjustment > 0 {
                current_asking_price -= adjustment;
            }

            if current_asking_price < final_asking_price {
                current_asking_price = final_asking_price;
                comment = "Final Offer".to_string();

                // Set the automatic haggle increment so that RET will give
                // a new_offer equal to the final_asking_price price.
                *store_last_increment() = (final_asking_price - new_offer) as i16;
                final_flag += 1;

                if final_flag > 3 {
                    if store_increase_insults(store_id) {
                        status = BidState::Insulted;
                    } else {
                        status = BidState::Rejected;
                    }
                    rejected = true;
                }
            } else if new_offer >= current_asking_price {
                rejected = true;
                new_price = new_offer;
            }

            if !rejected {
                last_offer = new_offer;
                offers_count += 1; // enable incremental haggling

                erase_line(Coord::new(1, 0));
                let msg = format!("Your last offer : {}", last_offer);
                put_string(&msg, Coord::new(1, 39));

                print_speech_selling_haggle(last_offer, current_asking_price, final_flag);

                // If the current increment would take you over the store's
                // price, then decrease it to an exact match.
                if current_asking_price - last_offer < *store_last_increment() as i32 {
                    *store_last_increment() = (current_asking_price - last_offer) as i16;
                }
            }
        }
    }

    // update bargaining info
    if status == BidState::Received && !accepted_without_haggle {
        store_update_bargaining_skills(store_id, new_price, final_asking_price);
    }

    *price = new_price; // update callers price before returning

    status
}

fn store_sell_customer_adjustment(owner: &Owner, cost: &mut i32, min_buy: &mut i32, max_buy: &mut i32, max_sell: &mut i32) {
    *cost = *cost * (200 - player_stat_adjustment_charisma()) / 100;
    *cost = *cost * (200 - crate::data_stores::RACE_GOLD_ADJUSTMENTS[owner.race as usize][py().misc.race_id as usize] as i32) / 100;
    if *cost < 1 {
        *cost = 1;
    }

    *max_sell = *cost * owner.max_inflate as i32 / 100;

    // cast max_inflate to signed so that subtraction works correctly
    *max_buy = *cost * (200 - owner.max_inflate as i32) / 100;
    *min_buy = *cost * (200 - owner.min_inflate as i32) / 100;
    if *min_buy < 1 {
        *min_buy = 1;
    }
    if *max_buy < 1 {
        *max_buy = 1;
    }
    if *min_buy < *max_buy {
        *min_buy = *max_buy;
    }
}

// Haggling routine -RAK-
fn store_sell_haggle(store_id: usize, price: &mut i32, item: &Inventory) -> BidState {
    let mut status = BidState::Received;

    let mut new_price: i32 = 0;

    let store = stores()[store_id];
    let mut cost = store_item_value(item);

    let mut rejected = false;

    let mut max_gold: i32 = 0;
    let mut min_per: i32 = 0;
    let mut max_per: i32 = 0;
    let mut max_sell: i32 = 0;
    let mut min_buy: i32 = 0;
    let mut max_buy: i32 = 0;

    if cost < 1 {
        status = BidState::Offended;
        rejected = true;
    } else {
        let owner = &STORE_OWNERS[store.owner_id as usize];

        store_sell_customer_adjustment(owner, &mut cost, &mut min_buy, &mut max_buy, &mut max_sell);

        min_per = owner.haggles_per as i32;
        max_per = min_per * 3;
        max_gold = owner.max_cost as i32;
    }

    let mut final_asking_price = 0;
    let mut current_asking_price = 0;

    let mut final_flag = 0;

    let mut comment = String::new();
    let mut accepted_without_haggle = false;

    if !rejected {
        display_store_haggle_commands(-1);

        let mut offer_count = 0; // this prevents incremental haggling on first try

        if max_buy > max_gold {
            final_flag = 1;
            comment = "Final Offer".to_string();

            // Disable the automatic haggle increment on RET.
            *store_last_increment() = 0;
            current_asking_price = max_gold;
            final_asking_price = max_gold;
            print_message(Some("I am sorry, but I have not the money to afford such a fine item."));
            accepted_without_haggle = true;
        } else {
            current_asking_price = max_buy;
            final_asking_price = min_buy;

            if final_asking_price > max_gold {
                final_asking_price = max_gold;
            }

            comment = "Offer".to_string();

            // go right to final price if player has bargained well
            if store_no_need_to_bargain(&store, final_asking_price) {
                print_message(Some("After a long bargaining session, you agree upon the price."));
                current_asking_price = final_asking_price;
                comment = "Final offer".to_string();
                accepted_without_haggle = true;

                // Set up automatic increment, so that a return
                // will accept the final price.
                *store_last_increment() = final_asking_price as i16;
                offer_count = 1;
            }
        }

        let min_offer = max_sell;
        let mut last_offer = min_offer;
        let mut new_offer: i32 = 0;

        if current_asking_price < 1 {
            current_asking_price = 1;
        }

        loop {
            let mut bidding_open;
            loop {
                bidding_open = true;

                let msg = format!("{} :  {}", comment, current_asking_price);
                put_string(&msg, Coord::new(1, 0));

                status = store_receive_offer(store_id, "What price do you ask? ", &mut new_offer, last_offer, offer_count, -1);

                if status != BidState::Received {
                    rejected = true;
                } else {
                    // review the received bid

                    if new_offer < current_asking_price {
                        print_speech_sorry();

                        // rejected, reset new_offer for incremental haggling
                        new_offer = last_offer;

                        // If the automatic increment is large enough to
                        // overflow, then the player must have made a mistake.
                        // Clear it because it is useless.
                        if last_offer + (*store_last_increment() as i32) < current_asking_price {
                            *store_last_increment() = 0;
                        }
                    } else if new_offer == current_asking_price {
                        rejected = true;
                        new_price = new_offer;
                    } else {
                        bidding_open = false;
                    }
                }

                if rejected || !bidding_open {
                    break;
                }
            }

            if !rejected {
                let mut adjustment = (last_offer - new_offer) * 100 / (last_offer - current_asking_price);

                if adjustment < min_per {
                    rejected = store_haggle_insults(store_id);
                    if rejected {
                        status = BidState::Insulted;
                    }
                } else if adjustment > max_per {
                    adjustment = adjustment * 75 / 100;
                    if adjustment < max_per {
                        adjustment = max_per;
                    }
                }

                adjustment = ((new_offer - current_asking_price) * (adjustment + random_number(5) - 3) / 100) + 1;

                // don't let the price go down
                if adjustment > 0 {
                    current_asking_price += adjustment;
                }

                if current_asking_price > final_asking_price {
                    current_asking_price = final_asking_price;
                    comment = "Final Offer".to_string();

                    // Set the automatic haggle increment so that RET will give
                    // a new_offer equal to the final_asking_price price.
                    *store_last_increment() = (final_asking_price - new_offer) as i16;
                    final_flag += 1;

                    if final_flag > 3 {
                        if store_increase_insults(store_id) {
                            status = BidState::Insulted;
                        } else {
                            status = BidState::Rejected;
                        }
                        rejected = true;
                    }
                } else if new_offer <= current_asking_price {
                    rejected = true;
                    new_price = new_offer;
                }

                if !rejected {
                    last_offer = new_offer;
                    offer_count += 1; // enable incremental haggling

                    erase_line(Coord::new(1, 0));
                    let msg = format!("Your last bid {}", last_offer);
                    put_string(&msg, Coord::new(1, 39));

                    print_speech_buying_haggle(current_asking_price, last_offer, final_flag);

                    // If the current decrement would take you under the store's
                    // price, then increase it to an exact match.
                    if current_asking_price - last_offer > *store_last_increment() as i32 {
                        *store_last_increment() = (current_asking_price - last_offer) as i16;
                    }
                }
            }

            if rejected {
                break;
            }
        }
    }

    // update bargaining info
    if status == BidState::Received && !accepted_without_haggle {
        store_update_bargaining_skills(store_id, new_price, final_asking_price);
    }

    *price = new_price; // update callers price before returning

    status
}

// Get the number of store items to display on the screen
fn store_items_to_display(store_counter: i32, current_top_item_id: i32) -> i32 {
    if current_top_item_id == 12 {
        return store_counter - 1 - 12;
    }

    if store_counter > 11 {
        return 11;
    }

    store_counter - 1
}

// Buy an item from a store -RAK-
// Returns true is the owner kicks out the customer
fn store_purchase_an_item(store_id: usize, current_top_item_id: &mut i32) -> bool {
    let mut kick_customer = false; // don't kick them out of the store!

    if stores()[store_id].unique_items_counter < 1 {
        print_message(Some("I am currently out of stock."));
        return false;
    }

    let mut item_id: i32 = 0;
    let item_count = store_items_to_display(stores()[store_id].unique_items_counter as i32, *current_top_item_id);
    if !store_get_item_id(&mut item_id, "Which item are you interested in? ", 0, item_count) {
        return false;
    }

    // Get the item number to be bought

    item_id += *current_top_item_id; // true item_id

    let mut sell_item = Inventory::empty();
    let store_item = stores()[store_id].inventory[item_id as usize].item;
    inventory_take_one_item(&mut sell_item, &store_item);

    if !inventory_can_carry_item_count(&sell_item) {
        put_string_clear_to_eol("You cannot carry that many different items.", Coord::new(0, 0));
        return false;
    }

    let mut status = BidState::Received;
    let mut price: i32 = 0;

    let item_cost = stores()[store_id].inventory[item_id as usize].cost;
    if item_cost > 0 {
        price = item_cost;
    } else {
        status = store_purchase_haggle(store_id, &mut price, &sell_item);
    }

    if status == BidState::Insulted {
        kick_customer = true;
    } else if status == BidState::Received {
        if py().misc.au >= price {
            print_speech_finished_haggling();
            store_decrease_insults(store_id);
            py().misc.au -= price;

            let new_item_id = inventory_carry_item(&mut sell_item);
            let saved_store_counter = stores()[store_id].unique_items_counter;

            store_destroy_item(store_id as i32, item_id, true);

            let description = item_description(&py().inventory[new_item_id as usize], true);
            let msg = format!("You have {} ({})", description, (b'a' + new_item_id as u8) as char);
            put_string_clear_to_eol(&msg, Coord::new(0, 0));

            player_strength();

            if *current_top_item_id >= stores()[store_id].unique_items_counter as i32 {
                *current_top_item_id = 0;
                display_store_inventory(store_id, *current_top_item_id);
            } else if saved_store_counter == stores()[store_id].unique_items_counter {
                if stores()[store_id].inventory[item_id as usize].cost < 0 {
                    stores()[store_id].inventory[item_id as usize].cost = price;
                    display_single_cost(store_id, item_id as usize);
                }
            } else {
                display_store_inventory(store_id, item_id);
            }
            display_player_remaining_gold();
        } else if store_increase_insults(store_id) {
            kick_customer = true;
        } else {
            print_speech_finished_haggling();
            print_message(Some("Liar!  You have not the gold!"));
        }
    }

    // Less intuitive, but looks better here than in storePurchaseHaggle.
    display_store_commands();
    erase_line(Coord::new(1, 0));

    kick_customer
}

// Functions to emulate the original Pascal sets
fn set_general_store_items(item_id: u8) -> bool {
    item_id == TV_DIGGING || item_id == TV_BOOTS || item_id == TV_CLOAK || item_id == TV_FOOD || item_id == TV_FLASK || item_id == TV_LIGHT || item_id == TV_SPIKE
}

fn set_armory_items(item_id: u8) -> bool {
    item_id == TV_BOOTS || item_id == TV_GLOVES || item_id == TV_HELM || item_id == TV_SHIELD || item_id == TV_HARD_ARMOR || item_id == TV_SOFT_ARMOR
}

fn set_weaponsmith_items(item_id: u8) -> bool {
    item_id == TV_SLING_AMMO || item_id == TV_BOLT || item_id == TV_ARROW || item_id == TV_BOW || item_id == TV_HAFTED || item_id == TV_POLEARM || item_id == TV_SWORD
}

fn set_temple_items(item_id: u8) -> bool {
    item_id == TV_HAFTED || item_id == TV_SCROLL1 || item_id == TV_SCROLL2 || item_id == TV_POTION1 || item_id == TV_POTION2 || item_id == TV_PRAYER_BOOK
}

fn set_alchemist_items(item_id: u8) -> bool {
    item_id == TV_SCROLL1 || item_id == TV_SCROLL2 || item_id == TV_POTION1 || item_id == TV_POTION2
}

fn set_magic_shop_items(item_id: u8) -> bool {
    item_id == TV_AMULET
        || item_id == TV_RING
        || item_id == TV_STAFF
        || item_id == TV_WAND
        || item_id == TV_SCROLL1
        || item_id == TV_SCROLL2
        || item_id == TV_POTION1
        || item_id == TV_POTION2
        || item_id == TV_MAGIC_BOOK
}

// Each store will buy only certain items, based on TVAL
static STORE_BUY: [fn(u8) -> bool; MAX_STORES] = [
    set_general_store_items,
    set_armory_items,
    set_weaponsmith_items,
    set_temple_items,
    set_alchemist_items,
    set_magic_shop_items,
];

// Sell an item to the store -RAK-
// Returns true is the owner kicks out the customer
fn store_sell_an_item(store_id: usize, current_top_item_id: &mut i32) -> bool {
    let mut kick_customer = false; // don't kick them out of the store!

    let mut first_item = py().pack.unique_items as i32;
    let mut last_item: i32 = -1;

    let mut mask = [false; WIELD];

    for counter in 0..py().pack.unique_items as usize {
        let flag = STORE_BUY[store_id](py().inventory[counter].category_id);

        if flag {
            mask[counter] = true;

            if (counter as i32) < first_item {
                first_item = counter as i32;
            }
            if (counter as i32) > last_item {
                last_item = counter as i32;
            }
        } else {
            mask[counter] = false;
        }
    }

    if last_item == -1 {
        print_message(Some("You have nothing to sell to this store!"));
        return false;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, "Which one? ", first_item, last_item, Some(&mask), Some("I do not buy such items.")) {
        return false;
    }

    let mut sold_item = Inventory::empty();
    inventory_take_one_item(&mut sold_item, &py().inventory[item_id as usize]);

    let description = item_description(&sold_item, true);
    let msg = format!("Selling {} ({})", description, (b'a' + item_id as u8) as char);
    print_message(Some(&msg));

    let store = stores()[store_id];
    if !store_check_player_items_count(&store, &sold_item) {
        print_message(Some("I have not the room in my store to keep it."));
        return false;
    }

    let mut price: i32 = 0;

    let status = store_sell_haggle(store_id, &mut price, &sold_item);

    if status == BidState::Insulted {
        kick_customer = true;
    } else if status == BidState::Offended {
        print_message(Some("How dare you!"));
        print_message(Some("I will not buy that!"));
        kick_customer = store_increase_insults(store_id);
    } else if status == BidState::Received {
        // bid received, and accepted!

        print_speech_finished_haggling();
        store_decrease_insults(store_id);
        py().misc.au += price;

        // identify object in inventory to set objects_identified array
        let mut item_id_usize = item_id as usize;
        item_identify(&mut item_id_usize);
        item_id = item_id_usize as i32;

        // retake sold_item so that it will be identified
        inventory_take_one_item(&mut sold_item, &py().inventory[item_id as usize]);

        // call spellItemIdentifyAndRemoveRandomInscription for store item, so charges/pluses are known
        spell_item_identify_and_remove_random_inscription(&mut sold_item);
        inventory_destroy_item(item_id as usize);

        let description = item_description(&sold_item, true);
        let msg = format!("You've sold {}", description);
        print_message(Some(&msg));

        let mut item_pos_id: i32 = -1;
        store_carry_item(store_id as i32, &mut item_pos_id, &mut sold_item);

        player_strength();

        if item_pos_id >= 0 {
            if item_pos_id < 12 {
                if *current_top_item_id < 12 {
                    display_store_inventory(store_id, item_pos_id);
                } else {
                    *current_top_item_id = 0;
                    display_store_inventory(store_id, *current_top_item_id);
                }
            } else if *current_top_item_id > 11 {
                display_store_inventory(store_id, item_pos_id);
            } else {
                *current_top_item_id = 12;
                display_store_inventory(store_id, *current_top_item_id);
            }
        }
        display_player_remaining_gold();
    }

    // Less intuitive, but looks better here than in storeSellHaggle.
    erase_line(Coord::new(1, 0));
    display_store_commands();

    kick_customer
}

// Entering a store -RAK-
pub fn store_enter(store_id: i32) {
    let store_id = store_id as usize;
    let store = stores()[store_id];

    if store.turns_left_before_closing >= dg().game_turn {
        print_message(Some("The doors are locked."));
        return;
    }

    let mut current_top_item_id: i32 = 0;
    display_store(store_id, STORE_OWNERS[store.owner_id as usize].name, current_top_item_id);

    let mut exit_store = false;
    while !exit_store {
        move_cursor(Coord::new(20, 9));

        // clear the msg flag just like we do in dungeon.c
        *message_ready_to_print() = false;

        let mut command = '\0';
        if get_command("", &mut command) {
            match command {
                'b' => {
                    if current_top_item_id == 0 {
                        if stores()[store_id].unique_items_counter > 12 {
                            current_top_item_id = 12;
                            display_store_inventory(store_id, current_top_item_id);
                        } else {
                            print_message(Some("Entire inventory is shown."));
                        }
                    } else {
                        current_top_item_id = 0;
                        display_store_inventory(store_id, current_top_item_id);
                    }
                }
                'E' | 'e' | 'I' | 'i' | 'T' | 't' | 'W' | 'w' | 'X' | 'x' => {
                    let saved_chr = py().stats.used[A_CHR];

                    loop {
                        inventory_execute_command(command);
                        command = game().doing_inventory_command;
                        if command == '\0' {
                            break;
                        }
                    }

                    // redisplay store prices if charisma changes
                    if saved_chr != py().stats.used[A_CHR] {
                        display_store_inventory(store_id, current_top_item_id);
                    }

                    game().player_free_turn = false; // No free moves here. -CJS-
                }
                'p' => {
                    exit_store = store_purchase_an_item(store_id, &mut current_top_item_id);
                }
                's' => {
                    exit_store = store_sell_an_item(store_id, &mut current_top_item_id);
                }
                _ => {
                    terminal_bell_sound();
                }
            }
        } else {
            exit_store = true;
        }
    }

    // Can't save and restore the screen because inventoryExecuteCommand() does that.
    draw_cave_panel();
}

// eliminate need to bargain if player has haggled well in the past -DJB-
fn store_no_need_to_bargain(store: &Store, min_price: i32) -> bool {
    if store.good_purchases == i16::MAX as u16 {
        return true;
    }

    let record = store.good_purchases as i32 - 3 * store.bad_purchases as i32 - 5;

    record > 0 && record * record > min_price / 50
}

// update the bargain info -DJB-
fn store_update_bargaining_skills(store_id: usize, price: i32, min_price: i32) {
    if min_price < 10 {
        return;
    }

    if price == min_price {
        if stores()[store_id].good_purchases < i16::MAX as u16 {
            stores()[store_id].good_purchases += 1;
        }
    } else if stores()[store_id].bad_purchases < i16::MAX as u16 {
        stores()[store_id].bad_purchases += 1;
    }
}
