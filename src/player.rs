// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::globals::RacyCell;
use crate::inventory::{Inventory, PLAYER_INVENTORY_SIZE};
use crate::types::Coord;

// Player class level adjustment indexes
pub const CLASS_BTH: usize = 0;
pub const CLASS_BTHB: usize = 1;
pub const CLASS_DEVICE: usize = 2;
pub const CLASS_DISARM: usize = 3;
pub const CLASS_SAVE: usize = 4;

// Attribute indexes -CJS-
pub const A_STR: usize = 0;
pub const A_INT: usize = 1;
pub const A_WIS: usize = 2;
pub const A_DEX: usize = 3;
pub const A_CON: usize = 4;
pub const A_CHR: usize = 5;

// this depends on the fact that class_level_adj[CLASS_SAVE] values are all the same,
// if not, then should add a separate column for this
pub const CLASS_MISC_HIT: usize = 4;
pub const CLASS_MAX_LEVEL_ADJUST: usize = 5;

// Player constants
pub const PLAYER_MAX_LEVEL: usize = 40; // Maximum possible character level
pub const PLAYER_MAX_CLASSES: usize = 6; // Number of defined classes
pub const PLAYER_MAX_RACES: usize = 8; // Number of defined races
pub const PLAYER_MAX_BACKGROUNDS: usize = 128; // Number of types of histories for univ

// Base to hit constants
pub const BTH_PER_PLUS_TO_HIT_ADJUST: i32 = 3; // Adjust BTH per plus-to-hit

pub const PLAYER_NAME_SIZE: usize = 27;

// Player misc data
#[derive(Default)]
pub struct PlayerMisc {
    pub name: String,                 // Name of character
    pub gender: bool,                 // Gender of character (Female = 0, Male = 1)
    pub date_of_birth: i32,           // Unix time for when the character was created
    pub au: i32,                      // Gold
    pub max_exp: i32,                 // Max experience
    pub exp: i32,                     // Cur experience
    pub exp_fraction: u16,            // Cur exp fraction * 2^16
    pub age: u16,                     // Characters age
    pub height: u16,                  // Height
    pub weight: u16,                  // Weight
    pub level: u16,                   // Level
    pub max_dungeon_depth: u16,       // Max level explored
    pub chance_in_search: i16,        // Chance in search
    pub fos: i16,                     // Frequency of search
    pub bth: i16,                     // Base to hit
    pub bth_with_bows: i16,           // BTH with bows
    pub mana: i16,                    // Mana points
    pub max_hp: i16,                  // Max hit pts
    pub plusses_to_hit: i16,          // Plusses to hit
    pub plusses_to_damage: i16,       // Plusses to dam
    pub ac: i16,                      // Total AC
    pub magical_ac: i16,              // Magical AC
    pub display_to_hit: i16,          // Display +ToHit
    pub display_to_damage: i16,       // Display +ToDam
    pub display_ac: i16,              // Display +ToTAC
    pub display_to_ac: i16,           // Display +ToAC
    pub disarm: i16,                  // % to Disarm
    pub saving_throw: i16,            // Saving throw
    pub social_class: i16,            // Social Class
    pub stealth_factor: i16,          // Stealth factor
    pub class_id: u8,                 // # of class
    pub race_id: u8,                  // # of race
    pub hit_die: u8,                  // Char hit die
    pub experience_factor: u8,        // Experience factor
    pub current_mana: i16,            // Current mana points
    pub current_mana_fraction: u16,   // Current mana fraction * 2^16
    pub current_hp: i16,              // Current hit points
    pub current_hp_fraction: u16,     // Current hit points fraction * 2^16
    pub history: [String; 4],         // History record
}

// Stats now kept in arrays, for more efficient access. -CJS-
#[derive(Default)]
pub struct PlayerStats {
    pub max: [u8; 6],      // What is restored
    pub current: [u8; 6],  // What is natural
    pub modified: [i16; 6], // What is modified, may be +/-
    pub used: [u8; 6],     // What is used
}

#[derive(Default)]
pub struct PlayerFlags {
    pub status: u32,           // Status of player
    pub rest: i16,             // Rest counter
    pub blind: i16,            // Blindness counter
    pub paralysis: i16,        // Paralysis counter
    pub confused: i16,         // Confusion counter
    pub food: i16,             // Food counter
    pub food_digested: i16,    // Food per round
    pub protection: i16,       // Protection fr. evil
    pub speed: i16,            // Cur speed adjust
    pub fast: i16,             // Temp speed change
    pub slow: i16,             // Temp speed change
    pub afraid: i16,           // Fear
    pub poisoned: i16,         // Poisoned
    pub image: i16,            // Hallucinate
    pub protect_evil: i16,     // Protect VS evil
    pub invulnerability: i16,  // Increases AC
    pub heroism: i16,          // Heroism
    pub super_heroism: i16,    // Super Heroism
    pub blessed: i16,          // Blessed
    pub heat_resistance: i16,  // Timed heat resist
    pub cold_resistance: i16,  // Timed cold resist
    pub detect_invisible: i16, // Timed see invisible
    pub word_of_recall: i16,   // Timed teleport level
    pub see_infra: i16,        // See warm creatures
    pub timed_infra: i16,      // Timed infra vision
    pub see_invisible: bool,   // Can see invisible
    pub teleport: bool,        // Random teleportation
    pub free_action: bool,     // Never paralyzed
    pub slow_digest: bool,     // Lower food needs
    pub aggravate: bool,       // Aggravate monsters
    pub resistant_to_fire: bool, // Resistance to fire
    pub resistant_to_cold: bool, // Resistance to cold
    pub resistant_to_acid: bool, // Resistance to acid
    pub regenerate_hp: bool,   // Regenerate hit pts
    pub resistant_to_light: bool, // Resistance to light
    pub free_fall: bool,       // No damage falling
    pub sustain_str: bool,     // Keep strength
    pub sustain_int: bool,     // Keep intelligence
    pub sustain_wis: bool,     // Keep wisdom
    pub sustain_con: bool,     // Keep constitution
    pub sustain_dex: bool,     // Keep dexterity
    pub sustain_chr: bool,     // Keep charisma
    pub confuse_monster: bool, // Glowing hands.

    pub new_spells_to_learn: u8,       // Number of spells can learn.
    pub spells_learnt: u32,            // bit mask of spells learned
    pub spells_worked: u32,            // bit mask of spells tried and worked
    pub spells_forgotten: u32,         // bit mask of spells learned but forgotten
    pub spells_learned_order: [u8; 32], // order spells learned/remembered/forgotten
}

#[derive(Default)]
pub struct PlayerPack {
    pub unique_items: i16, // unique inventory items in pack
    pub weight: i16,       // Weight of currently carried items
    pub heaviness: i16,    // Heaviness of pack - used to calculate if pack is too heavy -CJS-
}

// Player contains everything to be known about our player character
pub struct Player {
    pub misc: PlayerMisc,
    pub stats: PlayerStats,
    pub flags: PlayerFlags,

    pub pos: Coord,    // location in dungeon
    pub prev_dir: i32, // Direction memory (1-9). -CJS-

    // calculated base hp values at each level, store them so that
    // drain life + restore life does not affect hit points.
    pub base_hp_levels: [u16; PLAYER_MAX_LEVEL],

    // Base experience levels, may be adjusted up for race and/or class
    pub base_exp_levels: [u32; PLAYER_MAX_LEVEL],

    pub running_tracker: u8,        // Tracker for number of turns taken during one run cycle
    pub temporary_light_only: bool, // Track if temporary light about player

    pub max_score: i32, // Maximum score attained

    pub pack: PlayerPack,

    pub inventory: [Inventory; PLAYER_INVENTORY_SIZE],

