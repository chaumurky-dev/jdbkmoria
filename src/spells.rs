// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player/creature spells, breaths, wands, scrolls, etc. code

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::{CLASSES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::dungeon::{
    cave_tile_visible, coord_distance_between, coord_in_bounds, dg, dungeon_delete_monster,
    dungeon_delete_object, dungeon_light_room, dungeon_lite_spot, dungeon_move_creature_record,
    dungeon_place_random_object_at, dungeon_remove_monster_from_level, dungeon_set_trap,
    trap_change_visibility, SCREEN_HEIGHT, SCREEN_WIDTH,
};
use crate::dungeon_los::los;
use crate::dungeon_tile::{
    MAX_CAVE_FLOOR, MAX_OPEN_SPACE, MIN_CAVE_WALL, MIN_CLOSED_SPACE, TILE_BLOCKED_FLOOR,
    TILE_BOUNDARY_WALL, TILE_CORR_FLOOR, TILE_DARK_FLOOR, TILE_GRANITE_WALL, TILE_LIGHT_FLOOR,
    TILE_MAGMA_WALL, TILE_QUARTZ_WALL,
};
use crate::game::{game, random_number};
use crate::game_objects::popt;
use crate::identification::{
    item_description, item_identify, spell_item_identified,
    spell_item_identify_and_remove_random_inscription, spell_item_remove_identification,
    item_identification_clear_empty, SpecialNameIds,
};
use crate::inventory::{
    damage_acid, damage_cold, damage_fire, damage_lightning_bolt, damage_poisoned_gas,
    inventory_destroy_item, inventory_find_range, inventory_item_copy_to, set_acid_destroyable_items,
    set_fire_destroyable_items, set_frost_destroyable_items, set_lightning_destroyable_items,
    set_null, Inventory, PlayerEquipment, PLAYER_INVENTORY_SIZE,
};
use crate::monster::{
    monster_death, monster_multiply, monster_levels, monster_name_description, monster_take_hit,
    monster_update_visibility, monsters, next_free_monster_id, print_monster_action_text,
    update_monsters, MON_MAX_LEVELS,
};
use crate::monster_manager::monster_place_new;
use crate::player::{
    py, player_calculate_allowed_spells_count, player_calculate_hit_points, player_gain_mana,
    player_move_position, player_recalculate_bonuses, player_stat_random_decrease,
    player_tunnel_wall, player_worn_item_is_cursed, player_worn_item_remove_curse, A_CHR, A_CON,
    A_DEX, A_INT, A_STR, A_WIS,
};
use crate::recall_data::creature_recall;
use crate::treasure::{
    TV_CHEST, TV_CLOSED_DOOR, TV_GOLD, TV_INVIS_TRAP, TV_MAX_OBJECT, TV_MAX_VISIBLE, TV_MIN_VISIBLE,
    TV_OPEN_DOOR, TV_RUBBLE, TV_SECRET_DOOR, TV_STAFF, TV_UP_STAIR, TV_VIS_TRAP, TV_WAND,
    TV_DOWN_STAIR,
};
use crate::types::Coord;
use crate::ui::{
    coord_inside_panel, display_character_experience, display_spells_list, draw_dungeon_panel,
    dungeon_reset_view, print_character_current_hit_points,
};
use crate::ui_inventory::{inventory_get_input_for_item_id, player_item_wearing_description};
use crate::ui_io::{
    get_input_confirmation, get_menu_item_id, get_tile_character, message_line_clear,
    panel_put_tile, print_message, put_qio, terminal_bell_sound, terminal_restore_screen,
    terminal_save_screen,
};
use crate::{tr, tr_fmt};

// Returns spell pointer -RAK-
fn spell_get_id(spell_ids: &[i32], spell_id: &mut i32, spell_chance: &mut i32, prompt: &str, first_spell: i32) -> bool {
    *spell_id = -1;

    let number_of_choices = spell_ids.len();

    let first_char = (spell_ids[0] + 'a' as i32 - first_spell) as u8 as char;
    let last_char = (spell_ids[number_of_choices - 1] + 'a' as i32 - first_spell) as u8 as char;
    let str = tr_fmt!("(Spells {}-{}, *=List, <ESCAPE>=exit) {}", first_char, last_char, prompt);

    let mut spell_found = false;
    let mut redraw = false;

    let offset: i32 = if CLASSES[py().misc.class_id as usize].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        config::spells::NAME_OFFSET_SPELLS as i32
    } else {
        config::spells::NAME_OFFSET_PRAYERS as i32
    };

    let mut spell_choice: char = '\0';
    while !spell_found && get_menu_item_id(&str, &mut spell_choice) {
        if spell_choice.is_ascii_uppercase() {
            *spell_id = spell_choice as i32 - 'A' as i32 + first_spell;

            // verify that this is in spells[], at most 22 entries in class_to_use_mage_spells[]
            let mut test_spell_id = 0;
            while test_spell_id < number_of_choices {
                if *spell_id == spell_ids[test_spell_id] {
                    break;
                }
                test_spell_id += 1;
            }

            if test_spell_id == number_of_choices {
                *spell_id = -2;
            } else {
                let spell = MAGIC_SPELLS[py().misc.class_id as usize - 1][*spell_id as usize];

                let tmp_str = tr_fmt!(
                    "Cast {} ({} mana, {}% fail)?",
                    tr!(SPELL_NAMES[(*spell_id + offset) as usize]),
                    spell.mana_required,
                    crate::mage_spells::spell_chance_of_success(*spell_id)
                );
                if get_input_confirmation(&tmp_str) {
                    spell_found = true;
                } else {
                    *spell_id = -1;
                }
            }
        } else if spell_choice.is_ascii_lowercase() {
            *spell_id = spell_choice as i32 - 'a' as i32 + first_spell;

            // verify that this is in spells[], at most 22 entries in class_to_use_mage_spells[]
            let mut test_spell_id = 0;
            while test_spell_id < number_of_choices {
                if *spell_id == spell_ids[test_spell_id] {
                    break;
                }
                test_spell_id += 1;
            }

            if test_spell_id == number_of_choices {
                *spell_id = -2;
            } else {
                spell_found = true;
            }
        } else if spell_choice == '*' {
            // only do this drawing once
            if !redraw {
                terminal_save_screen();
                redraw = true;
                display_spells_list(spell_ids, number_of_choices as i32, false, first_spell);
            }
        } else if spell_choice.is_ascii_alphabetic() {
            *spell_id = -2;
        } else {
            *spell_id = -1;
            terminal_bell_sound();
        }

        if *spell_id == -2 {
            let tmp_str = if offset == config::spells::NAME_OFFSET_SPELLS as i32 {
                tr!("You don't know that spell.")
            } else {
                tr!("You don't know that prayer.")
            };
            print_message(Some(tmp_str));
        }
    }

    if redraw {
        terminal_restore_screen();
    }

    message_line_clear();

    if spell_found {
        *spell_chance = crate::mage_spells::spell_chance_of_success(*spell_id);
    }

    spell_found
}

// Return spell number and failure chance -RAK-
// returns -1 if no spells in book
// returns  1 if choose a spell in book to cast
// returns  0 if don't choose a spell, i.e. exit with an escape
pub fn cast_spell_get_id(prompt: &str, item_id: i32, spell_id: &mut i32, spell_chance: &mut i32) -> i32 {
    // NOTE: `flags` gets set again, since get_and_clear_first_bit modified it
    let mut flags = py().inventory[item_id as usize].flags;
    let first_spell = crate::helpers::get_and_clear_first_bit(&mut flags);

    // Get flags again since get_and_clear_first_bit modified variable.
    let mut flags = py().inventory[item_id as usize].flags & py().flags.spells_learnt;

    let class_id = py().misc.class_id as usize;
    let spells = MAGIC_SPELLS[class_id - 1];

    let mut spell_count = 0;
    let mut spell_list = [0i32; 31];

    while flags != 0 {
        let pos = crate::helpers::get_and_clear_first_bit(&mut flags);

        if spells[pos as usize].level_required as u16 <= py().misc.level {
            spell_list[spell_count] = pos;
            spell_count += 1;
        }
    }

    if spell_count == 0 {
        return -1;
    }

    let mut result = 0;
    if spell_get_id(&spell_list[..spell_count], spell_id, spell_chance, prompt, first_spell) {
        result = 1;
    }

    if result != 0 && MAGIC_SPELLS[class_id - 1][*spell_id as usize].mana_required as i16 > py().misc.current_mana {
        result = if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
            get_input_confirmation(tr!("You summon your limited strength to cast this one! Confirm?")) as i32
        } else {
            get_input_confirmation(tr!("The gods may think you presumptuous for this! Confirm?")) as i32
        };
    }

    result
}

// Following are spell procedure/functions -RAK-
// These routines are commonly used in the scroll, potion, wands, and
// staves routines, and are occasionally called from other areas.
// Now included are creature spells also.           -RAK