    pub equipment_count: i16,  // Number of equipped items
    pub weapon_is_heavy: bool, // Weapon is too heavy -CJS-
    pub carrying_light: bool,  // `true` when player is carrying light
}

impl Default for Player {
    fn default() -> Self {
        Player {
            misc: PlayerMisc::default(),
            stats: PlayerStats::default(),
            flags: PlayerFlags::default(),
            pos: Coord::default(),
            prev_dir: ' ' as i32,
            base_hp_levels: [0; PLAYER_MAX_LEVEL],
            base_exp_levels: [0; PLAYER_MAX_LEVEL],
            running_tracker: 0,
            temporary_light_only: false,
            max_score: 0,
            pack: PlayerPack::default(),
            inventory: [Inventory::empty(); PLAYER_INVENTORY_SIZE],
            equipment_count: 0,
            weapon_is_heavy: false,
            carrying_light: false,
        }
    }
}

// ------------------------------------------------------------------
// Player specific functions (port of player.cpp)
// ------------------------------------------------------------------

// re-export the stats functions most callers use via the player module
pub use crate::player_stats::{
    player_calculate_hit_points, player_disarm_adjustment, player_stat_adjustment_wisdom_intelligence,
    player_stat_random_decrease,
};


use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::data_player::{CLASSES, CLASS_LEVEL_ADJ, CLASS_RANK_TITLES, MAGIC_SPELLS, SPELL_NAMES};
use crate::dice::{dice_roll, Dice};
use crate::dungeon::{
    coord_distance_between, dg, dungeon_lite_spot, dungeon_move_creature_record,
};
use crate::dungeon_tile::{MAX_CAVE_ROOM, MIN_CLOSED_SPACE, TILE_BLOCKED_FLOOR, TILE_CORR_FLOOR};
use crate::game::{game, get_direction_with_memory, random_number};
use crate::helpers::{is_vowel, string_to_number};
use crate::identification::{
    item_description, object_blocked_by_monster, spell_item_identified,
    spell_item_identify_and_remove_random_inscription, SpecialNameIds,
};
use crate::inventory::{
    inventory_collect_all_item_flags, inventory_item_copy_to, inventory_item_is_cursed,
    inventory_item_remove_curse, PlayerEquipment,
};
use crate::monster::{monster_death, monster_take_hit, monsters, next_free_monster_id, MON_MAX_LEVELS};
use crate::player_stats::{
    player_armor_class_adjustment, player_attack_blows, player_damage_adjustment,
    player_stat_boost, player_to_hit_adjustment,
};
use crate::recall_data::creature_recall;
use crate::treasure::{
    TV_BOW, TV_CHEST, TV_CLOSED_DOOR, TV_INVIS_TRAP, TV_MAGIC_BOOK, TV_NOTHING, TV_OPEN_DOOR,
    TV_SECRET_DOOR, TV_SLING_AMMO, TV_SPIKE,
};
use crate::ui::{
    coord_inside_panel, display_character_experience, display_spells_list, dungeon_reset_view,
    print_character_current_hit_points, print_character_movement_state, print_character_speed,
};
use crate::ui_io::{
    erase_line, flush_input_buffer, get_menu_item_id, get_string_input, message_line_clear,
    print_message, print_message_no_command_interrupt, put_qio, put_string_clear_to_eol,
    terminal_bell_sound, terminal_restore_screen, terminal_save_screen,
};

const WIELD: usize = PlayerEquipment::Wield as usize;
const AUXILIARY: usize = PlayerEquipment::Auxiliary as usize;
const LIGHT: usize = PlayerEquipment::Light as usize;

fn player_reset_flags() {
    py().flags.see_invisible = false;
    py().flags.teleport = false;
    py().flags.free_action = false;
    py().flags.slow_digest = false;
    py().flags.aggravate = false;
    py().flags.sustain_str = false;
    py().flags.sustain_int = false;
    py().flags.sustain_wis = false;
    py().flags.sustain_con = false;
    py().flags.sustain_dex = false;
    py().flags.sustain_chr = false;
    py().flags.resistant_to_fire = false;
    py().flags.resistant_to_acid = false;
    py().flags.resistant_to_cold = false;
    py().flags.regenerate_hp = false;
    py().flags.resistant_to_light = false;
    py().flags.free_fall = false;
}

pub fn player_is_male() -> bool {
    py().misc.gender
}

pub fn player_set_gender(is_male: bool) {
    py().misc.gender = is_male;
}

pub fn player_get_gender_label() -> &'static str {
    if player_is_male() {
        "Male"
    } else {
        "Female"
    }
}

// Given direction "dir", returns new row, column location -RAK-
pub fn player_move_position(dir: i32, coord: &mut Coord) -> bool {
    let new_coord = match dir {
        1 => Coord::new(coord.y + 1, coord.x - 1),
        2 => Coord::new(coord.y + 1, coord.x),
        3 => Coord::new(coord.y + 1, coord.x + 1),
        4 => Coord::new(coord.y, coord.x - 1),
        5 => Coord::new(coord.y, coord.x),
        6 => Coord::new(coord.y, coord.x + 1),
        7 => Coord::new(coord.y - 1, coord.x - 1),
        8 => Coord::new(coord.y - 1, coord.x),
        9 => Coord::new(coord.y - 1, coord.x + 1),
        _ => Coord::new(0, 0),
    };

    let mut can_move = false;

    if new_coord.y >= 0 && new_coord.y < dg().height as i32 && new_coord.x >= 0 && new_coord.x < dg().width as i32 {
        *coord = new_coord;
        can_move = true;
    }

    can_move
}

// Teleport the player to a new location -RAK-
pub fn player_teleport(new_distance: i32) {
    let mut location;

    loop {
        location = Coord::new(random_number(dg().height as i32) - 1, random_number(dg().width as i32) - 1);

        while coord_distance_between(location, py().pos) > new_distance {
            location.y += (py().pos.y - location.y) / 2;
            location.x += (py().pos.x - location.x) / 2;
        }

        let tile = &dg().floor[location.y as usize][location.x as usize];
        if tile.feature_id < MIN_CLOSED_SPACE && tile.creature_id < 2 {
            break;
        }
    }

    dungeon_move_creature_record(py().pos, location);

    for y in (py().pos.y - 1)..=(py().pos.y + 1) {
        for x in (py().pos.x - 1)..=(py().pos.x + 1) {
            let spot = Coord::new(y, x);
            dg().floor[y as usize][x as usize].temporary_light = false;
            dungeon_lite_spot(spot);
        }
    }

    dungeon_lite_spot(py().pos);

    py().pos = location;

    dungeon_reset_view();
    crate::monster::update_monsters(false);

    game().teleport_player = false;
}

// Returns true if player has no light -RAK-
pub fn player_no_light() -> bool {
    let tile = &dg().floor[py().pos.y as usize][py().pos.x as usize];
    !tile.temporary_light && !tile.permanent_light
}

// Something happens to disturb the player. -CJS-
// The first arg indicates a major disturbance, which affects search.
// The second arg indicates a light change.
pub fn player_disturb(major_disturbance: i32, light_disturbance: i32) {
    game().command_count = 0;

    if major_disturbance != 0 && (py().flags.status & config::player::status::PY_SEARCH) != 0 {
        player_search_off();
    }

    if py().flags.rest != 0 {
        player_rest_off();
    }

    if light_disturbance != 0 || py().running_tracker != 0 {
        py().running_tracker = 0;
        dungeon_reset_view();
    }

    flush_input_buffer();
}

// Search Mode enhancement -RAK-
pub fn player_search_on() {
    player_change_speed(1);

    py().flags.status |= config::player::status::PY_SEARCH;

    print_character_movement_state();
    print_character_speed();

    py().flags.food_digested += 1;
}

pub fn player_search_off() {
    dungeon_reset_view();
    player_change_speed(-1);

    py().flags.status &= !config::player::status::PY_SEARCH;

    print_character_movement_state();
    print_character_speed();
    py().flags.food_digested -= 1;
}

// Resting allows a player to safely restore his hp -RAK-
pub fn player_rest_on() {
    let mut rest_num: i32;

    if game().command_count > 0 {
        rest_num = game().command_count as i32;
        game().command_count = 0;
    } else {
        rest_num = 0;

        put_string_clear_to_eol("Rest for how long? ", Coord::new(0, 0));

        let mut rest_str = String::new();
        if get_string_input(&mut rest_str, Coord::new(0, 19), 5) {
            if rest_str.starts_with('*') {
                rest_num = -(i16::MAX as i32);
            } else {
                string_to_number(&rest_str, &mut rest_num);
            }
        }
    }

    // check for reasonable value, must be positive number
    // in range of a short, or must be -MAX_SHORT
    if rest_num == -(i16::MAX as i32) || (rest_num > 0 && rest_num <= i16::MAX as i32) {
        if (py().flags.status & config::player::status::PY_SEARCH) != 0 {
            player_search_off();
        }

        py().flags.rest = rest_num as i16;
        py().flags.status |= config::player::status::PY_REST;
        print_character_movement_state();
        py().flags.food_digested -= 1;

        put_string_clear_to_eol("Press any key to stop resting...", Coord::new(0, 0));
        put_qio();

        return;
    }

    // Something went wrong
    if rest_num != 0 {
        print_message(Some("Invalid rest count."));
    }
    message_line_clear();

    game().player_free_turn = true;
}

pub fn player_rest_off() {
    py().flags.rest = 0;
    py().flags.status &= !config::player::status::PY_REST;

    print_character_movement_state();

    // flush last message, or delete "press any key" message
    print_message(None);

    py().flags.food_digested += 1;
}

// For "DIED_FROM" string
pub fn player_died_from_string(monster_name: &str, move_flags: u32) -> String {
    if (move_flags & config::monsters::move_flags::CM_WIN) != 0 {
        format!("The {}", monster_name)
    } else if is_vowel(monster_name.chars().next().unwrap_or(' ')) {
        format!("an {}", monster_name)
    } else {
        format!("a {}", monster_name)
    }
}

pub fn player_test_attack_hits(attack_id: i32, level: u8) -> bool {
    let ac = py().misc.ac as i32 + py().misc.magical_ac as i32;
    let level = level as i32;

    match attack_id {
        1 => player_test_being_hit(60, level, 0, ac, CLASS_MISC_HIT),  // Normal attack
        2 => player_test_being_hit(-3, level, 0, ac, CLASS_MISC_HIT),  // Lose Strength
        3 | 4 | 5 => player_test_being_hit(10, level, 0, ac, CLASS_MISC_HIT), // Confusion/Fear/Fire attack
        6 => player_test_being_hit(0, level, 0, ac, CLASS_MISC_HIT),   // Acid attack
        7 | 8 => player_test_being_hit(10, level, 0, ac, CLASS_MISC_HIT), // Cold/Lightning attack
        9 => player_test_being_hit(0, level, 0, ac, CLASS_MISC_HIT),   // Corrosion attack
        10 | 11 => player_test_being_hit(2, level, 0, ac, CLASS_MISC_HIT), // Blindness/Paralysis attack
        12 => player_test_being_hit(5, level, 0, py().misc.level as i32, CLASS_MISC_HIT) && py().misc.au > 0, // Steal Money
        13 => player_test_being_hit(2, level, 0, py().misc.level as i32, CLASS_MISC_HIT) && py().pack.unique_items > 0, // Steal Object
        14 => player_test_being_hit(5, level, 0, ac, CLASS_MISC_HIT),  // Poison
        15 | 16 => player_test_being_hit(0, level, 0, ac, CLASS_MISC_HIT), // Lose dexterity/constitution
        17 | 18 => player_test_being_hit(2, level, 0, ac, CLASS_MISC_HIT), // Lose intelligence/wisdom
        19 => player_test_being_hit(5, level, 0, ac, CLASS_MISC_HIT),  // Lose experience
        20 => true,                                                    // Aggravate monsters
        21 => player_test_being_hit(20, level, 0, ac, CLASS_MISC_HIT), // Disenchant
        22 | 23 => player_test_being_hit(5, level, 0, ac, CLASS_MISC_HIT), // Eat food/light
        24 => player_test_being_hit(15, level, 0, ac, CLASS_MISC_HIT) && py().pack.unique_items > 0, // Eat charges
        99 => true,
        _ => false,
    }
}

// Changes speed of monsters relative to player -RAK-
// Note: When the player is sped up or slowed down, I simply change
// the speed of all the monsters. This greatly simplified the logic.
pub fn player_change_speed(speed: i32) {
    py().flags.speed += speed as i16;
    py().flags.status |= config::player::status::PY_SPEED;

    let mut i = *next_free_monster_id() as i32 - 1;
    while i >= config::monsters::MON_MIN_INDEX_ID as i32 {
        monsters()[i as usize].speed += speed as i16;
        i -= 1;
    }
}

// Player bonuses -RAK-
//
// When an item is worn or taken off, this re-adjusts the player bonuses.
//     Factor =  1 : wear
//     Factor = -1 : removed
//
// Only calculates properties with cumulative effect.  Properties that
// depend on everything being worn are recalculated by player_recalculate_bonuses() -CJS-
pub fn player_adjust_bonuses_for_item(item: crate::inventory::Inventory, factor: i32) {
    let amount = item.misc_use as i32 * factor;

    if (item.flags & config::treasure::flags::TR_STATS) != 0 {
        for i in 0..6 {
            if ((1 << i) & item.flags) != 0 {
                player_stat_boost(i, amount);
            }
        }
    }

    if (item.flags & config::treasure::flags::TR_SEARCH) != 0 {
        py().misc.chance_in_search += amount as i16;
        py().misc.fos -= amount as i16;
    }

    if (item.flags & config::treasure::flags::TR_STEALTH) != 0 {
        py().misc.stealth_factor += amount as i16;
    }

    if (item.flags & config::treasure::flags::TR_SPEED) != 0 {
        player_change_speed(-amount);
    }

    if (item.flags & config::treasure::flags::TR_BLIND) != 0 && factor > 0 {
        py().flags.blind += 1000;
    }

    if (item.flags & config::treasure::flags::TR_TIMID) != 0 && factor > 0 {
        py().flags.afraid += 50;
    }

    if (item.flags & config::treasure::flags::TR_INFRA) != 0 {
        py().flags.see_infra += amount as i16;
    }
}

fn player_recalculate_bonuses_from_inventory() {
    for i in WIELD..LIGHT {
        let item = py().inventory[i];

        if item.category_id != TV_NOTHING {
            py().misc.plusses_to_hit += item.to_hit;

            // Bows can't damage. -CJS-
            if item.category_id != TV_BOW {
                py().misc.plusses_to_damage += item.to_damage;
            }

            py().misc.magical_ac += item.to_ac;
            py().misc.ac += item.ac;

            if spell_item_identified(&item) {
                py().misc.display_to_hit += item.to_hit;

                // Bows can't damage. -CJS-
                if item.category_id != TV_BOW {
                    py().misc.display_to_damage += item.to_damage;
                }

                py().misc.display_to_ac += item.to_ac;
                py().misc.display_ac += item.ac;
            } else if !inventory_item_is_cursed(&item) {
                // Base AC values should always be visible,
                // as long as the item is not cursed.
                py().misc.display_ac += item.ac;
            }
        }
    }
}