// Detect any treasure on the current panel -RAK-
pub fn spell_detect_treasure_within_vicinity() -> bool {
    let mut detected = false;

    let (top, bottom, left, right) = {
        let panel = dg().panel;
        (panel.top, panel.bottom, panel.left, panel.right)
    };

    for y in top..=bottom {
        for x in left..=right {
            let coord = Coord::new(y, x);
            let treasure_id = dg().tile(coord).treasure_id;

            if treasure_id != 0 && game().treasure.list[treasure_id as usize].category_id == TV_GOLD && !cave_tile_visible(coord) {
                dg().tile_mut(coord).field_mark = true;
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

// Detect all objects on the current panel -RAK-
pub fn spell_detect_objects_within_vicinity() -> bool {
    let mut detected = false;

    let (top, bottom, left, right) = {
        let panel = dg().panel;
        (panel.top, panel.bottom, panel.left, panel.right)
    };

    for y in top..=bottom {
        for x in left..=right {
            let coord = Coord::new(y, x);
            let treasure_id = dg().tile(coord).treasure_id;

            if treasure_id != 0 && game().treasure.list[treasure_id as usize].category_id < TV_MAX_OBJECT && !cave_tile_visible(coord) {
                dg().tile_mut(coord).field_mark = true;
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

// Locates and displays traps on current panel -RAK-
pub fn spell_detect_traps_within_vicinity() -> bool {
    let mut detected = false;

    let (top, bottom, left, right) = {
        let panel = dg().panel;
        (panel.top, panel.bottom, panel.left, panel.right)
    };

    for y in top..=bottom {
        for x in left..=right {
            let coord = Coord::new(y, x);
            let treasure_id = dg().tile(coord).treasure_id;

            if treasure_id == 0 {
                continue;
            }

            if game().treasure.list[treasure_id as usize].category_id == TV_INVIS_TRAP {
                dg().tile_mut(coord).field_mark = true;
                trap_change_visibility(coord);
                detected = true;
            } else if game().treasure.list[treasure_id as usize].category_id == TV_CHEST {
                let item = &mut game().treasure.list[treasure_id as usize];
                spell_item_identify_and_remove_random_inscription(item);
            }
        }
    }

    // jdbkmoria extension: trapped paintings register as traps too
    if crate::paintings::detect_painting_traps() {
        detected = true;
    }

    detected
}

// Locates and displays all secret doors on current panel -RAK-
pub fn spell_detect_secret_doors_within_vicinity() -> bool {
    let mut detected = false;

    let (top, bottom, left, right) = {
        let panel = dg().panel;
        (panel.top, panel.bottom, panel.left, panel.right)
    };

    for y in top..=bottom {
        for x in left..=right {
            let coord = Coord::new(y, x);
            let treasure_id = dg().tile(coord).treasure_id;

            if treasure_id == 0 {
                continue;
            }

            let category_id = game().treasure.list[treasure_id as usize].category_id;

            if category_id == TV_SECRET_DOOR {
                // Secret doors

                dg().tile_mut(coord).field_mark = true;
                trap_change_visibility(coord);
                detected = true;
            } else if (category_id == TV_UP_STAIR || category_id == TV_DOWN_STAIR) && !dg().tile(coord).field_mark {
                // Staircases

                dg().tile_mut(coord).field_mark = true;
                dungeon_lite_spot(coord);
                detected = true;
            }
        }
    }

    detected
}

// Locates and displays all invisible creatures on current panel -RAK-
pub fn spell_detect_invisible_creatures_within_vicinity() -> bool {
    let mut detected = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];

        if coord_inside_panel(monster.pos) && (creature.movement & config::monsters::move_flags::CM_INVISIBLE) != 0 {
            monsters()[id as usize].lit = true;

            // works correctly even if hallucinating
            panel_put_tile(creature.sprite as char, monster.pos);

            detected = true;
        }

        id -= 1;
    }

    if detected {
        print_message(Some(tr!("You sense the presence of invisible creatures!")));
        print_message(None);

        // must unlight every monster just lighted
        update_monsters(false);
    }

    detected
}

// Light an area: -RAK-
//     1.  If corridor  light immediate area
//     2.  If room      light entire room plus immediate area.
pub fn spell_light_area(coord: Coord) -> bool {
    if py().flags.blind < 1 {
        print_message(Some(tr!("You are surrounded by a white light.")));
    }

    // NOTE: this is not changed anywhere. A bug or correct? -MRC-
    let lit = true;

    if dg().tile(coord).perma_lit_room && dg().current_level > 0 {
        dungeon_light_room(coord);
    }

    // Must always light immediate area, because one might be standing on
    // the edge of a room, or next to a destroyed area, etc.
    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            let spot = Coord::new(y, x);
            dg().tile_mut(spot).permanent_light = true;
            dungeon_lite_spot(spot);
        }
    }

    lit
}

// Darken an area, opposite of light area -RAK-
pub fn spell_darken_area(coord: Coord) -> bool {
    let mut darkened = false;

    if dg().tile(coord).perma_lit_room && dg().current_level > 0 {
        let half_height = SCREEN_HEIGHT / 2;
        let half_width = SCREEN_WIDTH / 2;
        let start_row = (coord.y / half_height) * half_height + 1;
        let start_col = (coord.x / half_width) * half_width + 1;
        let end_row = start_row + half_height - 1;
        let end_col = start_col + half_width - 1;

        for y in start_row..=end_row {
            for x in start_col..=end_col {
                let spot = Coord::new(y, x);
                let tile = *dg().tile(spot);

                if tile.perma_lit_room && tile.feature_id <= MAX_CAVE_FLOOR {
                    dg().tile_mut(spot).permanent_light = false;
                    dg().tile_mut(spot).feature_id = TILE_DARK_FLOOR;

                    dungeon_lite_spot(spot);

                    if !cave_tile_visible(spot) {
                        darkened = true;
                    }
                }
            }
        }
    } else {
        for y in (coord.y - 1)..=(coord.y + 1) {
            for x in (coord.x - 1)..=(coord.x + 1) {
                let spot = Coord::new(y, x);
                let tile = *dg().tile(spot);

                if tile.feature_id == TILE_CORR_FLOOR && tile.permanent_light {
                    // permanent_light could have been set by star-lite wand, etc
                    dg().tile_mut(spot).permanent_light = false;
                    darkened = true;
                }
            }
        }
    }

    if darkened && py().flags.blind < 1 {
        print_message(Some(tr!("Darkness surrounds you.")));
    }

    darkened
}

pub fn dungeon_light_area_around_floor_tile(coord: Coord) {
    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            let spot = Coord::new(y, x);
            let tile = *dg().tile(spot);

            if tile.feature_id >= MIN_CAVE_WALL {
                dg().tile_mut(spot).permanent_light = true;
            } else if tile.treasure_id != 0 {
                let category_id = game().treasure.list[tile.treasure_id as usize].category_id;
                if category_id >= TV_MIN_VISIBLE && category_id <= TV_MAX_VISIBLE {
                    dg().tile_mut(spot).field_mark = true;
                }
            }
        }
    }
}

// Map the current area plus some -RAK-
pub fn spell_map_current_area() {
    let panel = dg().panel;
    let row_min = panel.top - random_number(10);
    let row_max = panel.bottom + random_number(10);
    let col_min = panel.left - random_number(20);
    let col_max = panel.right + random_number(20);

    for y in row_min..=row_max {
        for x in col_min..=col_max {
            let coord = Coord::new(y, x);
            if coord_in_bounds(coord) && dg().tile(coord).feature_id <= MAX_CAVE_FLOOR {
                dungeon_light_area_around_floor_tile(coord);
            }
        }
    }

    draw_dungeon_panel();
}

// Identify an object -RAK-
pub fn spell_identify_item() -> bool {
    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, tr!("Item you wish identified?"), 0, PLAYER_INVENTORY_SIZE as i32, None, None) {
        return false;
    }

    let mut item_id = item_id as usize;
    item_identify(&mut item_id);

    let item = &mut py().inventory[item_id];
    spell_item_identify_and_remove_random_inscription(item);

    let item = py().inventory[item_id];
    let description = item_description(&item, true);

    let msg = if item_id >= PlayerEquipment::Wield as usize {
        player_recalculate_bonuses();
        format!("{}: {}", player_item_wearing_description(item_id), description)
    } else {
        format!("{} {}", (item_id as u8 + 97) as char, description)
    };
    print_message(Some(&msg));

    true
}

// Get all the monsters on the level pissed off. -RAK-
pub fn spell_aggravate_monsters(affect_distance: i32) -> bool {
    let mut aggravated = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = &mut monsters()[id as usize];
        monster.sleep_count = 0;

        if monster.distance_from_player as i32 <= affect_distance && monster.speed < 2 {
            monster.speed += 1;
            aggravated = true;
        }

        id -= 1;
    }

    if aggravated {
        print_message(Some(tr!("You hear a sudden stirring in the distance!")));
    }

    aggravated
}

// Surround the fool with traps (chuckle) -RAK-
pub fn spell_surround_player_with_traps() -> bool {
    let pos = py().pos;

    for y in (pos.y - 1)..=(pos.y + 1) {
        for x in (pos.x - 1)..=(pos.x + 1) {
            // Don't put a trap under the player, since this can lead to
            // strange situations, e.g. falling through a trap door while
            // trying to rest, setting off a falling rock trap and ending
            // up under the rock.
            if y == pos.y && x == pos.x {
                continue;
            }

            let coord = Coord::new(y, x);
            let tile = *dg().tile(coord);

            if tile.feature_id <= MAX_CAVE_FLOOR {
                if tile.treasure_id != 0 {
                    dungeon_delete_object(coord);
                }

                dungeon_set_trap(coord, random_number(config::dungeon::objects::MAX_TRAPS as i32) - 1);

                // don't let player gain exp from the newly created traps
                let treasure_id = dg().tile(coord).treasure_id;
                game().treasure.list[treasure_id as usize].misc_use = 0;

                // open pits are immediately visible, so call dungeon_lite_spot
                dungeon_lite_spot(coord);
            }
        }
    }

    // traps are always placed, so just return true
    true
}

// Surround the player with doors. -RAK-
pub fn spell_surround_player_with_doors() -> bool {
    let mut created = false;
    let pos = py().pos;

    for y in (pos.y - 1)..=(pos.y + 1) {
        for x in (pos.x - 1)..=(pos.x + 1) {
            // Don't put a door under the player!
            if y == pos.y && x == pos.x {
                continue;
            }

            let coord = Coord::new(y, x);
            let tile = *dg().tile(coord);

            if tile.feature_id <= MAX_CAVE_FLOOR {
                if tile.treasure_id != 0 {
                    dungeon_delete_object(coord);
                }

                let free_id = popt();
                dg().tile_mut(coord).feature_id = TILE_BLOCKED_FLOOR;
                dg().tile_mut(coord).treasure_id = free_id as u8;

                inventory_item_copy_to(config::dungeon::objects::OBJ_CLOSED_DOOR as usize, &mut game().treasure.list[free_id as usize]);
                dungeon_lite_spot(coord);

                created = true;
            }
        }
    }

    created
}

// Destroys any adjacent door(s)/trap(s) -RAK-
pub fn spell_destroy_adjacent_doors_traps() -> bool {
    let mut destroyed = false;
    let pos = py().pos;

    for y in (pos.y - 1)..=(pos.y + 1) {
        for x in (pos.x - 1)..=(pos.x + 1) {
            let coord = Coord::new(y, x);
            let treasure_id = dg().tile(coord).treasure_id;

            if treasure_id == 0 {
                continue;
            }

            let item = game().treasure.list[treasure_id as usize];

            if (item.category_id >= TV_INVIS_TRAP && item.category_id <= TV_CLOSED_DOOR && item.category_id != TV_RUBBLE) || item.category_id == TV_SECRET_DOOR {
                if dungeon_delete_object(coord) {
                    destroyed = true;
                }
            } else if item.category_id == TV_CHEST && item.flags != 0 {
                // destroy traps on chest and unlock
                let item = &mut game().treasure.list[treasure_id as usize];
                item.flags &= !(config::treasure::chests::CH_TRAPPED | config::treasure::chests::CH_LOCKED);
                item.special_name_id = SpecialNameIds::SnUnlocked as u8;

                destroyed = true;

                print_message(Some(tr!("You have disarmed the chest.")));
                let item = &mut game().treasure.list[treasure_id as usize];
                spell_item_identify_and_remove_random_inscription(item);
            }
        }
    }

    destroyed
}

// Display all creatures on the current panel -RAK-
pub fn spell_detect_monsters() -> bool {
    let mut detected = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];

        if coord_inside_panel(monster.pos) && (creature.movement & config::monsters::move_flags::CM_INVISIBLE) == 0 {
            monsters()[id as usize].lit = true;
            detected = true;

            // works correctly even if hallucinating
            panel_put_tile(creature.sprite as char, monster.pos);
        }

        id -= 1;
    }

    if detected {
        print_message(Some(tr!("You sense the presence of monsters!")));
        print_message(None);

        // must unlight every monster just lighted
        update_monsters(false);
    }

    detected
}

// Update monster when light line spell touches it.
fn spell_light_line_touches_monster(monster_id: i32) {
    let monster = monsters()[monster_id as usize];
    let creature_id = monster.creature_id as usize;

    // light up and draw monster
    monster_update_visibility(monster_id);

    let monster = monsters()[monster_id as usize];
    let creature_name = tr!(CREATURES_LIST[creature_id].name);
    let name = monster_name_description(creature_name, monster.lit);

    if (CREATURES_LIST[creature_id].defenses & config::monsters::defense::CD_LIGHT) != 0 {
        if monster.lit {
            creature_recall()[creature_id].defenses |= config::monsters::defense::CD_LIGHT;
        }

        if monster_take_hit(monster_id, dice_roll(Dice::new(2, 8))) >= 0 {
            print_monster_action_text(&name, tr!("shrivels away in the light!"));
            display_character_experience();
        } else {
            print_monster_action_text(&name, tr!("cringes from the light!"));
        }
    }
}

// Leave a line of light in given dir, blue light can sometimes hurt creatures. -RAK-
pub fn spell_light_line(coord: Coord, direction: i32) {
    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            player_move_position(direction, &mut coord);
            finished = true;
            continue; // we're done here, break out of the loop
        }

        if !tile.permanent_light && !tile.temporary_light {
            // set permanent_light so that dungeon_lite_spot will work
            dg().tile_mut(coord).permanent_light = true;

            let tmp_coord = coord;

            if tile.feature_id == TILE_LIGHT_FLOOR {
                if coord_inside_panel(tmp_coord) {
                    dungeon_light_room(tmp_coord);
                }
            } else {
                dungeon_lite_spot(tmp_coord);
            }
        }

        // set permanent_light in case temporary_light was true above
        dg().tile_mut(coord).permanent_light = true;

        if tile.creature_id > 1 {
            spell_light_line_touches_monster(tile.creature_id as i32);
        }

        // move must be at end because want to light up current tmp_coord
        player_move_position(direction, &mut coord);
        distance += 1;
    }
}

// Light line in all directions -RAK-
pub fn spell_starlite(coord: Coord) {
    if py().flags.blind < 1 {
        print_message(Some(tr!("The end of the staff bursts into a blue shimmering light.")));
    }

    for dir in 1..=9 {
        if dir != 5 {
            spell_light_line(coord, dir);
        }
    }
}

// Disarms all traps/chests in a given direction -RAK-
pub fn spell_disarm_all_in_direction(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut disarmed = false;

    loop {
        let tile = *dg().tile(coord);

        // note, must continue up to and including the first non open space,
        // because secret doors have feature_id greater than MAX_OPEN_SPACE
        if tile.treasure_id != 0 {
            let treasure_id = tile.treasure_id as usize;
            let category_id = game().treasure.list[treasure_id].category_id;

            if category_id == TV_INVIS_TRAP || category_id == TV_VIS_TRAP {
                if dungeon_delete_object(coord) {
                    disarmed = true;
                }
            } else if category_id == TV_CLOSED_DOOR {
                // Locked or jammed doors become merely closed.
                game().treasure.list[treasure_id].misc_use = 0;
            } else if category_id == TV_SECRET_DOOR {
                dg().tile_mut(coord).field_mark = true;
                trap_change_visibility(coord);
                disarmed = true;
            } else if category_id == TV_CHEST && game().treasure.list[treasure_id].flags != 0 {
                disarmed = true;
                print_message(Some(tr!("Click!")));

                let item = &mut game().treasure.list[treasure_id];
                item.flags &= !(config::treasure::chests::CH_TRAPPED | config::treasure::chests::CH_LOCKED);
                item.special_name_id = SpecialNameIds::SnUnlocked as u8;

                let item = &mut game().treasure.list[treasure_id];
                spell_item_identify_and_remove_random_inscription(item);
            }
        }

        // move must be at end because want to light up current spot
        player_move_position(direction, &mut coord);

        distance += 1;

        let tile = *dg().tile(coord);
        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id > MAX_OPEN_SPACE {
            break;
        }
    }

    disarmed
}

type DestroyFn = fn(&Inventory) -> bool;

// Return flags for given type area affect -RAK-
fn spell_get_area_affect_flags(spell_type: i32) -> (u32, u16, DestroyFn) {
    if spell_type == crate::spells_data::MagicSpellFlags::MagicMissile as i32 {
        (0, 0, set_null)
    } else if spell_type == crate::spells_data::MagicSpellFlags::Lightning as i32 {
        (config::monsters::spells::CS_BR_LIGHT, config::monsters::defense::CD_LIGHT, set_lightning_destroyable_items)
    } else if spell_type == crate::spells_data::MagicSpellFlags::PoisonGas as i32 {
        (config::monsters::spells::CS_BR_GAS, config::monsters::defense::CD_POISON, set_null)
    } else if spell_type == crate::spells_data::MagicSpellFlags::Acid as i32 {
        (config::monsters::spells::CS_BR_ACID, config::monsters::defense::CD_ACID, set_acid_destroyable_items)
    } else if spell_type == crate::spells_data::MagicSpellFlags::Frost as i32 {
        (config::monsters::spells::CS_BR_FROST, config::monsters::defense::CD_FROST, set_frost_destroyable_items)
    } else if spell_type == crate::spells_data::MagicSpellFlags::Fire as i32 {
        (config::monsters::spells::CS_BR_FIRE, config::monsters::defense::CD_FIRE, set_fire_destroyable_items)
    } else if spell_type == crate::spells_data::MagicSpellFlags::HolyOrb as i32 {
        (0, config::monsters::defense::CD_EVIL, set_null)
    } else {
        print_message(Some("ERROR in spell_get_area_affect_flags()\n"));
        (0, 0, set_null)
    }
}

fn print_bolt_strikes_monster_message(creature_name: &str, bolt_name: &str, is_lit: bool) {
    let monster_name = if is_lit { tr_fmt!("the {}", creature_name) } else { tr!("it").to_string() };
    let msg = tr_fmt!("The {} strikes {}.", bolt_name, monster_name);
    print_message(Some(&msg));
}