fn player_recalculate_sustain_stats_from_inventory() {
    for i in WIELD..LIGHT {
        if (py().inventory[i].flags & config::treasure::flags::TR_SUST_STAT) == 0 {
            continue;
        }

        match py().inventory[i].misc_use {
            1 => py().flags.sustain_str = true,
            2 => py().flags.sustain_int = true,
            3 => py().flags.sustain_wis = true,
            4 => py().flags.sustain_con = true,
            5 => py().flags.sustain_dex = true,
            6 => py().flags.sustain_chr = true,
            _ => {}
        }
    }
}

// Recalculate the effect of all the stuff we use. -CJS-
pub fn player_recalculate_bonuses() {
    // Temporarily adjust food_digested
    if py().flags.slow_digest {
        py().flags.food_digested += 1;
    }
    if py().flags.regenerate_hp {
        py().flags.food_digested -= 3;
    }

    let saved_display_ac = py().misc.display_ac;

    player_reset_flags();

    // Real values
    py().misc.plusses_to_hit = player_to_hit_adjustment();
    py().misc.plusses_to_damage = player_damage_adjustment();
    py().misc.magical_ac = player_armor_class_adjustment();
    py().misc.ac = 0;

    // Display values
    py().misc.display_to_hit = py().misc.plusses_to_hit;
    py().misc.display_to_damage = py().misc.plusses_to_damage;
    py().misc.display_ac = 0;
    py().misc.display_to_ac = py().misc.magical_ac;

    player_recalculate_bonuses_from_inventory();

    py().misc.display_ac += py().misc.display_to_ac;

    if py().weapon_is_heavy {
        py().misc.display_to_hit += (py().stats.used[A_STR] as i16) * 15 - py().inventory[WIELD].weight as i16;
    }

    // Add in temporary spell increases
    if py().flags.invulnerability > 0 {
        py().misc.ac += 100;
        py().misc.display_ac += 100;
    }

    if py().flags.blessed > 0 {
        py().misc.ac += 2;
        py().misc.display_ac += 2;
    }

    if py().flags.detect_invisible > 0 {
        py().flags.see_invisible = true;
    }

    // can't print AC here because might be in a store
    if saved_display_ac != py().misc.display_ac {
        py().flags.status |= config::player::status::PY_ARMOR;
    }

    let item_flags = inventory_collect_all_item_flags();

    if (item_flags & config::treasure::flags::TR_SLOW_DIGEST) != 0 {
        py().flags.slow_digest = true;
    }
    if (item_flags & config::treasure::flags::TR_AGGRAVATE) != 0 {
        py().flags.aggravate = true;
    }
    if (item_flags & config::treasure::flags::TR_TELEPORT) != 0 {
        py().flags.teleport = true;
    }
    if (item_flags & config::treasure::flags::TR_REGEN) != 0 {
        py().flags.regenerate_hp = true;
    }
    if (item_flags & config::treasure::flags::TR_RES_FIRE) != 0 {
        py().flags.resistant_to_fire = true;
    }
    if (item_flags & config::treasure::flags::TR_RES_ACID) != 0 {
        py().flags.resistant_to_acid = true;
    }
    if (item_flags & config::treasure::flags::TR_RES_COLD) != 0 {
        py().flags.resistant_to_cold = true;
    }
    if (item_flags & config::treasure::flags::TR_FREE_ACT) != 0 {
        py().flags.free_action = true;
    }
    if (item_flags & config::treasure::flags::TR_SEE_INVIS) != 0 {
        py().flags.see_invisible = true;
    }
    if (item_flags & config::treasure::flags::TR_RES_LIGHT) != 0 {
        py().flags.resistant_to_light = true;
    }
    if (item_flags & config::treasure::flags::TR_FFALL) != 0 {
        py().flags.free_fall = true;
    }

    player_recalculate_sustain_stats_from_inventory();

    // Reset food_digested values
    if py().flags.slow_digest {
        py().flags.food_digested -= 1;
    }
    if py().flags.regenerate_hp {
        py().flags.food_digested += 3;
    }
}

// Remove item from equipment list -RAK-
pub fn player_take_off(item_id: i32, pack_position_id: i32) {
    py().flags.status |= config::player::status::PY_STR_WGT;

    let item_id = item_id as usize;
    let item = py().inventory[item_id];

    py().pack.weight -= item.weight as i16 * item.items_count as i16;
    py().equipment_count -= 1;

    let p = if item_id == WIELD || item_id == AUXILIARY {
        "Was wielding "
    } else if item_id == LIGHT {
        "Light source was "
    } else {
        "Was wearing "
    };

    let description = item_description(&item, true);

    let msg = if pack_position_id >= 0 {
        format!("{}{} ({})", p, description, (b'a' + pack_position_id as u8) as char)
    } else {
        format!("{}{}", p, description)
    };
    print_message(Some(&msg));

    // For secondary weapon
    if item_id != AUXILIARY {
        player_adjust_bonuses_for_item(item, -1);
    }

    inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut py().inventory[item_id]);
}

// Attacker's level and plusses,  defender's AC -RAK-
pub fn player_test_being_hit(base_to_hit: i32, level: i32, plus_to_hit: i32, armor_class: i32, attack_type_id: usize) -> bool {
    player_disturb(1, 0);

    // `plus_to_hit` could be less than 0 if player wielding weapon too heavy for them
    let hit_chance = base_to_hit + plus_to_hit * BTH_PER_PLUS_TO_HIT_ADJUST + (level * CLASS_LEVEL_ADJ[py().misc.class_id as usize][attack_type_id] as i32);

    // always miss 1 out of 20, always hit 1 out of 20
    let die = random_number(20);

    // normal hit
    die != 1 && (die == 20 || (hit_chance > 0 && random_number(hit_chance) > armor_class))
}

// Decreases players hit points and sets game.character_is_dead flag if necessary -RAK-
pub fn player_takes_hit(damage: i32, creature_name_label: &str) {
    let mut damage = damage;

    if py().flags.invulnerability > 0 {
        damage = 0;
    }
    py().misc.current_hp -= damage as i16;

    if py().misc.current_hp >= 0 {
        print_character_current_hit_points();
        return;
    }

    if !game().character_is_dead {
        game().character_is_dead = true;

        game().character_died_from = creature_name_label.to_string();

        game().total_winner = false;
    }

    dg().generate_new_level = true;
}

// Searches for hidden things. -RAK-
pub fn player_search(coord: Coord, chance: i32) {
    let mut chance = chance;

    if py().flags.confused > 0 {
        chance /= 10;
    }

    if py().flags.blind > 0 || player_no_light() {
        chance /= 10;
    }

    if py().flags.image > 0 {
        chance /= 10;
    }

    for y in (coord.y - 1)..=(coord.y + 1) {
        for x in (coord.x - 1)..=(coord.x + 1) {
            let spot = Coord::new(y, x);

            // always coord_in_bounds() here
            if random_number(100) >= chance {
                continue;
            }

            let treasure_id = dg().floor[y as usize][x as usize].treasure_id as usize;
            if treasure_id == 0 {
                continue;
            }

            // Search for hidden objects

            let category_id = game().treasure.list[treasure_id].category_id;

            if category_id == TV_INVIS_TRAP {
                // Trap on floor?

                let description = item_description(&game().treasure.list[treasure_id], true);
                let msg = format!("You have found {}", description);
                print_message(Some(&msg));

                crate::dungeon::trap_change_visibility(spot);
                crate::player_run::player_end_running();
            } else if category_id == TV_SECRET_DOOR {
                // Secret door?

                print_message(Some("You have found a secret door."));

                crate::dungeon::trap_change_visibility(spot);
                crate::player_run::player_end_running();
            } else if category_id == TV_CHEST {
                // Chest is trapped?

                // mask out the treasure bits
                if (game().treasure.list[treasure_id].flags & config::treasure::chests::CH_TRAPPED) > 1 {
                    if !spell_item_identified(&game().treasure.list[treasure_id]) {
                        spell_item_identify_and_remove_random_inscription(&mut game().treasure.list[treasure_id]);
                        print_message(Some("You have discovered a trap on the chest!"));
                    } else {
                        print_message(Some("The chest is trapped!"));
                    }
                }
            }
        }
    }
}