// Light up, draw, and check for monster damage when Fire Bolt touches it.
fn spell_fire_bolt_touches_monster(coord: Coord, damage: i32, harm_type: u16, weapon_id: u32, bolt_name: &str) {
    let creature_id_of_tile = dg().tile(coord).creature_id as i32;
    let monster = monsters()[creature_id_of_tile as usize];
    let creature_id = monster.creature_id as usize;

    // light up monster and draw monster, temporarily set
    // permanent_light so that `monster_update_visibility()` will work
    let saved_lit_status = dg().tile(coord).permanent_light;
    dg().tile_mut(coord).permanent_light = true;
    monster_update_visibility(creature_id_of_tile);
    dg().tile_mut(coord).permanent_light = saved_lit_status;

    // draw monster and clear previous bolt
    put_qio();

    let monster = monsters()[creature_id_of_tile as usize];
    let creature = &CREATURES_LIST[creature_id];

    print_bolt_strikes_monster_message(tr!(creature.name), bolt_name, monster.lit);

    let mut damage = damage;
    if (harm_type & creature.defenses) != 0 {
        damage *= 2;
        if monster.lit {
            creature_recall()[creature_id].defenses |= harm_type;
        }
    } else if (weapon_id & creature.spells) != 0 {
        damage /= 4;
        if monster.lit {
            creature_recall()[creature_id].spells |= weapon_id;
        }
    }

    let name = monster_name_description(tr!(creature.name), monster.lit);

    if monster_take_hit(creature_id_of_tile, damage) >= 0 {
        print_monster_action_text(&name, tr!("dies in a fit of agony."));
        display_character_experience();
    } else if damage > 0 {
        print_monster_action_text(&name, tr!("screams in agony."));
    }
}

// Shoot a bolt in a given direction -RAK-
pub fn spell_fire_bolt(coord: Coord, direction: i32, damage_hp: i32, spell_type: i32, spell_name: &str) {
    let (weapon_type, harm_type, _dummy) = spell_get_area_affect_flags(spell_type);

    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let old_coord = coord;
        player_move_position(direction, &mut coord);

        distance += 1;

        let tile = *dg().tile(coord);

        dungeon_lite_spot(old_coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;

            // jdbkmoria extension: a bolt stopped by a painted wall strikes the painting
            if distance <= config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 && crate::paintings::painting_index_at(coord).is_some() {
                crate::paintings::painting_struck_by_magic(coord, damage_hp, spell_name);
            }

            continue; // we're done here, break out of the loop
        }

        if tile.creature_id > 1 {
            finished = true;
            spell_fire_bolt_touches_monster(coord, damage_hp, harm_type, weapon_type, spell_name);
        } else if coord_inside_panel(coord) && py().flags.blind < 1 {
            panel_put_tile('*', coord);

            // show the bolt
            put_qio();
        }
    }
}

// Shoot a ball in a given direction.  Note that balls have an area affect. -RAK-
pub fn spell_fire_ball(coord: Coord, direction: i32, damage_hp: i32, spell_type: i32, spell_name: &str) {
    let mut total_hits = 0;
    let mut total_kills = 0;
    let max_distance = 2;

    let (weapon_type, harm_type, destroy) = spell_get_area_affect_flags(spell_type);

    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        let old_coord = coord;
        player_move_position(direction, &mut coord);

        distance += 1;

        dungeon_lite_spot(old_coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 {
            finished = true;
            continue;
        }

        let tile = *dg().tile(coord);

        if tile.feature_id >= MIN_CLOSED_SPACE || tile.creature_id > 1 {
            finished = true;

            if tile.feature_id >= MIN_CLOSED_SPACE {
                coord = old_coord;
            }

            // The ball hits and explodes.

            // The explosion.
            for row in (coord.y - max_distance)..=(coord.y + max_distance) {
                for col in (coord.x - max_distance)..=(coord.x + max_distance) {
                    let spot = Coord::new(row, col);

                    if coord_in_bounds(spot) && coord_distance_between(coord, spot) <= max_distance && los(coord, spot) {
                        let spot_tile = *dg().tile(spot);

                        if spot_tile.treasure_id != 0 && destroy(&game().treasure.list[spot_tile.treasure_id as usize]) {
                            dungeon_delete_object(spot);
                        }

                        let spot_tile = *dg().tile(spot);
                        if spot_tile.feature_id <= MAX_OPEN_SPACE {
                            if spot_tile.creature_id > 1 {
                                let monster_id = spot_tile.creature_id as i32;
                                let monster = monsters()[monster_id as usize];
                                let creature_id = monster.creature_id as usize;
                                let creature = &CREATURES_LIST[creature_id];

                                // lite up creature if visible, temp set permanent_light so that monster_update_visibility works
                                let saved_lit_status = spot_tile.permanent_light;
                                dg().tile_mut(spot).permanent_light = true;
                                monster_update_visibility(monster_id);

                                total_hits += 1;
                                let mut damage = damage_hp;

                                let monster = monsters()[monster_id as usize];
                                if (harm_type & creature.defenses) != 0 {
                                    damage *= 2;
                                    if monster.lit {
                                        creature_recall()[creature_id].defenses |= harm_type;
                                    }
                                } else if (weapon_type & creature.spells) != 0 {
                                    damage /= 4;
                                    if monster.lit {
                                        creature_recall()[creature_id].spells |= weapon_type;
                                    }
                                }

                                damage /= coord_distance_between(spot, coord) + 1;

                                if monster_take_hit(monster_id, damage) >= 0 {
                                    total_kills += 1;
                                }
                                dg().tile_mut(spot).permanent_light = saved_lit_status;
                            } else if coord_inside_panel(spot) && py().flags.blind < 1 {
                                panel_put_tile('*', spot);
                            }
                        } else if crate::paintings::painting_index_at(spot).is_some() {
                            // jdbkmoria extension: the blast catches a painting on this wall
                            let painting_damage = damage_hp / (coord_distance_between(spot, coord) + 1);
                            crate::paintings::painting_struck_by_magic(spot, painting_damage, spell_name);
                        }
                    }
                }
            }

            // show ball of whatever
            put_qio();

            for row in (coord.y - 2)..=(coord.y + 2) {
                for col in (coord.x - 2)..=(coord.x + 2) {
                    let spot = Coord::new(row, col);

                    if coord_in_bounds(spot) && coord_inside_panel(spot) && coord_distance_between(coord, spot) <= max_distance {
                        dungeon_lite_spot(spot);
                    }
                }
            }
            // End explosion.

            if total_hits == 1 {
                print_message(Some(&tr_fmt!("The {} envelops a creature!", spell_name)));
            } else if total_hits > 1 {
                print_message(Some(&tr_fmt!("The {} envelops several creatures!", spell_name)));
            }

            if total_kills == 1 {
                print_message(Some(tr!("There is a scream of agony!")));
            } else if total_kills > 1 {
                print_message(Some(tr!("There are several screams of agony!")));
            }

            if total_kills >= 0 {
                display_character_experience();
            }
            // End ball hitting.
        } else if coord_inside_panel(coord) && py().flags.blind < 1 {
            panel_put_tile('*', coord);

            // show bolt
            put_qio();
        }
    }
}

// Breath weapon works like a spell_fire_ball(), but affects the player.
// Note the area affect. -RAK-
pub fn spell_breath(coord: Coord, monster_id: i32, damage_hp: i32, spell_type: i32, spell_name: &str) {
    let max_distance = 2;

    let (weapon_type, harm_type, destroy) = spell_get_area_affect_flags(spell_type);

    for y in (coord.y - 2)..=(coord.y + 2) {
        for x in (coord.x - 2)..=(coord.x + 2) {
            let location = Coord::new(y, x);
            if coord_in_bounds(location) && coord_distance_between(coord, location) <= max_distance && los(coord, location) {
                let tile = *dg().tile(location);

                if tile.treasure_id != 0 && destroy(&game().treasure.list[tile.treasure_id as usize]) {
                    dungeon_delete_object(location);
                }

                let tile = *dg().tile(location);
                if tile.feature_id <= MAX_OPEN_SPACE {
                    // must test status bit, not py.flags.blind here, flag could have
                    // been set by a previous monster, but the breath should still
                    // be visible until the blindness takes effect
                    if coord_inside_panel(location) && (py().flags.status & config::player::status::PY_BLIND) == 0 {
                        panel_put_tile('*', location);
                    }

                    if tile.creature_id > 1 {
                        let target_monster_id = tile.creature_id as i32;
                        let monster = monsters()[target_monster_id as usize];
                        let creature_id = monster.creature_id as usize;
                        let creature = &CREATURES_LIST[creature_id];

                        let mut damage = damage_hp;

                        if (harm_type & creature.defenses) != 0 {
                            damage *= 2;
                        } else if (weapon_type & creature.spells) != 0 {
                            damage /= 4;
                        }

                        damage /= coord_distance_between(location, coord) + 1;

                        // can not call monster_take_hit here, since player does not
                        // get experience for kill
                        monsters()[target_monster_id as usize].hp -= damage as i16;
                        monsters()[target_monster_id as usize].sleep_count = 0;

                        let monster = monsters()[target_monster_id as usize];
                        if monster.hp < 0 {
                            let creature_movement = creature.movement;
                            let mut treasure_id = monster_death(monster.pos, creature_movement);

                            if monster.lit {
                                let tmp = (creature_recall()[creature_id].movement & config::monsters::move_flags::CM_TREASURE) >> config::monsters::move_flags::CM_TR_SHIFT;
                                if tmp > ((treasure_id & config::monsters::move_flags::CM_TREASURE) >> config::monsters::move_flags::CM_TR_SHIFT) {
                                    treasure_id = (treasure_id & !config::monsters::move_flags::CM_TREASURE) | (tmp << config::monsters::move_flags::CM_TR_SHIFT);
                                }
                                creature_recall()[creature_id].movement = treasure_id | (creature_recall()[creature_id].movement & !config::monsters::move_flags::CM_TREASURE);
                            }

                            // It ate an already processed monster. Handle normally.
                            if monster_id < target_monster_id {
                                dungeon_delete_monster(target_monster_id);
                            } else {
                                // If it eats this monster, an already processed monster
                                // will take its place, causing all kinds of havoc.
                                // Delay the kill a bit.
                                dungeon_remove_monster_from_level(target_monster_id);
                            }
                        }
                    } else if tile.creature_id == 1 {
                        let mut damage = damage_hp / (coord_distance_between(location, coord) + 1);

                        // let's do at least one point of damage
                        // prevents random_number(0) problem with damage_poisoned_gas, also
                        if damage == 0 {
                            damage = 1;
                        }

                        if spell_type == crate::spells_data::MagicSpellFlags::Lightning as i32 {
                            damage_lightning_bolt(damage, spell_name);
                        } else if spell_type == crate::spells_data::MagicSpellFlags::PoisonGas as i32 {
                            damage_poisoned_gas(damage, spell_name);
                        } else if spell_type == crate::spells_data::MagicSpellFlags::Acid as i32 {
                            damage_acid(damage, spell_name);
                        } else if spell_type == crate::spells_data::MagicSpellFlags::Frost as i32 {
                            damage_cold(damage, spell_name);
                        } else if spell_type == crate::spells_data::MagicSpellFlags::Fire as i32 {
                            damage_fire(damage, spell_name);
                        }
                    }
                }
            }
        }
    }

    // show the ball of gas
    put_qio();

    for y in (coord.y - 2)..=(coord.y + 2) {
        for x in (coord.x - 2)..=(coord.x + 2) {
            let spot = Coord::new(y, x);
            if coord_in_bounds(spot) && coord_inside_panel(spot) && coord_distance_between(coord, spot) <= max_distance {
                dungeon_lite_spot(spot);
            }
        }
    }
}

// Recharge a wand, staff, or rod.  Sometimes the item breaks. -RAK-
pub fn spell_recharge_item(number_of_charges: i32) -> bool {
    let mut item_pos_start = 0;
    let mut item_pos_end = 0;
    if !inventory_find_range(TV_STAFF as i32, TV_WAND as i32, &mut item_pos_start, &mut item_pos_end) {
        print_message(Some(tr!("You have nothing to recharge.")));
        return false;
    }

    let mut item_id: i32 = 0;
    if !inventory_get_input_for_item_id(&mut item_id, tr!("Recharge which item?"), item_pos_start, item_pos_end, None, None) {
        return false;
    }
    let item_id = item_id as usize;

    let item = py().inventory[item_id];

    // recharge  I = recharge(20) = 1/6  failure for empty 10th level wand
    // recharge II = recharge(60) = 1/10 failure for empty 10th level wand
    //
    // make it harder to recharge high level, and highly charged wands,
    // note that `fail_chance` can be negative, so check its value before
    // trying to call random_number().
    let mut fail_chance = number_of_charges + 50 - item.depth_first_found as i32 - item.misc_use as i32;

    // Automatic failure.
    if fail_chance < 19 {
        fail_chance = 1;
    } else {
        fail_chance = random_number(fail_chance / 10);
    }

    if fail_chance == 1 {
        print_message(Some(tr!("There is a bright flash of light.")));
        inventory_destroy_item(item_id);
    } else {
        let mut number_of_charges = (number_of_charges / (item.depth_first_found as i32 + 2)) + 1;
        number_of_charges = 2 + random_number(number_of_charges);
        py().inventory[item_id].misc_use += number_of_charges as i16;

        let item = py().inventory[item_id];
        if spell_item_identified(&item) {
            spell_item_remove_identification(&mut py().inventory[item_id]);
        }

        item_identification_clear_empty(&mut py().inventory[item_id]);
    }

    true
}

// Increase or decrease a creatures hit points -RAK-
pub fn spell_change_monster_hit_points(coord: Coord, direction: i32, damage_hp: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut changed = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            let name = monster_name_description(tr!(creature.name), monster.lit);

            if monster_take_hit(monster_id, damage_hp) >= 0 {
                print_monster_action_text(&name, tr!("dies in a fit of agony."));
                display_character_experience();
            } else if damage_hp > 0 {
                print_monster_action_text(&name, tr!("screams in agony."));
            }

            changed = true;
        }
    }

    changed
}

// Drains life; note it must be living. -RAK-
pub fn spell_drain_life_from_monster(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut drained = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature_id = monster.creature_id as usize;
            let creature = &CREATURES_LIST[creature_id];

            if (creature.defenses & config::monsters::defense::CD_UNDEAD) == 0 {
                let name = monster_name_description(tr!(creature.name), monster.lit);

                if monster_take_hit(monster_id, 75) >= 0 {
                    print_monster_action_text(&name, tr!("dies in a fit of agony."));
                    display_character_experience();
                } else {
                    print_monster_action_text(&name, tr!("screams in agony."));
                }

                drained = true;
            } else {
                creature_recall()[creature_id].defenses |= config::monsters::defense::CD_UNDEAD;
            }
        }
    }

    drained
}

// Increase or decrease a creatures speed -RAK-
// NOTE: cannot slow a winning creature (BALROG)
pub fn spell_speed_monster(coord: Coord, direction: i32, speed: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut changed = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            let name = monster_name_description(tr!(creature.name), monster.lit);

            if speed > 0 {
                monsters()[monster_id as usize].speed += speed as i16;
                monsters()[monster_id as usize].sleep_count = 0;

                changed = true;

                print_monster_action_text(&name, tr!("starts moving faster."));
            } else if random_number(MON_MAX_LEVELS as i32) > creature.level as i32 {
                monsters()[monster_id as usize].speed += speed as i16;
                monsters()[monster_id as usize].sleep_count = 0;

                changed = true;

                print_monster_action_text(&name, tr!("starts moving slower."));
            } else {
                monsters()[monster_id as usize].sleep_count = 0;

                print_monster_action_text(&name, tr!("is unaffected."));
            }
        }
    }

    changed
}

// Confuse a creature -RAK-
pub fn spell_confuse_monster(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut confused = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature_id = monster.creature_id as usize;
            let creature = &CREATURES_LIST[creature_id];

            let name = monster_name_description(tr!(creature.name), monster.lit);

            if random_number(MON_MAX_LEVELS as i32) < creature.level as i32 || (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                if monster.lit && (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                    creature_recall()[creature_id].defenses |= config::monsters::defense::CD_NO_SLEEP;
                }

                // Monsters which resisted the attack should wake up.
                // Monsters with innate resistance ignore the attack.
                if (creature.defenses & config::monsters::defense::CD_NO_SLEEP) == 0 {
                    monsters()[monster_id as usize].sleep_count = 0;
                }

                print_monster_action_text(&name, tr!("is unaffected."));
            } else {
                if monsters()[monster_id as usize].confused_amount != 0 {
                    monsters()[monster_id as usize].confused_amount += 3;
                } else {
                    monsters()[monster_id as usize].confused_amount = (2 + random_number(16)) as u8;
                }
                monsters()[monster_id as usize].sleep_count = 0;

                confused = true;

                print_monster_action_text(&name, tr!("appears confused."));
            }
        }
    }

    confused
}

// Sleep a creature. -RAK-
pub fn spell_sleep_monster(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut asleep = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature_id = monster.creature_id as usize;
            let creature = &CREATURES_LIST[creature_id];

            let name = monster_name_description(tr!(creature.name), monster.lit);

            if random_number(MON_MAX_LEVELS as i32) < creature.level as i32 || (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                if monster.lit && (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                    creature_recall()[creature_id].defenses |= config::monsters::defense::CD_NO_SLEEP;
                }

                print_monster_action_text(&name, tr!("is unaffected."));
            } else {
                monsters()[monster_id as usize].sleep_count = 500;

                asleep = true;

                print_monster_action_text(&name, tr!("falls asleep."));
            }
        }
    }

    asleep
}