// Computes current weight limit -RAK-
pub fn player_carrying_load_limit() -> i32 {
    let mut weight_cap = py().stats.used[A_STR] as i32 * config::player::PLAYER_WEIGHT_CAP as i32 + py().misc.weight as i32;

    if weight_cap > 3000 {
        weight_cap = 3000;
    }

    weight_cap
}

// Are we strong enough for the current pack and weapon? -CJS-
pub fn player_strength() {
    let item = py().inventory[WIELD];

    if item.category_id != TV_NOTHING && (py().stats.used[A_STR] as i32) * 15 < item.weight as i32 {
        if !py().weapon_is_heavy {
            print_message(Some("You have trouble wielding such a heavy weapon."));
            py().weapon_is_heavy = true;
            player_recalculate_bonuses();
        }
    } else if py().weapon_is_heavy {
        py().weapon_is_heavy = false;
        if item.category_id != TV_NOTHING {
            print_message(Some("You are strong enough to wield your weapon."));
        }
        player_recalculate_bonuses();
    }

    let mut limit = player_carrying_load_limit();

    if limit < py().pack.weight as i32 {
        limit = py().pack.weight as i32 / (limit + 1);
    } else {
        limit = 0;
    }

    if py().pack.heaviness as i32 != limit {
        if (py().pack.heaviness as i32) < limit {
            print_message(Some("Your pack is so heavy that it slows you down."));
        } else {
            print_message(Some("You move more easily under the weight of your pack."));
        }
        player_change_speed(limit - py().pack.heaviness as i32);
        py().pack.heaviness = limit as i16;
    }

    py().flags.status &= !config::player::status::PY_STR_WGT;
}

pub fn player_left_hand_ring_empty() -> bool {
    py().inventory[PlayerEquipment::Left as usize].category_id == TV_NOTHING
}

pub fn player_right_hand_ring_empty() -> bool {
    py().inventory[PlayerEquipment::Right as usize].category_id == TV_NOTHING
}

pub fn player_is_wielding_item() -> bool {
    !(py().inventory[WIELD].category_id == TV_NOTHING && py().inventory[AUXILIARY].category_id == TV_NOTHING)
}

pub fn player_worn_item_is_cursed(id: PlayerEquipment) -> bool {
    inventory_item_is_cursed(&py().inventory[id as usize])
}

pub fn player_worn_item_remove_curse(id: PlayerEquipment) {
    inventory_item_remove_curse(&mut py().inventory[id as usize]);
}

fn player_can_read() -> bool {
    if py().flags.blind > 0 {
        print_message(Some("You can't see to read your spell book!"));
        return false;
    }

    if player_no_light() {
        print_message(Some("You have no light to read by."));
        return false;
    }

    true
}

fn last_known_spell() -> usize {
    for last_known in 0..32 {
        if py().flags.spells_learned_order[last_known] == 99 {
            return last_known;
        }
    }

    // We should never actually reach this, but just in case... -MRC-
    0
}

fn player_determine_learnable_spells() -> u32 {
    let mut spell_flag = 0;

    for i in 0..py().pack.unique_items as usize {
        if py().inventory[i].category_id == TV_MAGIC_BOOK {
            spell_flag |= py().inventory[i].flags;
        }
    }

    spell_flag
}

// gain spells when player wants to -JW-
pub fn player_gain_spells() {
    if py().flags.confused > 0 {
        print_message(Some("You are too confused."));
        return;
    }

    let mut new_spells = py().flags.new_spells_to_learn as i32;
    let mut diff_spells = 0;

    let class_id = py().misc.class_id as usize;

    let stat;
    let offset;

    // Priests don't need light because they get spells from their god, so only
    // fail when can't see if player has SPELL_TYPE_MAGE spells. This check is done below.
    if CLASSES[class_id].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        // People with SPELL_TYPE_MAGE spells can't learn spell_bank if they can't read their books.
        if !player_can_read() {
            return;
        }
        stat = A_INT;
        offset = config::spells::NAME_OFFSET_SPELLS as i32;
    } else {
        stat = A_WIS;
        offset = config::spells::NAME_OFFSET_PRAYERS as i32;
    }

    let mut last_known = last_known_spell();

    if new_spells == 0 {
        let tmp_str = format!("You can't learn any new {}s!", if stat == A_INT { "spell" } else { "prayer" });
        print_message(Some(&tmp_str));

        game().player_free_turn = true;
        return;
    }

    // determine which spells player can learn
    // mages need the book to learn a spell, priests do not need the book
    let mut spell_flag: u32 = if stat == A_INT {
        player_determine_learnable_spells()
    } else {
        0x7FFFFFFF
    };

    // clear bits for spells already learned
    spell_flag &= !py().flags.spells_learnt;

    let mut spell_id = 0;
    let mut spell_bank = [0i32; 31];
    let mut mask: u32 = 0x1;

    let mut i = 0;
    while spell_flag != 0 {
        if (spell_flag & mask) != 0 {
            spell_flag &= !mask;
            if MAGIC_SPELLS[class_id - 1][i].level_required as u16 <= py().misc.level {
                spell_bank[spell_id] = i as i32;
                spell_id += 1;
            }
        }
        mask <<= 1;
        i += 1;
    }

    if new_spells > spell_id as i32 {
        print_message(Some("You seem to be missing a book."));

        diff_spells = new_spells - spell_id as i32;
        new_spells = spell_id as i32;
    }

    if new_spells == 0 {
        // do nothing
    } else if stat == A_INT {
        // get to choose which mage spells will be learned
        terminal_save_screen();
        display_spells_list(&spell_bank, spell_id as i32, false, -1);

        let mut query = '\0';
        while new_spells != 0 && get_menu_item_id("Learn which spell?", &mut query) {
            let c = query as i32 - 'a' as i32;

            // test j < 23 in case i is greater than 22, only 22 spells
            // are actually shown on the screen, so limit choice to those
            if c >= 0 && c < spell_id as i32 && c < 22 {
                let c = c as usize;
                new_spells -= 1;

                py().flags.spells_learnt |= 1 << spell_bank[c];
                py().flags.spells_learned_order[last_known] = spell_bank[c] as u8;
                last_known += 1;

                for j in c..(spell_id - 1) {
                    spell_bank[j] = spell_bank[j + 1];
                }

                spell_id -= 1;

                erase_line(Coord::new(spell_id as i32 + 1, 31));
                display_spells_list(&spell_bank, spell_id as i32, false, -1);
            } else {
                terminal_bell_sound();
            }
        }

        terminal_restore_screen();
    } else {
        // pick a prayer at random
        while new_spells != 0 {
            let id = (random_number(spell_id as i32) - 1) as usize;
            py().flags.spells_learnt |= 1 << spell_bank[id];
            py().flags.spells_learned_order[last_known] = spell_bank[id] as u8;
            last_known += 1;

            let tmp_str = format!("You have learned the prayer of {}.", SPELL_NAMES[(spell_bank[id] + offset) as usize]);
            print_message(Some(&tmp_str));

            for j in id..(spell_id - 1) {
                spell_bank[j] = spell_bank[j + 1];
            }

            spell_id -= 1;
            new_spells -= 1;
        }
    }

    py().flags.new_spells_to_learn = (new_spells + diff_spells) as u8;

    if py().flags.new_spells_to_learn == 0 {
        py().flags.status |= config::player::status::PY_STUDY;
    }

    // set the mana for first level characters when they learn their first spell.
    if py().misc.mana == 0 {
        player_gain_mana(stat);
    }
}