// Turn stone to mud, delete wall. -RAK-
pub fn spell_wall_to_mud(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut turned = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        // note, this ray can move through walls as it turns them to mud
        if distance == config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 {
            finished = true;
        }

        if tile.feature_id >= MIN_CAVE_WALL && tile.feature_id != TILE_BOUNDARY_WALL {
            finished = true;

            player_tunnel_wall(coord, 1, 0);

            if cave_tile_visible(coord) {
                turned = true;
                print_message(Some(tr!("The wall turns into mud.")));
            }
        } else if tile.treasure_id != 0 && tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;

            if coord_inside_panel(coord) && cave_tile_visible(coord) {
                turned = true;

                let item = game().treasure.list[tile.treasure_id as usize];
                let description = item_description(&item, false);

                let out_val = tr_fmt!("The {} turns into mud.", description);
                print_message(Some(&out_val));
            }

            if game().treasure.list[tile.treasure_id as usize].category_id == TV_RUBBLE {
                dungeon_delete_object(coord);
                if random_number(10) == 1 {
                    dungeon_place_random_object_at(coord, false);
                    if cave_tile_visible(coord) {
                        print_message(Some(tr!("You have found something!")));
                    }
                }
                dungeon_lite_spot(coord);
            } else {
                dungeon_delete_object(coord);
            }
        }

        let tile = *dg().tile(coord);
        if tile.creature_id > 1 {
            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            if (creature.defenses & config::monsters::defense::CD_STONE) != 0 {
                let name = monster_name_description(tr!(creature.name), monster.lit);

                // Should get these messages even if the monster is not visible.
                let creature_id = monster_take_hit(monster_id, 100);
                if creature_id >= 0 {
                    creature_recall()[creature_id as usize].defenses |= config::monsters::defense::CD_STONE;
                    print_monster_action_text(&name, tr!("dissolves!"));
                    display_character_experience(); // print msg before calling prt_exp
                } else {
                    creature_recall()[monster.creature_id as usize].defenses |= config::monsters::defense::CD_STONE;
                    print_monster_action_text(&name, tr!("grunts in pain!"));
                }
                finished = true;
            }
        }
    }

    turned
}

// Destroy all traps and doors in a given direction -RAK-
pub fn spell_destroy_doors_traps_in_direction(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut destroyed = false;
    let mut distance = 0;

    loop {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        // must move into first closed spot, as it might be a secret door
        if tile.treasure_id != 0 {
            let treasure_id = tile.treasure_id as usize;
            let category_id = game().treasure.list[treasure_id].category_id;

            if category_id == TV_INVIS_TRAP || category_id == TV_CLOSED_DOOR || category_id == TV_VIS_TRAP || category_id == TV_OPEN_DOOR || category_id == TV_SECRET_DOOR {
                if dungeon_delete_object(coord) {
                    destroyed = true;
                    print_message(Some(tr!("There is a bright flash of light!")));
                }
            } else if category_id == TV_CHEST && game().treasure.list[treasure_id].flags != 0 {
                destroyed = true;
                print_message(Some(tr!("Click!")));

                let item = &mut game().treasure.list[treasure_id];
                item.flags &= !(config::treasure::chests::CH_TRAPPED | config::treasure::chests::CH_LOCKED);
                item.special_name_id = SpecialNameIds::SnUnlocked as u8;

                let item = &mut game().treasure.list[treasure_id];
                spell_item_identify_and_remove_random_inscription(item);
            }
        }

        let tile = *dg().tile(coord);
        if !(distance <= config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id <= MAX_OPEN_SPACE) {
            break;
        }
    }

    destroyed
}

// Polymorph a monster -RAK-
// NOTE: cannot polymorph a winning creature (BALROG)
pub fn spell_polymorph_monster(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut morphed = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            if random_number(MON_MAX_LEVELS as i32) > creature.level as i32 {
                finished = true;

                dungeon_delete_monster(monster_id);

                // Place_monster() should always return true here.
                let level_range = monster_levels()[MON_MAX_LEVELS] - monster_levels()[0];
                morphed = monster_place_new(coord, random_number(level_range as i32) - 1 + monster_levels()[0] as i32, false);

                // don't test tile.field_mark here, only permanent_light/temporary_light
                let tile = *dg().tile(coord);
                if morphed && coord_inside_panel(coord) && (tile.temporary_light || tile.permanent_light) {
                    morphed = true;
                }
            } else {
                let name = monster_name_description(tr!(creature.name), monster.lit);
                print_monster_action_text(&name, tr!("is unaffected."));
            }
        }
    }

    morphed
}

// Create a wall. -RAK-
pub fn spell_build_wall(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut built = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue; // we're done here, break out of the loop
        }

        if tile.treasure_id != 0 {
            dungeon_delete_object(coord);
        }

        let tile = *dg().tile(coord);
        if tile.creature_id > 1 {
            finished = true;

            let monster_id = tile.creature_id as i32;
            let monster = monsters()[monster_id as usize];
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            if (creature.movement & config::monsters::move_flags::CM_PHASE) == 0 {
                // monster does not move, can't escape the wall
                let damage = if (creature.movement & config::monsters::move_flags::CM_ATTACK_ONLY) != 0 {
                    // this will kill everything
                    3000
                } else {
                    dice_roll(Dice::new(4, 8))
                };

                let name = monster_name_description(tr!(creature.name), monster.lit);

                print_monster_action_text(&name, tr!("wails out in pain!"));

                if monster_take_hit(monster_id, damage) >= 0 {
                    print_monster_action_text(&name, tr!("is embedded in the rock."));
                    display_character_experience();
                }
            } else if creature.sprite == b'E' || creature.sprite == b'X' {
                // must be an earth elemental, an earth spirit,
                // or a Xorn to increase its hit points
                monsters()[monster_id as usize].hp += dice_roll(Dice::new(4, 8)) as i16;
            }
        }

        let tile_ref = dg().tile_mut(coord);
        tile_ref.feature_id = TILE_MAGMA_WALL;
        tile_ref.field_mark = false;

        // Permanently light this wall if it is lit by player's lamp.
        tile_ref.permanent_light = tile_ref.temporary_light || tile_ref.permanent_light;
        dungeon_lite_spot(coord);

        built = true;
    }

    built
}

// Replicate a creature -RAK-
pub fn spell_clone_monster(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
        } else if tile.creature_id > 1 {
            monsters()[tile.creature_id as usize].sleep_count = 0;

            // monptr of 0 is safe here, since can't reach here from creatures
            return monster_multiply(coord, monsters()[tile.creature_id as usize].creature_id as i32, 0);
        }
    }

    false
}

// Move the creature record to a new location -RAK-
pub fn spell_teleport_away_monster(monster_id: i32, distance_from_player: i32) {
    let mut distance_from_player = distance_from_player;
    let mut counter = 0;

    let monster = monsters()[monster_id as usize];
    let mut coord = Coord::new(0, 0);

    loop {
        loop {
            coord.y = monster.pos.y + (random_number(2 * distance_from_player + 1) - (distance_from_player + 1));
            coord.x = monster.pos.x + (random_number(2 * distance_from_player + 1) - (distance_from_player + 1));

            if coord_in_bounds(coord) {
                break;
            }
        }

        counter += 1;
        if counter > 9 {
            counter = 0;
            distance_from_player += 5;
        }

        let tile = *dg().tile(coord);
        if tile.feature_id < MIN_CLOSED_SPACE && tile.creature_id == 0 {
            break;
        }
    }

    dungeon_move_creature_record(monster.pos, coord);
    dungeon_lite_spot(monster.pos);

    monsters()[monster_id as usize].pos = coord;

    // this is necessary, because the creature is
    // not currently visible in its new position.
    monsters()[monster_id as usize].lit = false;
    monsters()[monster_id as usize].distance_from_player = coord_distance_between(py().pos, coord) as u8;

    monster_update_visibility(monster_id);
}

// Teleport player to spell casting creature -RAK-
pub fn spell_teleport_player_to(coord: Coord) {
    let mut distance = 1;
    let mut counter = 0;

    let mut rnd_coord = Coord::new(0, 0);

    loop {
        rnd_coord.y = coord.y + (random_number(2 * distance + 1) - (distance + 1));
        rnd_coord.x = coord.x + (random_number(2 * distance + 1) - (distance + 1));
        counter += 1;
        if counter > 9 {
            counter = 0;
            distance += 1;
        }

        if coord_in_bounds(rnd_coord) {
            let tile = *dg().tile(rnd_coord);
            if tile.feature_id < MIN_CLOSED_SPACE && tile.creature_id < 2 {
                break;
            }
        }
    }

    let pos = py().pos;
    dungeon_move_creature_record(pos, rnd_coord);

    for y in (pos.y - 1)..=(pos.y + 1) {
        for x in (pos.x - 1)..=(pos.x + 1) {
            let spot = Coord::new(y, x);
            dg().tile_mut(spot).temporary_light = false;
            dungeon_lite_spot(spot);
        }
    }

    dungeon_lite_spot(pos);

    py().pos = rnd_coord;

    dungeon_reset_view();

    // light creatures
    update_monsters(false);
}

// Teleport all creatures in a given direction away -RAK-
pub fn spell_teleport_away_monster_in_direction(coord: Coord, direction: i32) -> bool {
    let mut coord = coord;
    let mut distance = 0;
    let mut teleported = false;
    let mut finished = false;

    while !finished {
        player_move_position(direction, &mut coord);
        distance += 1;

        let tile = *dg().tile(coord);

        if distance > config::treasure::OBJECT_BOLTS_MAX_RANGE as i32 || tile.feature_id >= MIN_CLOSED_SPACE {
            finished = true;
            continue;
        }

        if tile.creature_id > 1 {
            // wake it up
            monsters()[tile.creature_id as usize].sleep_count = 0;

            spell_teleport_away_monster(tile.creature_id as i32, config::monsters::MON_MAX_SIGHT as i32);

            teleported = true;
        }
    }

    teleported
}

// Delete all creatures within max_sight distance -RAK-
// NOTE : Winning creatures cannot be killed by genocide.
pub fn spell_mass_genocide() -> bool {
    let mut killed = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];

        if monster.distance_from_player <= config::monsters::MON_MAX_SIGHT && (creature.movement & config::monsters::move_flags::CM_WIN) == 0 {
            killed = true;
            dungeon_delete_monster(id);
        }

        id -= 1;
    }

    killed
}

// Delete all creatures of a given type from level. -RAK-
// This does not keep creatures of type from appearing later.
// NOTE : Winning creatures can not be killed by genocide.
pub fn spell_genocide() -> bool {
    let mut creature_char = '\0';
    if !get_tile_character(tr!("Which type of creature do you wish exterminated?"), &mut creature_char) {
        return false;
    }

    let mut killed = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];

        if creature_char as u8 == creature.sprite {
            if (creature.movement & config::monsters::move_flags::CM_WIN) == 0 {
                killed = true;
                dungeon_delete_monster(id);
            } else {
                // genocide is a powerful spell, so we will let the player
                // know the names of the creatures they did not destroy,
                // this message makes no sense otherwise
                print_message(Some(&tr_fmt!("The {} is unaffected.", tr!(creature.name))));
            }
        }

        id -= 1;
    }

    killed
}

// Change speed of any creature . -RAK-
// NOTE: cannot slow a winning creature (BALROG)
pub fn spell_speed_all_monsters(speed: i32) -> bool {
    let mut speedy = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature = &CREATURES_LIST[monster.creature_id as usize];

        let name = monster_name_description(tr!(creature.name), monster.lit);

        if monster.distance_from_player > config::monsters::MON_MAX_SIGHT || !los(py().pos, monster.pos) {
            id -= 1;
            continue; // do nothing
        }

        if speed > 0 {
            monsters()[id as usize].speed += speed as i16;
            monsters()[id as usize].sleep_count = 0;

            if monster.lit {
                speedy = true;
                print_monster_action_text(&name, tr!("starts moving faster."));
            }
        } else if random_number(MON_MAX_LEVELS as i32) > creature.level as i32 {
            monsters()[id as usize].speed += speed as i16;
            monsters()[id as usize].sleep_count = 0;

            if monster.lit {
                speedy = true;
                print_monster_action_text(&name, tr!("starts moving slower."));
            }
        } else if monster.lit {
            monsters()[id as usize].sleep_count = 0;
            print_monster_action_text(&name, tr!("is unaffected."));
        }

        id -= 1;
    }

    speedy
}

// Sleep any creature . -RAK-
pub fn spell_sleep_all_monsters() -> bool {
    let mut asleep = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature_id = monster.creature_id as usize;
        let creature = &CREATURES_LIST[creature_id];

        let name = monster_name_description(tr!(creature.name), monster.lit);

        if monster.distance_from_player > config::monsters::MON_MAX_SIGHT || !los(py().pos, monster.pos) {
            id -= 1;
            continue; // do nothing
        }

        if random_number(MON_MAX_LEVELS as i32) < creature.level as i32 || (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
            if monster.lit {
                if (creature.defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                    creature_recall()[creature_id].defenses |= config::monsters::defense::CD_NO_SLEEP;
                }
                print_monster_action_text(&name, tr!("is unaffected."));
            }
        } else {
            monsters()[id as usize].sleep_count = 500;
            if monster.lit {
                asleep = true;
                print_monster_action_text(&name, tr!("falls asleep."));
            }
        }

        id -= 1;
    }

    asleep
}

// Polymorph any creature that player can see. -RAK-
// NOTE: cannot polymorph a winning creature (BALROG)
pub fn spell_mass_polymorph() -> bool {
    let mut morphed = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];

        if monster.distance_from_player <= config::monsters::MON_MAX_SIGHT {
            let creature = &CREATURES_LIST[monster.creature_id as usize];

            if (creature.movement & config::monsters::move_flags::CM_WIN) == 0 {
                let coord = monster.pos;
                dungeon_delete_monster(id);

                // Place_monster() should always return true here.
                let level_range = monster_levels()[MON_MAX_LEVELS] - monster_levels()[0];
                morphed = monster_place_new(coord, random_number(level_range as i32) - 1 + monster_levels()[0] as i32, false);
            }
        }

        id -= 1;
    }

    morphed
}

// Display evil creatures on current panel -RAK-
pub fn spell_detect_evil() -> bool {
    let mut detected = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];

        if coord_inside_panel(monster.pos) && (CREATURES_LIST[monster.creature_id as usize].defenses & config::monsters::defense::CD_EVIL) != 0 {
            monsters()[id as usize].lit = true;

            detected = true;

            // works correctly even if hallucinating
            panel_put_tile(CREATURES_LIST[monster.creature_id as usize].sprite as char, monster.pos);
        }

        id -= 1;
    }

    if detected {
        print_message(Some(tr!("You sense the presence of evil!")));
        print_message(None);

        // must unlight every monster just lighted
        update_monsters(false);
    }

    detected
}

// Change players hit points in some manner -RAK-
pub fn spell_change_player_hit_points(adjustment: i32) -> bool {
    if py().misc.current_hp >= py().misc.max_hp {
        return false;
    }

    py().misc.current_hp += adjustment as i16;
    if py().misc.current_hp > py().misc.max_hp {
        py().misc.current_hp = py().misc.max_hp;
        py().misc.current_hp_fraction = 0;
    }
    print_character_current_hit_points();

    let adjustment = adjustment / 5;

    if adjustment < 3 {
        if adjustment == 0 {
            print_message(Some(tr!("You feel a little better.")));
        } else {
            print_message(Some(tr!("You feel better.")));
        }
    } else if adjustment < 7 {
        print_message(Some(tr!("You feel much better.")));
    } else {
        print_message(Some(tr!("You feel very good.")));
    }

    true
}

fn earthquake_hits_monster(monster_id: i32) {
    let monster = monsters()[monster_id as usize];
    let creature = &CREATURES_LIST[monster.creature_id as usize];

    if (creature.movement & config::monsters::move_flags::CM_PHASE) == 0 {
        let damage = if (creature.movement & config::monsters::move_flags::CM_ATTACK_ONLY) != 0 {
            // this will kill everything
            3000
        } else {
            dice_roll(Dice::new(4, 8))
        };

        let name = monster_name_description(tr!(creature.name), monster.lit);

        print_monster_action_text(&name, tr!("wails out in pain!"));

        if monster_take_hit(monster_id, damage) >= 0 {
            print_monster_action_text(&name, tr!("is embedded in the rock."));
            display_character_experience();
        }
    } else if creature.sprite == b'E' || creature.sprite == b'X' {
        // must be an earth elemental or an earth spirit, or a
        // Xorn increase its hit points
        monsters()[monster_id as usize].hp += dice_roll(Dice::new(4, 8)) as i16;
    }
}

// This is a fun one.  In a given block, pick some walls and
// turn them into open spots.  Pick some open spots and dg.game_turn
// them into walls.  An "Earthquake" effect. -RAK-
pub fn spell_earthquake() {
    let pos = py().pos;

    for y in (pos.y - 8)..=(pos.y + 8) {
        for x in (pos.x - 8)..=(pos.x + 8) {
            let coord = Coord::new(y, x);
            if (y != pos.y || x != pos.x) && coord_in_bounds(coord) && random_number(8) == 1 {
                let tile = *dg().tile(coord);

                if tile.treasure_id != 0 {
                    dungeon_delete_object(coord);
                }

                if tile.creature_id > 1 {
                    earthquake_hits_monster(tile.creature_id as i32);
                }

                let tile = dg().tile_mut(coord);
                if tile.feature_id >= MIN_CAVE_WALL && tile.feature_id != TILE_BOUNDARY_WALL {
                    tile.feature_id = TILE_CORR_FLOOR;
                    tile.permanent_light = false;
                    tile.field_mark = false;
                } else if tile.feature_id <= MAX_CAVE_FLOOR {
                    let tmp = random_number(10);

                    if tmp < 6 {
                        tile.feature_id = TILE_QUARTZ_WALL;
                    } else if tmp < 9 {
                        tile.feature_id = TILE_MAGMA_WALL;
                    } else {
                        tile.feature_id = TILE_GRANITE_WALL;
                    }

                    tile.field_mark = false;
                }
                dungeon_lite_spot(coord);
            }
        }
    }
}

// Create some high quality mush for the player. -RAK-
pub fn spell_create_food() {
    let pos = py().pos;

    // take no action here, don't want to destroy object under player
    if dg().tile(pos).treasure_id != 0 {
        // set player_free_turn so that scroll/spell points won't be used
        game().player_free_turn = true;

        print_message(Some(tr!("There is already an object under you.")));

        return;
    }

    dungeon_place_random_object_at(pos, false);
    let treasure_id = dg().tile(pos).treasure_id;
    inventory_item_copy_to(config::dungeon::objects::OBJ_MUSH as usize, &mut game().treasure.list[treasure_id as usize]);
}