fn new_mana(stat: usize) -> i32 {
    let levels = py().misc.level as i32 - CLASSES[py().misc.class_id as usize].min_level_for_spell_casting as i32 + 1;

    match crate::player_stats::player_stat_adjustment_wisdom_intelligence(stat) {
        1 | 2 => levels,
        3 => 3 * levels / 2,
        4 => 2 * levels,
        5 => 5 * levels / 2,
        6 => 3 * levels,
        7 => 4 * levels,
        _ => 0,
    }
}

// Gain some mana if you know at least one spell -RAK-
pub fn player_gain_mana(stat: usize) {
    if py().flags.spells_learnt != 0 {
        let mut mana = new_mana(stat);

        // increment mana by one, so that first level chars have 2 mana
        if mana > 0 {
            mana += 1;
        }

        // mana can be zero when creating character
        if py().misc.mana as i32 != mana {
            if py().misc.mana != 0 {
                // change current mana proportionately to change of max mana,
                // divide first to avoid overflow, little loss of accuracy
                let value = (((py().misc.current_mana as i32) << 16) + py().misc.current_mana_fraction as i32) / py().misc.mana as i32 * mana;
                py().misc.current_mana = (value >> 16) as i16;
                py().misc.current_mana_fraction = (value & 0xFFFF) as u16;
            } else {
                py().misc.current_mana = mana as i16;
                py().misc.current_mana_fraction = 0;
            }

            py().misc.mana = mana as i16;

            // can't print mana here, may be in store or inventory mode
            py().flags.status |= config::player::status::PY_MANA;
        }
    } else if py().misc.mana != 0 {
        py().misc.mana = 0;
        py().misc.current_mana = 0;

        // can't print mana here, may be in store or inventory mode
        py().flags.status |= config::player::status::PY_MANA;
    }
}

// Critical hits, Nasty way to die. -RAK-
pub fn player_weapon_critical_blow(weapon_weight: i32, plus_to_hit: i32, damage: i32, attack_type_id: usize) -> i32 {
    let mut critical = damage;

    // Weight of weapon, plusses to hit, and character level all
    // contribute to the chance of a critical
    if random_number(5000) <= weapon_weight + 5 * plus_to_hit + (CLASS_LEVEL_ADJ[py().misc.class_id as usize][attack_type_id] as i32 * py().misc.level as i32) {
        let weapon_weight = weapon_weight + random_number(650);

        if weapon_weight < 400 {
            critical = 2 * damage + 5;
            print_message(Some("It was a good hit! (x2 damage)"));
        } else if weapon_weight < 700 {
            critical = 3 * damage + 10;
            print_message(Some("It was an excellent hit! (x3 damage)"));
        } else if weapon_weight < 900 {
            critical = 4 * damage + 15;
            print_message(Some("It was a superb hit! (x4 damage)"));
        } else {
            critical = 5 * damage + 20;
            print_message(Some("It was a *GREAT* hit! (x5 damage)"));
        }
    }

    critical
}

// Saving throws for player character. -RAK-
pub fn player_saving_throw() -> bool {
    let class_level_adjustment = CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_SAVE] as i32 * py().misc.level as i32 / 3;

    let saving = py().misc.saving_throw as i32 + crate::player_stats::player_stat_adjustment_wisdom_intelligence(A_WIS) + class_level_adjustment;

    random_number(100) <= saving
}

pub fn player_gain_kill_experience(creature_id: usize) {
    let creature = &CREATURES_LIST[creature_id];

    let exp: i32 = creature.kill_exp_value as i32 * creature.level as i32;

    let mut quotient = exp / py().misc.level as i32;
    let mut remainder = exp % py().misc.level as i32;

    remainder *= 0x10000;
    remainder /= py().misc.level as i32;
    remainder += py().misc.exp_fraction as i32;

    if remainder >= 0x10000 {
        quotient += 1;
        py().misc.exp_fraction = (remainder - 0x10000) as u16;
    } else {
        py().misc.exp_fraction = remainder as u16;
    }

    py().misc.exp += quotient;
}

fn player_calculate_to_hit_blows(weapon_id: u8, weapon_weight: u16, blows: &mut i32, total_to_hit: &mut i32) {
    if weapon_id != TV_NOTHING {
        // Proper weapon
        *blows = player_attack_blows(weapon_weight as i32, total_to_hit);
    } else {
        // Bare hands?
        *blows = 2;
        *total_to_hit = -3;
    }

    // Fix for arrows
    if weapon_id >= TV_SLING_AMMO && weapon_id <= TV_SPIKE {
        *blows = 1;
    }

    *total_to_hit += py().misc.plusses_to_hit as i32;
}

fn player_calculate_base_to_hit(creature_lit: bool, tot_tohit: i32) -> i32 {
    if creature_lit {
        return py().misc.bth as i32;
    }

    // creature not lit, make it more difficult to hit
    let mut bth = py().misc.bth as i32 / 2;
    bth -= tot_tohit * (BTH_PER_PLUS_TO_HIT_ADJUST - 1);
    bth -= py().misc.level as i32 * CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_BTH] as i32 / 2;

    bth
}

// Player attacks a (poor, defenseless) creature -RAK-
fn player_attack_monster(coord: Coord) {
    let creature_id = dg().floor[coord.y as usize][coord.x as usize].creature_id as usize;

    monsters()[creature_id].sleep_count = 0;

    let monster_creature_id = monsters()[creature_id].creature_id as usize;
    let monster_lit = monsters()[creature_id].lit;

    // Does the player know what they're fighting?
    let name = if !monster_lit {
        "it".to_string()
    } else {
        format!("the {}", CREATURES_LIST[monster_creature_id].name)
    };

    let item = py().inventory[WIELD];

    let mut blows = 0;
    let mut total_to_hit = 0;
    player_calculate_to_hit_blows(item.category_id, item.weight, &mut blows, &mut total_to_hit);

    let base_to_hit = player_calculate_base_to_hit(monster_lit, total_to_hit);

    // Loop for number of blows, trying to hit the critter.
    // Note: blows will always be greater than 0 at the start of the loop -MRC-
    let mut i = blows;
    while i > 0 {
        let creature_ac = CREATURES_LIST[monster_creature_id].ac;

        if !player_test_being_hit(base_to_hit, py().misc.level as i32, total_to_hit, creature_ac as i32, CLASS_BTH) {
            let msg = format!("You miss {}.", name);
            print_message(Some(&msg));
            i -= 1;
            continue;
        }

        let msg = format!("You hit {}.", name);
        print_message(Some(&msg));

        let item = py().inventory[WIELD];
        let mut damage;
        if item.category_id != TV_NOTHING {
            damage = dice_roll(item.damage);
            damage = crate::player_magic::item_magic_ability_damage(&item, damage, monster_creature_id);
            damage = player_weapon_critical_blow(item.weight as i32, total_to_hit, damage, CLASS_BTH);
        } else {
            // Bare hands!?
            damage = dice_roll(Dice::new(1, 1));
            damage = player_weapon_critical_blow(1, 0, damage, CLASS_BTH);
        }

        damage += py().misc.plusses_to_damage as i32;
        if damage < 0 {
            damage = 0;
        }

        if py().flags.confuse_monster {
            py().flags.confuse_monster = false;

            print_message(Some("Your hands stop glowing."));

            let creature_defenses = CREATURES_LIST[monster_creature_id].defenses;
            let creature_level = CREATURES_LIST[monster_creature_id].level;

            let msg;
            if (creature_defenses & config::monsters::defense::CD_NO_SLEEP) != 0 || random_number(MON_MAX_LEVELS as i32) < creature_level as i32 {
                msg = format!("{} is unaffected.", name);
            } else {
                msg = format!("{} appears confused.", name);
                if monsters()[creature_id].confused_amount != 0 {
                    monsters()[creature_id].confused_amount += 3;
                } else {
                    monsters()[creature_id].confused_amount = (2 + random_number(16)) as u8;
                }
            }
            print_message(Some(&msg));

            if monster_lit && random_number(4) == 1 {
                creature_recall()[monster_creature_id].defenses |= creature_defenses & config::monsters::defense::CD_NO_SLEEP;
            }
        }

        // See if we done it in.
        if monster_take_hit(creature_id as i32, damage) >= 0 {
            let msg = format!("You have slain {}.", name);
            print_message(Some(&msg));
            display_character_experience();

            return;
        }

        // Use missiles up
        let item = py().inventory[WIELD];
        if item.category_id >= TV_SLING_AMMO && item.category_id <= TV_SPIKE {
            py().inventory[WIELD].items_count -= 1;
            py().pack.weight -= item.weight as i16;
            py().flags.status |= config::player::status::PY_STR_WGT;

            if py().inventory[WIELD].items_count == 0 {
                py().equipment_count -= 1;
                player_adjust_bonuses_for_item(py().inventory[WIELD], -1);
                inventory_item_copy_to(config::dungeon::objects::OBJ_NOTHING as usize, &mut py().inventory[WIELD]);
                player_recalculate_bonuses();
            }
        }

        i -= 1;
    }
}