// Attempts to destroy a type of creature.  Success depends on
// the creatures level VS. the player's level -RAK-
pub fn spell_dispel_creature(creature_defense: i32, damage: i32) -> bool {
    let mut dispelled = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature_id = monster.creature_id as usize;

        if monster.distance_from_player <= config::monsters::MON_MAX_SIGHT
            && (creature_defense as u16 & CREATURES_LIST[creature_id].defenses) != 0
            && los(py().pos, monster.pos)
        {
            let creature = &CREATURES_LIST[creature_id];

            creature_recall()[creature_id].defenses |= creature_defense as u16;

            dispelled = true;

            let name = monster_name_description(tr!(creature.name), monster.lit);

            let hit = monster_take_hit(id, random_number(damage));

            // Should get these messages even if the monster is not visible.
            if hit >= 0 {
                print_monster_action_text(&name, tr!("dissolves!"));
            } else {
                print_monster_action_text(&name, tr!("shudders."));
            }

            if hit >= 0 {
                display_character_experience();
            }
        }

        id -= 1;
    }

    dispelled
}

// Attempt to turn (confuse) undead creatures. -RAK-
pub fn spell_turn_undead() -> bool {
    let mut turned = false;

    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 {
        let monster = monsters()[id as usize];
        let creature_id = monster.creature_id as usize;
        let creature = &CREATURES_LIST[creature_id];

        if monster.distance_from_player <= config::monsters::MON_MAX_SIGHT
            && (creature.defenses & config::monsters::defense::CD_UNDEAD) != 0
            && los(py().pos, monster.pos)
        {
            let name = monster_name_description(tr!(creature.name), monster.lit);

            if py().misc.level as i32 + 1 > creature.level as i32 || random_number(5) == 1 {
                if monster.lit {
                    creature_recall()[creature_id].defenses |= config::monsters::defense::CD_UNDEAD;

                    turned = true;

                    print_monster_action_text(&name, tr!("runs frantically!"));
                }

                monsters()[id as usize].confused_amount = py().misc.level as u8;
            } else if monster.lit {
                print_monster_action_text(&name, tr!("is unaffected."));
            }
        }

        id -= 1;
    }

    turned
}

// Leave a glyph of warding. Creatures will not pass over! -RAK-
pub fn spell_warding_glyph() {
    let pos = py().pos;
    if dg().tile(pos).treasure_id == 0 {
        let free_id = popt();
        dg().tile_mut(pos).treasure_id = free_id as u8;
        inventory_item_copy_to(config::dungeon::objects::OBJ_SCARE_MON as usize, &mut game().treasure.list[free_id as usize]);
    }
}

// Lose a strength point. -RAK-
pub fn spell_lose_str() {
    if !py().flags.sustain_str {
        player_stat_random_decrease(A_STR);
        print_message(Some(tr!("You feel very sick.")));
    } else {
        print_message(Some(tr!("You feel sick for a moment,  it passes.")));
    }
}

// Lose an intelligence point. -RAK-
pub fn spell_lose_int() {
    if !py().flags.sustain_int {
        player_stat_random_decrease(A_INT);
        print_message(Some(tr!("You become very dizzy.")));
    } else {
        print_message(Some(tr!("You become dizzy for a moment,  it passes.")));
    }
}

// Lose a wisdom point. -RAK-
pub fn spell_lose_wis() {
    if !py().flags.sustain_wis {
        player_stat_random_decrease(A_WIS);
        print_message(Some(tr!("You feel very naive.")));
    } else {
        print_message(Some(tr!("You feel naive for a moment,  it passes.")));
    }
}

// Lose a dexterity point. -RAK-
// (never called in the original C++ either; kept for fidelity)
#[allow(dead_code)]
pub fn spell_lose_dex() {
    if !py().flags.sustain_dex {
        player_stat_random_decrease(A_DEX);
        print_message(Some(tr!("You feel very sore.")));
    } else {
        print_message(Some(tr!("You feel sore for a moment,  it passes.")));
    }
}

// Lose a constitution point. -RAK-
pub fn spell_lose_con() {
    if !py().flags.sustain_con {
        player_stat_random_decrease(A_CON);
        print_message(Some(tr!("You feel very sick.")));
    } else {
        print_message(Some(tr!("You feel sick for a moment,  it passes.")));
    }
}

// Lose a charisma point. -RAK-
pub fn spell_lose_chr() {
    if !py().flags.sustain_chr {
        player_stat_random_decrease(A_CHR);
        print_message(Some(tr!("Your skin starts to itch.")));
    } else {
        print_message(Some(tr!("Your skin starts to itch, but feels better now.")));
    }
}

// Lose experience -RAK-
pub fn spell_lose_exp(adjustment: i32) {
    if adjustment > py().misc.exp {
        py().misc.exp = 0;
    } else {
        py().misc.exp -= adjustment;
    }
    display_character_experience();

    let mut exp: i32 = 0;
    while (py().base_exp_levels[exp as usize] as i64 * py().misc.experience_factor as i64 / 100) as i32 <= py().misc.exp {
        exp += 1;
    }

    // increment exp once more, because level 1 exp is stored in player_base_exp_levels[0]
    exp += 1;

    if py().misc.level != exp as u16 {
        py().misc.level = exp as u16;

        player_calculate_hit_points();

        let class_to_use_mage_spells = CLASSES[py().misc.class_id as usize].class_to_use_mage_spells;

        if class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
            player_calculate_allowed_spells_count(A_INT);
            player_gain_mana(A_INT);
        } else if class_to_use_mage_spells == config::spells::SPELL_TYPE_PRIEST {
            player_calculate_allowed_spells_count(A_WIS);
            player_gain_mana(A_WIS);
        }
        crate::ui::print_character_level();
        crate::ui::print_character_title();
    }
}

// Slow Poison -RAK-
pub fn spell_slow_poison() -> bool {
    if py().flags.poisoned > 0 {
        py().flags.poisoned /= 2;
        if py().flags.poisoned < 1 {
            py().flags.poisoned = 1;
        }
        print_message(Some(tr!("The effect of the poison has been reduced.")));
        return true;
    }

    false
}

fn replace_spot(coord: Coord, typ: i32) {
    {
        let tile = dg().tile_mut(coord);

        match typ {
            1..=3 => tile.feature_id = TILE_CORR_FLOOR,
            4 | 7 | 10 => tile.feature_id = TILE_GRANITE_WALL,
            5 | 8 | 11 => tile.feature_id = TILE_MAGMA_WALL,
            6 | 9 | 12 => tile.feature_id = TILE_QUARTZ_WALL,
            _ => {}
        }

        tile.permanent_light = false;
        tile.field_mark = false;
        tile.perma_lit_room = false; // this is no longer part of a room
    }

    let tile = *dg().tile(coord);

    if tile.treasure_id != 0 {
        dungeon_delete_object(coord);
    }

    if tile.creature_id > 1 {
        dungeon_delete_monster(tile.creature_id as i32);
    }
}

// The spell of destruction. -RAK-
// NOTE:
//   Winning creatures that are deleted will be considered as teleporting to another level.
//   This will NOT win the game.
pub fn spell_destroy_area(coord: Coord) {
    if dg().current_level > 0 {
        for y in (coord.y - 15)..=(coord.y + 15) {
            for x in (coord.x - 15)..=(coord.x + 15) {
                let spot = Coord::new(y, x);
                if coord_in_bounds(spot) && dg().tile(spot).feature_id != TILE_BOUNDARY_WALL {
                    let distance = coord_distance_between(spot, coord);

                    // clear player's spot, but don't put wall there
                    if distance == 0 {
                        replace_spot(spot, 1);
                    } else if distance < 13 {
                        replace_spot(spot, random_number(6));
                    } else if distance < 16 {
                        replace_spot(spot, random_number(9));
                    }
                }
            }
        }
    }

    print_message(Some(tr!("There is a searing blast of light!")));
    py().flags.blind += (10 + random_number(10)) as i16;
}

// Enchants a plus onto an item. -RAK-
// `limit` param is the maximum bonus allowed; usually 10,
// but weapon's maximum damage when enchanting melee weapons to damage.
pub fn spell_enchant_item(plusses: &mut i16, max_bonus_limit: i16) -> bool {
    // avoid random_number(0) call
    if max_bonus_limit <= 0 {
        return false;
    }

    let mut chance = 0;

    if *plusses > 0 {
        chance = *plusses as i32;

        // very rarely allow enchantment over limit
        if random_number(100) == 1 {
            chance = random_number(chance) - 1;
        }
    }

    if random_number(max_bonus_limit as i32) > chance {
        *plusses += 1;
        return true;
    }

    false
}

// Removes curses from items in inventory -RAK-
pub fn spell_remove_curse_from_all_worn_items() -> bool {
    const WORN_ITEMS: [PlayerEquipment; 10] = [
        PlayerEquipment::Wield,
        PlayerEquipment::Head,
        PlayerEquipment::Neck,
        PlayerEquipment::Body,
        PlayerEquipment::Arm,
        PlayerEquipment::Hands,
        PlayerEquipment::Right,
        PlayerEquipment::Left,
        PlayerEquipment::Feet,
        PlayerEquipment::Outer,
    ];

    let mut removed = false;

    for equipment_id in WORN_ITEMS {
        if player_worn_item_is_cursed(equipment_id) {
            player_worn_item_remove_curse(equipment_id);
            player_recalculate_bonuses();
            removed = true;
        }
    }

    removed
}

// Restores any drained experience -RAK-
pub fn spell_restore_player_levels() -> bool {
    if py().misc.max_exp > py().misc.exp {
        print_message(Some(tr!("You feel your life energies returning.")));

        // this while loop is not redundant, ptr_exp may reduce the exp level
        while py().misc.exp < py().misc.max_exp {
            py().misc.exp = py().misc.max_exp;
            display_character_experience();
        }

        return true;
    }

    false
}