fn player_lock_picking_skill() -> i32 {
    let mut skill = py().misc.disarm as i32;

    skill += 2 * crate::player_stats::player_disarm_adjustment() as i32;
    skill += crate::player_stats::player_stat_adjustment_wisdom_intelligence(A_INT);
    skill += CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_DISARM] as i32 * py().misc.level as i32 / 3;

    skill
}

fn open_closed_door(coord: Coord) {
    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;

    let misc_use = game().treasure.list[treasure_id].misc_use;

    if misc_use > 0 {
        // It's locked.

        if py().flags.confused > 0 {
            print_message(Some("You are too confused to pick the lock."));
        } else if player_lock_picking_skill() - misc_use as i32 > random_number(100) {
            print_message(Some("You have picked the lock."));
            py().misc.exp += 1;
            display_character_experience();
            game().treasure.list[treasure_id].misc_use = 0;
        } else {
            print_message_no_command_interrupt("You failed to pick the lock.");
        }
    } else if misc_use < 0 {
        // It's stuck

        print_message(Some("It appears to be stuck."));
    }

    if game().treasure.list[treasure_id].misc_use == 0 {
        inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[treasure_id]);
        dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
        dungeon_lite_spot(coord);
        game().command_count = 0;
    }
}

fn open_closed_chest(coord: Coord) {
    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;

    let mut success = false;

    if (game().treasure.list[treasure_id].flags & config::treasure::chests::CH_LOCKED) != 0 {
        if py().flags.confused > 0 {
            print_message(Some("You are too confused to pick the lock."));
        } else if player_lock_picking_skill() - game().treasure.list[treasure_id].depth_first_found as i32 > random_number(100) {
            print_message(Some("You have picked the lock."));

            py().misc.exp += game().treasure.list[treasure_id].depth_first_found as i32;
            display_character_experience();

            success = true;
        } else {
            print_message_no_command_interrupt("You failed to pick the lock.");
        }
    } else {
        success = true;
    }

    if success {
        let item = &mut game().treasure.list[treasure_id];
        item.flags &= !config::treasure::chests::CH_LOCKED;
        item.special_name_id = SpecialNameIds::SnEmpty as u8;
        spell_item_identify_and_remove_random_inscription(item);
        item.cost = 0;
    }

    // Was chest still trapped?
    if (game().treasure.list[treasure_id].flags & config::treasure::chests::CH_LOCKED) != 0 {
        return;
    }

    // Oh, yes it was...   (Snicker)
    crate::player_traps::chest_trap(coord);

    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;
    if treasure_id != 0 {
        // Chest treasure is allocated as if a creature had been killed.
        // clear the cursed chest/monster win flag, so that people
        // can not win by opening a cursed chest
        inventory_item_remove_curse(&mut game().treasure.list[treasure_id]);

        monster_death(coord, game().treasure.list[treasure_id].flags);

        game().treasure.list[treasure_id].flags = 0;
    }
}

// Opens a closed door or closed chest. -RAK-
pub fn player_open_closed_object() {
    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = py().pos;
    player_move_position(dir, &mut coord);

    let mut no_object = false;

    let tile = dg().floor[coord.y as usize][coord.x as usize];
    let category_id = game().treasure.list[tile.treasure_id as usize].category_id;

    if tile.creature_id > 1 && tile.treasure_id != 0 && (category_id == TV_CLOSED_DOOR || category_id == TV_CHEST) {
        object_blocked_by_monster(tile.creature_id as usize);
    } else if tile.treasure_id != 0 {
        if category_id == TV_CLOSED_DOOR {
            open_closed_door(coord);
        } else if category_id == TV_CHEST {
            open_closed_chest(coord);
        } else {
            no_object = true;
        }
    } else {
        no_object = true;
    }

    if no_object {
        game().player_free_turn = true;
        print_message(Some("I do not see anything you can open there."));
    }
}

// Closes an open door. -RAK-
pub fn player_close_door() {
    let mut dir = 0;

    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = py().pos;
    player_move_position(dir, &mut coord);

    let tile = dg().floor[coord.y as usize][coord.x as usize];
    let treasure_id = tile.treasure_id as usize;

    let mut no_object = false;

    if tile.treasure_id != 0 {
        if game().treasure.list[treasure_id].category_id == TV_OPEN_DOOR {
            if tile.creature_id == 0 {
                if game().treasure.list[treasure_id].misc_use == 0 {
                    inventory_item_copy_to(config::dungeon::objects::OBJ_CLOSED_DOOR as usize, &mut game().treasure.list[treasure_id]);
                    dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_BLOCKED_FLOOR;
                    dungeon_lite_spot(coord);
                } else {
                    print_message(Some("The door appears to be broken."));
                }
            } else {
                object_blocked_by_monster(tile.creature_id as usize);
            }
        } else {
            no_object = true;
        }
    } else {
        no_object = true;
    }

    if no_object {
        game().player_free_turn = true;
        print_message(Some("I do not see anything you can close there."));
    }
}

// Tunneling through real wall: 10, 11, 12 -RAK-
// Used by TUNNEL and WALL_TO_MUD
pub fn player_tunnel_wall(coord: Coord, digging_ability: i32, digging_chance: i32) -> bool {
    if digging_ability <= digging_chance {
        return false;
    }

    let y = coord.y as usize;
    let x = coord.x as usize;

    if dg().floor[y][x].perma_lit_room {
        // Should become a room space, check to see whether
        // it should be TILE_LIGHT_FLOOR or TILE_DARK_FLOOR.
        let mut found = false;

        'outer: for yy in (coord.y - 1)..=(coord.y + 1) {
            if yy >= crate::dungeon::MAX_HEIGHT {
                break;
            }
            for xx in (coord.x - 1)..=(coord.x + 1) {
                if xx >= crate::dungeon::MAX_WIDTH {
                    break;
                }
                if dg().floor[yy as usize][xx as usize].feature_id <= MAX_CAVE_ROOM {
                    dg().floor[y][x].feature_id = dg().floor[yy as usize][xx as usize].feature_id;
                    dg().floor[y][x].permanent_light = dg().floor[yy as usize][xx as usize].permanent_light;
                    found = true;
                    break 'outer;
                }
            }
        }

        if !found {
            dg().floor[y][x].feature_id = TILE_CORR_FLOOR;
            dg().floor[y][x].permanent_light = false;
        }
    } else {
        // should become a corridor space
        dg().floor[y][x].feature_id = TILE_CORR_FLOOR;
        dg().floor[y][x].permanent_light = false;
    }

    dg().floor[y][x].field_mark = false;

    if coord_inside_panel(coord) && (dg().floor[y][x].temporary_light || dg().floor[y][x].permanent_light) && dg().floor[y][x].treasure_id != 0 {
        print_message(Some("You have found something!"));
    }

    dungeon_lite_spot(coord);

    true
}

// let the player attack the creature
pub fn player_attack_position(coord: Coord) {
    // Is a Coward?
    if py().flags.afraid > 0 {
        print_message(Some("You are too afraid!"));
        return;
    }

    player_attack_monster(coord);
}

// check to see if know any spells greater than level, eliminate them
fn eliminate_known_spells_greater_than_level(class_id: usize, p: &str, offset: i32) {
    let mut mask: u32 = 0x80000000;

    let mut i: i32 = 31;
    while mask != 0 {
        if (mask & py().flags.spells_learnt) != 0 {
            if MAGIC_SPELLS[class_id - 1][i as usize].level_required as u16 > py().misc.level {
                py().flags.spells_learnt &= !mask;
                py().flags.spells_forgotten |= mask;

                let msg = format!("You have forgotten the {} of {}.", p, SPELL_NAMES[(i + offset) as usize]);
                print_message(Some(&msg));
            } else {
                break;
            }
        }
        mask >>= 1;
        i -= 1;
    }
}

fn number_of_spells_allowed(stat: usize) -> i32 {
    let levels = py().misc.level as i32 - CLASSES[py().misc.class_id as usize].min_level_for_spell_casting as i32 + 1;

    match crate::player_stats::player_stat_adjustment_wisdom_intelligence(stat) {
        1 | 2 | 3 => levels,
        4 | 5 => 3 * levels / 2,
        6 => 2 * levels,
        7 => 5 * levels / 2,
        _ => 0,
    }
}

fn number_of_spells_known() -> i32 {
    let mut known = 0;

    let mut mask: u32 = 0x1;
    while mask != 0 {
        if (mask & py().flags.spells_learnt) != 0 {
            known += 1;
        }
        mask = mask.wrapping_shl(1);
        if mask == 0 {
            break;
        }
    }

    known
}

// remember forgotten spells while forgotten spells exist of new_spells_to_learn positive,
// remember the spells in the order that they were learned
fn remember_forgotten_spells(class_id: usize, allowed_spells: i32, new_spells: i32, p: &str, offset: i32) -> i32 {
    let mut new_spells = new_spells;
    let mut allowed_spells = allowed_spells;

    let mut n = 0;
    while py().flags.spells_forgotten != 0 && new_spells != 0 && n < allowed_spells && n < 32 {
        // order ID is (i+1)th spell learned
        let order_id = py().flags.spells_learned_order[n as usize];

        // shifting by amounts greater than number of bits in long gives
        // an undefined result, so don't shift for unknown spells
        let mask: u32 = if order_id == 99 { 0x0 } else { 1 << order_id };

        if (mask & py().flags.spells_forgotten) != 0 {
            if MAGIC_SPELLS[class_id - 1][order_id as usize].level_required as u16 <= py().misc.level {
                new_spells -= 1;
                py().flags.spells_forgotten &= !mask;
                py().flags.spells_learnt |= mask;

                let msg = format!("You have remembered the {} of {}.", p, SPELL_NAMES[(order_id as i32 + offset) as usize]);
                print_message(Some(&msg));
            } else {
                allowed_spells += 1;
            }
        }
        n += 1;
    }

    new_spells
}

// determine which spells player can learn must check all spells here,
// in gain_spell() we actually check if the books are present
fn learnable_spells(class_id: usize, new_spells: i32) -> i32 {
    let mut spell_flag = 0x7FFFFFFF & !py().flags.spells_learnt;

    let mut id = 0;
    let mut mask: u32 = 0x1;

    let mut i = 0;
    while spell_flag != 0 {
        if (spell_flag & mask) != 0 {
            spell_flag &= !mask;
            if MAGIC_SPELLS[class_id - 1][i].level_required as u16 <= py().misc.level {
                id += 1;
            }
        }
        mask <<= 1;
        i += 1;
    }

    if new_spells > id {
        id
    } else {
        new_spells
    }
}

// forget spells until new_spells_to_learn zero or no more spells know,
// spells are forgotten in the opposite order that they were learned
// NOTE: newSpells is always a negative value
fn forget_spells(new_spells: i32, p: &str, offset: i32) {
    let mut new_spells = new_spells;

    let mut i: i32 = 31;
    while new_spells != 0 && py().flags.spells_learnt != 0 {
        // orderID is the (i+1)th spell learned
        let order_id = py().flags.spells_learned_order[i as usize];

        // shifting by amounts greater than number of bits in long gives
        // an undefined result, so don't shift for unknown spells
        let mask: u32 = if order_id == 99 { 0x0 } else { 1 << order_id };

        if (mask & py().flags.spells_learnt) != 0 {
            py().flags.spells_learnt &= !mask;
            py().flags.spells_forgotten |= mask;
            new_spells += 1;

            let msg = format!("You have forgotten the {} of {}.", p, SPELL_NAMES[(order_id as i32 + offset) as usize]);
            print_message(Some(&msg));
        }
        i -= 1;
    }
}

// calculate number of spells player should have, and
// learn forget spells until that number is met -JEW-
pub fn player_calculate_allowed_spells_count(stat: usize) {
    let class_id = py().misc.class_id as usize;

    let magic_type_str;
    let offset;

    if stat == A_INT {
        magic_type_str = "spell";
        offset = config::spells::NAME_OFFSET_SPELLS as i32;
    } else {
        magic_type_str = "prayer";
        offset = config::spells::NAME_OFFSET_PRAYERS as i32;
    }

    // check to see if know any spells greater than level, eliminate them
    eliminate_known_spells_greater_than_level(class_id, magic_type_str, offset);

    // calc number of spells allowed
    let num_allowed = number_of_spells_allowed(stat);
    let num_known = number_of_spells_known();
    let mut new_spells = num_allowed - num_known;

    if new_spells > 0 {
        new_spells = remember_forgotten_spells(class_id, num_allowed, new_spells, magic_type_str, offset);

        // If `new_spells_to_learn` is still greater than zero
        if new_spells > 0 {
            new_spells = learnable_spells(class_id, new_spells);
        }
    } else if new_spells < 0 {
        forget_spells(new_spells, magic_type_str, offset);
        new_spells = 0;
    }

    if new_spells != py().flags.new_spells_to_learn as i32 {
        if new_spells > 0 && py().flags.new_spells_to_learn == 0 {
            let msg = format!("You can learn some new {}s now.", magic_type_str);
            print_message(Some(&msg));
        }

        py().flags.new_spells_to_learn = new_spells as u8;
        py().flags.status |= config::player::status::PY_STUDY;
    }
}

pub fn player_rank_title() -> String {
    let p: &str = if py().misc.level < 1 {
        "Babe in arms"
    } else if py().misc.level as usize <= PLAYER_MAX_LEVEL {
        CLASS_RANK_TITLES[py().misc.class_id as usize][py().misc.level as usize - 1]
    } else if player_is_male() {
        "**KING**"
    } else {
        "**QUEEN**"
    };

    p.to_string()
}

static PY: RacyCell<Option<Player>> = RacyCell::new(None);

// Access the global player object (the C code's `py` global).
pub fn py() -> &'static mut Player {
    let slot = PY.get();
    if slot.is_none() {
        *slot = Some(Player::default());
    }
    slot.as_mut().unwrap()
}
