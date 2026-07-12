// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::dice::Dice;
use crate::globals::RacyCell;
use crate::types::Coord;

// Monster is created for any living monster found on the current dungeon level
#[derive(Debug, Clone, Copy, Default)]
pub struct Monster {
    pub hp: i16,          // Hit points
    pub sleep_count: i16, // Inactive counter
    pub speed: i16,       // Movement speed
    pub creature_id: u16, // Pointer into creature

    // Note: fy, fx, and cdis constrain dungeon size to less than 256 by 256
    pub pos: Coord,                  // (y,x) Pointer into map
    pub distance_from_player: u8,    // Current distance from player

    pub lit: bool,
    pub stunned_amount: u8,
    pub confused_amount: u8,
}

impl Monster {
    pub const fn empty() -> Self {
        Monster {
            hp: 0,
            sleep_count: 0,
            speed: 0,
            creature_id: 0,
            pos: Coord::new(0, 0),
            distance_from_player: 0,
            lit: false,
            stunned_amount: 0,
            confused_amount: 0,
        }
    }
}

// Creature is a base data object.
// Holds the base game data for any given creature in the game such
// as: Kobold, Orc, Giant Red Ant, Quasit, Young Black Dragon, etc.
pub struct Creature {
    pub name: &'static str,      // Description of creature
    pub movement: u32,           // Bit field
    pub spells: u32,             // Creature spells
    pub defenses: u16,           // Bit field
    pub kill_exp_value: u16,     // Exp value for kill
    pub sleep_counter: u8,       // Inactive counter / 10
    pub area_affect_radius: u8,  // Area affect radius
    pub ac: u8,                  // AC
    pub speed: u8,               // Movement speed+10 (NOTE: +10 so that it can be an unsigned int)
    pub sprite: u8,              // Character representation (cchar)
    pub hit_die: Dice,           // Creatures hit die
    pub damage: [u8; 4],         // Type attack and damage
    pub level: u8,               // Level of creature
}

// MonsterAttack is a base data object.
// Holds the data for a monster's attack and damage type
pub struct MonsterAttack {
    pub type_id: u8,
    pub description_id: u8,
    pub dice: Dice,
}

// Creature constants
pub const MON_MAX_CREATURES: usize = 279; // Number of creatures defined for univ
pub const MON_ATTACK_TYPES: usize = 215; // Number of monster attack types.

// With MON_TOTAL_ALLOCATIONS set to 101, it is possible to get compacting
// monsters messages while breeding/cloning monsters.
pub const MON_TOTAL_ALLOCATIONS: usize = 125; // Max that can be allocated
pub const MON_MAX_LEVELS: usize = 40; // Maximum level of creatures
pub const MON_MAX_ATTACKS: usize = 4; // Max num attacks (used in mons memory) -CJS-

pub const BLANK_MONSTER: Monster = Monster::empty();

static HACK_MONPTR: RacyCell<i32> = RacyCell::new(-1);
static MONSTERS: RacyCell<[Monster; MON_TOTAL_ALLOCATIONS]> = RacyCell::new([Monster::empty(); MON_TOTAL_ALLOCATIONS]);
static MONSTER_LEVELS: RacyCell<[i16; MON_MAX_LEVELS + 1]> = RacyCell::new([0; MON_MAX_LEVELS + 1]);
static NEXT_FREE_MONSTER_ID: RacyCell<i16> = RacyCell::new(0);
static MONSTER_MULTIPLY_TOTAL: RacyCell<i16> = RacyCell::new(0);

pub fn hack_monptr() -> &'static mut i32 {
    HACK_MONPTR.get()
}

pub fn monsters() -> &'static mut [Monster; MON_TOTAL_ALLOCATIONS] {
    MONSTERS.get()
}

pub fn monster_levels() -> &'static mut [i16; MON_MAX_LEVELS + 1] {
    MONSTER_LEVELS.get()
}

pub fn next_free_monster_id() -> &'static mut i16 {
    NEXT_FREE_MONSTER_ID.get()
}

pub fn monster_multiply_total() -> &'static mut i16 {
    MONSTER_MULTIPLY_TOTAL.get()
}

// ------------------------------------------------------------------
// Handle monster movement and attacks (port of monster.cpp)
// ------------------------------------------------------------------

use crate::config;
use crate::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use crate::dice::dice_roll;
use crate::dungeon::{
    coord_distance_between, coord_in_bounds, dg, dungeon_delete_monster, dungeon_delete_monster_record,
    dungeon_delete_object, dungeon_lite_spot, dungeon_move_creature_record, dungeon_remove_monster_from_level,
    dungeon_summon_object,
};
use crate::dungeon_los::los;
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CAVE_WALL, TILE_BOUNDARY_WALL, TILE_CORR_FLOOR};
use crate::game::{game, random_number};
use crate::helpers::get_and_clear_first_bit;
use crate::inventory::inventory_item_copy_to;
use crate::player::{py, player_disturb, A_CON, A_DEX, A_INT, A_STR, A_WIS};
use crate::recall_data::creature_recall;
use crate::spells_data::MagicSpellFlags;
use crate::treasure::{TV_CLOSED_DOOR, TV_MAX_OBJECT, TV_NEVER, TV_FOOD, TV_SECRET_DOOR, TV_VIS_TRAP};
use crate::ui::{coord_inside_panel, print_character_current_mana, print_character_gold_value, print_character_winner, display_character_experience};
use crate::ui_io::{print_message, screen_has_changed};

fn monster_is_visible(monster: &Monster) -> bool {
    let mut visible = false;

    let tile = &dg().floor[monster.pos.y as usize][monster.pos.x as usize];
    let creature = &CREATURES_LIST[monster.creature_id as usize];

    if tile.permanent_light || tile.temporary_light || (py().running_tracker != 0 && monster.distance_from_player < 2 && py().carrying_light) {
        // Normal sight.
        if (creature.movement & config::monsters::move_flags::CM_INVISIBLE) == 0 {
            visible = true;
        } else if py().flags.see_invisible {
            visible = true;
            creature_recall()[monster.creature_id as usize].movement |= config::monsters::move_flags::CM_INVISIBLE;
        }
    } else if py().flags.see_infra > 0
        && monster.distance_from_player as i16 <= py().flags.see_infra
        && (creature.defenses & config::monsters::defense::CD_INFRA) != 0
    {
        // Infra vision.
        visible = true;
        creature_recall()[monster.creature_id as usize].defenses |= config::monsters::defense::CD_INFRA;
    }

    visible
}

// Updates screen when monsters move about -RAK-
pub fn monster_update_visibility(monster_id: i32) {
    let mut visible = false;
    let monster = monsters()[monster_id as usize];

    if monster.distance_from_player <= config::monsters::MON_MAX_SIGHT
        && (py().flags.status & config::player::status::PY_BLIND) == 0
        && coord_inside_panel(monster.pos)
    {
        if game().wizard_mode {
            // Wizard sight.
            visible = true;
        } else if los(py().pos, monster.pos) {
            visible = monster_is_visible(&monster);
        }
    }

    if visible {
        // Light it up.
        if !monster.lit {
            player_disturb(1, 0);
            monsters()[monster_id as usize].lit = true;
            dungeon_lite_spot(monster.pos);

            // notify inventory_execute_command()
            *screen_has_changed() = true;
        }
    } else if monster.lit {
        // Turn it off.
        monsters()[monster_id as usize].lit = false;
        dungeon_lite_spot(monster.pos);

        // notify inventory_execute_command()
        *screen_has_changed() = true;
    }
}

// Given speed, returns number of moves this turn. -RAK-
// NOTE: Player must always move at least once per iteration,
// a slowed player is handled by moving monsters faster
fn monster_movement_rate(speed: i16) -> i32 {
    if speed > 0 {
        if py().flags.rest != 0 {
            return 1;
        }
        return speed as i32;
    }

    // speed must be negative here
    if (dg().game_turn % (2 - speed as i32)) == 0 {
        1
    } else {
        0
    }
}

// Makes sure a new creature gets lit up. -CJS-
fn monster_make_visible(coord: Coord) -> bool {
    let monster_id = dg().floor[coord.y as usize][coord.x as usize].creature_id as i32;
    if monster_id <= 1 {
        return false;
    }

    monster_update_visibility(monster_id);
    monsters()[monster_id as usize].lit
}

// Choose correct directions for monster movement -RAK-
fn monster_get_move_direction(monster_id: i32, directions: &mut [i32; 9]) {
    let mut movement;
    let ay;
    let ax;

    let y = monsters()[monster_id as usize].pos.y - py().pos.y;
    let x = monsters()[monster_id as usize].pos.x - py().pos.x;

    if y < 0 {
        movement = 8;
        ay = -y;
    } else {
        movement = 0;
        ay = y;
    }
    if x > 0 {
        movement += 4;
        ax = x;
    } else {
        ax = -x;
    }

    // this has the advantage of preventing the diamond maneuver, also faster
    if ay > (ax << 1) {
        movement += 2;
    } else if ax > (ay << 1) {
        movement += 1;
    }

    match movement {
        0 => {
            directions[0] = 9;
            if ay > ax {
                directions[1] = 8;
                directions[2] = 6;
                directions[3] = 7;
                directions[4] = 3;
            } else {
                directions[1] = 6;
                directions[2] = 8;
                directions[3] = 3;
                directions[4] = 7;
            }
        }
        1 | 9 => {
            directions[0] = 6;
            if y < 0 {
                directions[1] = 3;
                directions[2] = 9;
                directions[3] = 2;
                directions[4] = 8;
            } else {
                directions[1] = 9;
                directions[2] = 3;
                directions[3] = 8;
                directions[4] = 2;
            }
        }
        2 | 6 => {
            directions[0] = 8;
            if x < 0 {
                directions[1] = 9;
                directions[2] = 7;
                directions[3] = 6;
                directions[4] = 4;
            } else {
                directions[1] = 7;
                directions[2] = 9;
                directions[3] = 4;
                directions[4] = 6;
            }
        }
        4 => {
            directions[0] = 7;
            if ay > ax {
                directions[1] = 8;
                directions[2] = 4;
                directions[3] = 9;
                directions[4] = 1;
            } else {
                directions[1] = 4;
                directions[2] = 8;
                directions[3] = 1;
                directions[4] = 9;
            }
        }
        5 | 13 => {
            directions[0] = 4;
            if y < 0 {
                directions[1] = 1;
                directions[2] = 7;
                directions[3] = 2;
                directions[4] = 8;
            } else {
                directions[1] = 7;
                directions[2] = 1;
                directions[3] = 8;
                directions[4] = 2;
            }
        }
        8 => {
            directions[0] = 3;
            if ay > ax {
                directions[1] = 2;
                directions[2] = 6;
                directions[3] = 1;
                directions[4] = 9;
            } else {
                directions[1] = 6;
                directions[2] = 2;
                directions[3] = 9;
                directions[4] = 1;
            }
        }
        10 | 14 => {
            directions[0] = 2;
            if x < 0 {
                directions[1] = 3;
                directions[2] = 1;
                directions[3] = 6;
                directions[4] = 4;
            } else {
                directions[1] = 1;
                directions[2] = 3;
                directions[3] = 4;
                directions[4] = 6;
            }
        }
        12 => {
            directions[0] = 1;
            if ay > ax {
                directions[1] = 2;
                directions[2] = 4;
                directions[3] = 3;
                directions[4] = 7;
            } else {
                directions[1] = 4;
                directions[2] = 2;
                directions[3] = 7;
                directions[4] = 3;
            }
        }
        _ => {}
    }
}

fn monster_print_attack_description(msg: &str, attack_id: i32) {
    match attack_id {
        1 => print_message(Some(&format!("{}hits you.", msg))),
        2 => print_message(Some(&format!("{}bites you.", msg))),
        3 => print_message(Some(&format!("{}claws you.", msg))),
        4 => print_message(Some(&format!("{}stings you.", msg))),
        5 => print_message(Some(&format!("{}touches you.", msg))),
        7 => print_message(Some(&format!("{}gazes at you.", msg))),
        8 => print_message(Some(&format!("{}breathes on you.", msg))),
        9 => print_message(Some(&format!("{}spits on you.", msg))),
        10 => print_message(Some(&format!("{}makes a horrible wail.", msg))),
        12 => print_message(Some(&format!("{}crawls on you.", msg))),
        13 => print_message(Some(&format!("{}releases a cloud of spores.", msg))),
        14 => print_message(Some(&format!("{}begs you for money.", msg))),
        15 => print_message(Some("You've been slimed!")),
        16 => print_message(Some(&format!("{}crushes you.", msg))),
        17 => print_message(Some(&format!("{}tramples you.", msg))),
        18 => print_message(Some(&format!("{}drools on you.", msg))),
        19 => match random_number(9) {
            1 => print_message(Some(&format!("{}insults you!", msg))),
            2 => print_message(Some(&format!("{}insults your mother!", msg))),
            3 => print_message(Some(&format!("{}gives you the finger!", msg))),
            4 => print_message(Some(&format!("{}humiliates you!", msg))),
            5 => print_message(Some(&format!("{}wets on your leg!", msg))),
            6 => print_message(Some(&format!("{}defiles you!", msg))),
            7 => print_message(Some(&format!("{}dances around you!", msg))),
            8 => print_message(Some(&format!("{}makes obscene gestures!", msg))),
            9 => print_message(Some(&format!("{}moons you!!!", msg))),
            _ => {}
        },
        99 => print_message(Some(&format!("{}is repelled.", msg))),
        _ => {}
    }
}

fn monster_confuse_on_attack(monster_id: i32, attack_type: i32, monster_name: &str, visible: bool) {
    if py().flags.confuse_monster && attack_type != 99 {
        print_message(Some("Your hands stop glowing."));
        py().flags.confuse_monster = false;

        let creature_id = monsters()[monster_id as usize].creature_id as usize;
        let creature_level = CREATURES_LIST[creature_id].level;
        let creature_defenses = CREATURES_LIST[creature_id].defenses;

        let msg;
        if random_number(MON_MAX_LEVELS as i32) < creature_level as i32 || (creature_defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
            msg = format!("{}is unaffected.", monster_name);
        } else {
            msg = format!("{}appears confused.", monster_name);
            if monsters()[monster_id as usize].confused_amount != 0 {
                monsters()[monster_id as usize].confused_amount += 3;
            } else {
                monsters()[monster_id as usize].confused_amount = (2 + random_number(16)) as u8;
            }
        }

        print_message(Some(&msg));

        if visible && !game().character_is_dead && random_number(4) == 1 {
            creature_recall()[creature_id].defenses |= creature_defenses & config::monsters::defense::CD_NO_SLEEP;
        }
    }
}

// Make an attack on the player (chuckle.) -RAK-
fn monster_attack_player(monster_id: i32) {
    // don't beat a dead body!
    if game().character_is_dead {
        return;
    }

    let creature_id = monsters()[monster_id as usize].creature_id as usize;

    let name = if !monsters()[monster_id as usize].lit {
        "It ".to_string()
    } else {
        format!("The {} ", CREATURES_LIST[creature_id].name)
    };

    let death_description = crate::player::player_died_from_string(CREATURES_LIST[creature_id].name, CREATURES_LIST[creature_id].movement);

    let mut attack_counter = 0;
    for i in 0..4 {
        let damage_type_id = CREATURES_LIST[creature_id].damage[i];

        if damage_type_id == 0 || game().character_is_dead {
            break;
        }

        let mut attack_type = MONSTER_ATTACKS[damage_type_id as usize].type_id as i32;
        let mut attack_desc = MONSTER_ATTACKS[damage_type_id as usize].description_id as i32;
        let dice = MONSTER_ATTACKS[damage_type_id as usize].dice;

        if py().flags.protect_evil > 0
            && (CREATURES_LIST[creature_id].defenses & config::monsters::defense::CD_EVIL) != 0
            && py().misc.level as i32 + 1 > CREATURES_LIST[creature_id].level as i32
        {
            if monsters()[monster_id as usize].lit {
                creature_recall()[creature_id].defenses |= config::monsters::defense::CD_EVIL;
            }
            attack_type = 99;
            attack_desc = 99;
        }

        if crate::player::player_test_attack_hits(attack_type, CREATURES_LIST[creature_id].level) {
            player_disturb(1, 0);

            // can not strcat to name because the creature may have multiple attacks.
            monster_print_attack_description(&name, attack_desc);

            // always fail to notice attack if creature invisible, set notice
            // and visible here since creature may be visible when attacking
            // and then teleport afterwards (becoming effectively invisible)
            let mut notice = true;
            let mut visible = true;
            if !monsters()[monster_id as usize].lit {
                visible = false;
                notice = false;
            }

            let damage = dice_roll(dice);
            let mut monster_hp = monsters()[monster_id as usize].hp;
            notice = execute_attack_on_player(CREATURES_LIST[creature_id].level, &mut monster_hp, monster_id, attack_type, damage, &death_description, notice);
            monsters()[monster_id as usize].hp = monster_hp;

            // Moved here from monster_move, so that monster only confused if it
            // actually hits. A monster that has been repelled has not hit
            // the player, so it should not be confused.
            monster_confuse_on_attack(monster_id, attack_desc, &name, visible);

            // increase number of attacks if notice true, or if visible and
            // had previously noticed the attack (in which case all this does
            // is help player learn damage), note that in the second case do
            // not increase attacks if creature repelled (no damage done)
            if (notice || (visible && creature_recall()[creature_id].attacks[attack_counter] != 0 && attack_type != 99))
                && creature_recall()[creature_id].attacks[attack_counter] < u8::MAX
            {
                creature_recall()[creature_id].attacks[attack_counter] += 1;
            }

            if game().character_is_dead && creature_recall()[creature_id].deaths < u16::MAX {
                creature_recall()[creature_id].deaths += 1;
            }
        } else if (attack_desc >= 1 && attack_desc <= 3) || attack_desc == 6 {
            player_disturb(1, 0);

            print_message(Some(&format!("{}misses you.", name)));
        }

        if attack_counter < MON_MAX_ATTACKS - 1 {
            attack_counter += 1;
        } else {
            break;
        }
    }
}

fn monster_open_door(monster_hp: i16, move_bits: u32, do_turn: &mut bool, do_move: &mut bool, rcmove: &mut u32, coord: Coord) {
    let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;

    // Creature can open doors.
    if (move_bits & config::monsters::move_flags::CM_OPEN_DOOR) != 0 {
        let mut door_is_stuck = false;

        let category_id = game().treasure.list[treasure_id].category_id;
        let misc_use = game().treasure.list[treasure_id].misc_use;

        if category_id == TV_CLOSED_DOOR {
            *do_turn = true;

            if misc_use == 0 {
                // Closed doors
                *do_move = true;
            } else if misc_use > 0 {
                // Locked doors
                if random_number((monster_hp as i32 + 1) * (50 + misc_use as i32)) < 40 * (monster_hp as i32 - 10 - misc_use as i32) {
                    game().treasure.list[treasure_id].misc_use = 0;
                }
            } else {
                // Stuck doors
                if random_number((monster_hp as i32 + 1) * (50 - misc_use as i32)) < 40 * (monster_hp as i32 - 10 + misc_use as i32) {
                    print_message(Some("You hear a door burst open!"));
                    player_disturb(1, 0);
                    door_is_stuck = true;
                    *do_move = true;
                }
            }
        } else if category_id == TV_SECRET_DOOR {
            *do_turn = true;
            *do_move = true;
        }

        if *do_move {
            inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[treasure_id]);

            // 50% chance of breaking door
            if door_is_stuck {
                game().treasure.list[treasure_id].misc_use = (1 - random_number(2)) as i16;
            }
            dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
            dungeon_lite_spot(coord);
            *rcmove |= config::monsters::move_flags::CM_OPEN_DOOR;
            *do_move = false;
        }
    } else if game().treasure.list[treasure_id].category_id == TV_CLOSED_DOOR {
        // Creature can not open doors, must bash them
        *do_turn = true;

        let abs_misc_use = game().treasure.list[treasure_id].misc_use.abs() as i32;
        if random_number((monster_hp as i32 + 1) * (80 + abs_misc_use)) < 40 * (monster_hp as i32 - 20 - abs_misc_use) {
            inventory_item_copy_to(config::dungeon::objects::OBJ_OPEN_DOOR as usize, &mut game().treasure.list[treasure_id]);

            // 50% chance of breaking door
            game().treasure.list[treasure_id].misc_use = (1 - random_number(2)) as i16;
            dg().floor[coord.y as usize][coord.x as usize].feature_id = TILE_CORR_FLOOR;
            dungeon_lite_spot(coord);
            print_message(Some("You hear a door burst open!"));
            player_disturb(1, 0);
        }
    }
}

fn glyph_of_warding_protection(creature_id: u16, move_bits: u32, do_move: &mut bool, do_turn: &mut bool, coord: Coord) {
    if random_number(config::treasure::OBJECTS_RUNE_PROTECTION as i32) < CREATURES_LIST[creature_id as usize].level as i32 {
        if coord.y == py().pos.y && coord.x == py().pos.x {
            print_message(Some("The rune of protection is broken!"));
        }
        dungeon_delete_object(coord);
        return;
    }

    *do_move = false;

    // If the creature moves only to attack, don't let it
    // move if the glyph prevents it from attacking
    if (move_bits & config::monsters::move_flags::CM_ATTACK_ONLY) != 0 {
        *do_turn = true;
    }
}

fn monster_moves_on_player(monster_id: i32, creature_id: u8, move_bits: u32, do_move: &mut bool, do_turn: &mut bool, rcmove: &mut u32, coord: Coord) {
    let monster = monsters()[monster_id as usize];

    if creature_id == 1 {
        // if the monster is not lit, must call monster_update_visibility, it
        // may be faster than character, and hence could have
        // just moved next to character this same turn.
        if !monster.lit {
            monster_update_visibility(monster_id);
        }
        monster_attack_player(monster_id);
        *do_move = false;
        *do_turn = true;
    } else if creature_id > 1 && (coord.y != monster.pos.y || coord.x != monster.pos.x) {
        // Creature is attempting to move on other creature?

        // Creature eats other creatures?
        if (move_bits & config::monsters::move_flags::CM_EATS_OTHER) != 0
            && CREATURES_LIST[monster.creature_id as usize].kill_exp_value >= CREATURES_LIST[monsters()[creature_id as usize].creature_id as usize].kill_exp_value
        {
            if monsters()[creature_id as usize].lit {
                *rcmove |= config::monsters::move_flags::CM_EATS_OTHER;
            }

            // It ate an already processed monster. Handle normally.
            if monster_id < creature_id as i32 {
                dungeon_delete_monster(creature_id as i32);
            } else {
                // If it eats this monster, an already processed
                // monster will take its place, causing all kinds
                // of havoc. Delay the kill a bit.
                dungeon_remove_monster_from_level(creature_id as i32);
            }
        } else {
            *do_move = false;
        }
    }
}

fn monster_allowed_to_move(monster_id: i32, move_bits: u32, do_turn: &mut bool, rcmove: &mut u32, coord: Coord) {
    // Pick up or eat an object
    if (move_bits & config::monsters::move_flags::CM_PICKS_UP) != 0 {
        let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id;

        if treasure_id != 0 && game().treasure.list[treasure_id as usize].category_id <= TV_MAX_OBJECT {
            *rcmove |= config::monsters::move_flags::CM_PICKS_UP;
            dungeon_delete_object(coord);
        }
    }

    // Move creature record
    let old_pos = monsters()[monster_id as usize].pos;
    dungeon_move_creature_record(old_pos, coord);

    if monsters()[monster_id as usize].lit {
        monsters()[monster_id as usize].lit = false;
        dungeon_lite_spot(old_pos);
    }

    monsters()[monster_id as usize].pos = coord;
    monsters()[monster_id as usize].distance_from_player = coord_distance_between(py().pos, coord) as u8;

    *do_turn = true;
}

// Make the move if possible, five choices -RAK-
fn make_move(monster_id: i32, directions: &[i32; 9], rcmove: &mut u32) {
    let mut do_turn = false;
    let mut do_move = false;

    let move_bits = CREATURES_LIST[monsters()[monster_id as usize].creature_id as usize].movement;

    // Up to 5 attempts at moving, give up.
    let mut i = 0;
    while !do_turn && i < 5 {
        // Get new position
        let mut coord = monsters()[monster_id as usize].pos;

        crate::player::player_move_position(directions[i], &mut coord);

        let feature_id = dg().floor[coord.y as usize][coord.x as usize].feature_id;

        if feature_id == TILE_BOUNDARY_WALL {
            i += 1;
            continue;
        }

        // Floor is open?
        if feature_id <= MAX_OPEN_SPACE {
            do_move = true;
        } else if (move_bits & config::monsters::move_flags::CM_PHASE) != 0 {
            // Creature moves through walls?
            do_move = true;
            *rcmove |= config::monsters::move_flags::CM_PHASE;
        } else if dg().floor[coord.y as usize][coord.x as usize].treasure_id != 0 {
            // Creature can open doors?
            let monster_hp = monsters()[monster_id as usize].hp;
            monster_open_door(monster_hp, move_bits, &mut do_turn, &mut do_move, rcmove, coord);
        }

        // Glyph of warding present?
        let treasure_id = dg().floor[coord.y as usize][coord.x as usize].treasure_id as usize;
        if do_move
            && treasure_id != 0
            && game().treasure.list[treasure_id].category_id == TV_VIS_TRAP
            && game().treasure.list[treasure_id].sub_category_id == 99
        {
            let creature_id = monsters()[monster_id as usize].creature_id;
            glyph_of_warding_protection(creature_id, move_bits, &mut do_move, &mut do_turn, coord);
        }

        // Creature has attempted to move on player?
        if do_move {
            let creature_id = dg().floor[coord.y as usize][coord.x as usize].creature_id;
            monster_moves_on_player(monster_id, creature_id, move_bits, &mut do_move, &mut do_turn, rcmove, coord);
        }

        // Creature has been allowed move.
        if do_move {
            monster_allowed_to_move(monster_id, move_bits, &mut do_turn, rcmove, coord);
        }

        i += 1;
    }
}

fn monster_can_cast_spells(monster: &Monster, spells: u32) -> bool {
    // 1 in x chance of casting spell
    if random_number((spells & config::monsters::spells::CS_FREQ) as i32) != 1 {
        return false;
    }

    // Must be within certain range
    let within_range = monster.distance_from_player <= config::monsters::MON_MAX_SPELL_CAST_DISTANCE;

    // Must have unobstructed Line-Of-Sight
    let unobstructed = los(py().pos, monster.pos);

    within_range && unobstructed
}

pub fn monster_execute_casting_of_spell(monster_id: i32, spell_id: i32, level: u8, monster_name: &str, death_description: &str) {
    // Cast the spell.
    match spell_id {
        5 => {
            // Teleport Short
            crate::spells::spell_teleport_away_monster(monster_id, 5);
        }
        6 => {
            // Teleport Long
            crate::spells::spell_teleport_away_monster(monster_id, config::monsters::MON_MAX_SIGHT as i32);
        }
        7 => {
            // Teleport To
            crate::spells::spell_teleport_player_to(monsters()[monster_id as usize].pos);
        }
        8 => {
            // Light Wound
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else {
                crate::player::player_takes_hit(dice_roll(Dice::new(3, 8)), death_description);
            }
        }
        9 => {
            // Serious Wound
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else {
                crate::player::player_takes_hit(dice_roll(Dice::new(8, 8)), death_description);
            }
        }
        10 => {
            // Hold Person
            if py().flags.free_action {
                print_message(Some("You are unaffected."));
            } else if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else if py().flags.paralysis > 0 {
                py().flags.paralysis += 2;
            } else {
                py().flags.paralysis = (random_number(5) + 4) as i16;
            }
        }
        11 => {
            // Cause Blindness
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else if py().flags.blind > 0 {
                py().flags.blind += 6;
            } else {
                py().flags.blind += (12 + random_number(3)) as i16;
            }
        }
        12 => {
            // Cause Confuse
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else if py().flags.confused > 0 {
                py().flags.confused += 2;
            } else {
                py().flags.confused = (random_number(5) + 3) as i16;
            }
        }
        13 => {
            // Cause Fear
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else if py().flags.afraid > 0 {
                py().flags.afraid += 2;
            } else {
                py().flags.afraid = (random_number(5) + 3) as i16;
            }
        }
        14 => {
            // Summon Monster
            print_message(Some(&format!("{}magically summons a monster!", monster_name)));
            let mut coord = py().pos;

            // in case compact_monster() is called,it needs monster_id
            *hack_monptr() = monster_id;
            crate::monster_manager::monster_summon(&mut coord, false);
            *hack_monptr() = -1;
            monster_update_visibility(dg().floor[coord.y as usize][coord.x as usize].creature_id as i32);
        }
        15 => {
            // Summon Undead
            print_message(Some(&format!("{}magically summons an undead!", monster_name)));
            let mut coord = py().pos;

            // in case compact_monster() is called,it needs monster_id
            *hack_monptr() = monster_id;
            crate::monster_manager::monster_summon_undead(&mut coord);
            *hack_monptr() = -1;
            monster_update_visibility(dg().floor[coord.y as usize][coord.x as usize].creature_id as i32);
        }
        16 => {
            // Slow Person
            if py().flags.free_action {
                print_message(Some("You are unaffected."));
            } else if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects of the spell."));
            } else if py().flags.slow > 0 {
                py().flags.slow += 2;
            } else {
                py().flags.slow = (random_number(5) + 3) as i16;
            }
        }
        17 => {
            // Drain Mana
            if py().misc.current_mana > 0 {
                player_disturb(1, 0);

                print_message(Some(&format!("{}draws psychic energy from you!", monster_name)));

                if monsters()[monster_id as usize].lit {
                    print_message(Some(&format!("{}appears healthier.", monster_name)));
                }

                let mut num = (random_number(level as i32) >> 1) + 1;
                if num as i16 > py().misc.current_mana {
                    num = py().misc.current_mana as i32;
                    py().misc.current_mana = 0;
                    py().misc.current_mana_fraction = 0;
                } else {
                    py().misc.current_mana -= num as i16;
                }
                print_character_current_mana();
                monsters()[monster_id as usize].hp += 6 * num as i16;
            }
        }
        20 => {
            // Breath Light
            print_message(Some(&format!("{}breathes lightning.", monster_name)));
            let hp = monsters()[monster_id as usize].hp;
            crate::spells::spell_breath(py().pos, monster_id, hp as i32 / 4, MagicSpellFlags::Lightning as i32, death_description);
        }
        21 => {
            // Breath Gas
            print_message(Some(&format!("{}breathes gas.", monster_name)));
            let hp = monsters()[monster_id as usize].hp;
            crate::spells::spell_breath(py().pos, monster_id, hp as i32 / 3, MagicSpellFlags::PoisonGas as i32, death_description);
        }
        22 => {
            // Breath Acid
            print_message(Some(&format!("{}breathes acid.", monster_name)));
            let hp = monsters()[monster_id as usize].hp;
            crate::spells::spell_breath(py().pos, monster_id, hp as i32 / 3, MagicSpellFlags::Acid as i32, death_description);
        }
        23 => {
            // Breath Frost
            print_message(Some(&format!("{}breathes frost.", monster_name)));
            let hp = monsters()[monster_id as usize].hp;
            crate::spells::spell_breath(py().pos, monster_id, hp as i32 / 3, MagicSpellFlags::Frost as i32, death_description);
        }
        24 => {
            // Breath Fire
            print_message(Some(&format!("{}breathes fire.", monster_name)));
            let hp = monsters()[monster_id as usize].hp;
            crate::spells::spell_breath(py().pos, monster_id, hp as i32 / 3, MagicSpellFlags::Fire as i32, death_description);
        }
        _ => {
            print_message(Some(&format!("{}cast unknown spell.", monster_name)));
        }
    }
}

// Creatures can cast spells too.  (Dragon Breath) -RAK-
//   cast_spell_get_id = true if creature changes position
//   return true (took_turn) if creature casts a spell
fn monster_cast_spell(monster_id: i32) -> bool {
    if game().character_is_dead {
        return false;
    }

    let monster = monsters()[monster_id as usize];
    let creature_id = monster.creature_id as usize;
    let creature_spells = CREATURES_LIST[creature_id].spells;
    let creature_level = CREATURES_LIST[creature_id].level;

    if !monster_can_cast_spells(&monster, creature_spells) {
        return false;
    }

    // Creature is going to cast a spell

    // Check to see if monster should be lit.
    monster_update_visibility(monster_id);

    // Describe the attack
    let mut name = if monsters()[monster_id as usize].lit {
        format!("The {} ", CREATURES_LIST[creature_id].name)
    } else {
        "It ".to_string()
    };

    let death_description = crate::player::player_died_from_string(CREATURES_LIST[creature_id].name, CREATURES_LIST[creature_id].movement);

    // Extract all possible spells into spell_choice
    let mut spell_choice = [0i32; 30];
    let mut spell_flags = creature_spells & !config::monsters::spells::CS_FREQ;

    let mut id = 0;
    while spell_flags != 0 {
        spell_choice[id] = get_and_clear_first_bit(&mut spell_flags);
        id += 1;
    }

    // Choose a spell to cast
    let mut thrown_spell = spell_choice[(random_number(id as i32) - 1) as usize];
    thrown_spell += 1;

    // all except spell_teleport_away_monster() and drain mana spells always disturb
    if thrown_spell > 6 && thrown_spell != 17 {
        player_disturb(1, 0);
    }

    // save some code/data space here, with a small time penalty
    if (thrown_spell < 14 && thrown_spell > 6) || thrown_spell == 16 {
        name.push_str("casts a spell.");
        print_message(Some(&name));
        // strip the appended text again for the actual spell message
        name.truncate(name.len() - "casts a spell.".len());
    }

    monster_execute_casting_of_spell(monster_id, thrown_spell, creature_level, &name, &death_description);

    if monsters()[monster_id as usize].lit {
        creature_recall()[creature_id].spells |= 1 << (thrown_spell - 1);
        if (creature_recall()[creature_id].spells & config::monsters::spells::CS_FREQ) != config::monsters::spells::CS_FREQ {
            creature_recall()[creature_id].spells += 1;
        }
        if game().character_is_dead && creature_recall()[creature_id].deaths < u16::MAX {
            creature_recall()[creature_id].deaths += 1;
        }
    }

    true
}

// Places creature adjacent to given location -RAK-
// Rats and Flys are fun!
pub fn monster_multiply(coord: Coord, creature_id: i32, monster_id: i32) -> bool {
    for _ in 0..=18 {
        let position = Coord::new(coord.y - 2 + random_number(3), coord.x - 2 + random_number(3));

        // don't create a new creature on top of the old one, that
        // causes invincible/invisible creatures to appear.
        if coord_in_bounds(position) && (position.y != coord.y || position.x != coord.x) {
            let tile = dg().floor[position.y as usize][position.x as usize];

            if tile.feature_id <= MAX_OPEN_SPACE && tile.treasure_id == 0 && tile.creature_id != 1 {
                // Creature there already?
                if tile.creature_id > 1 {
                    // Some critters are cannibalistic!
                    let cannibalistic = (CREATURES_LIST[creature_id as usize].movement & config::monsters::move_flags::CM_EATS_OTHER) != 0;

                    // Check the experience level -CJS-
                    let experienced = CREATURES_LIST[creature_id as usize].kill_exp_value
                        >= CREATURES_LIST[monsters()[tile.creature_id as usize].creature_id as usize].kill_exp_value;

                    if cannibalistic && experienced {
                        // It ate an already processed monster. Handle * normally.
                        if monster_id < tile.creature_id as i32 {
                            dungeon_delete_monster(tile.creature_id as i32);
                        } else {
                            // If it eats this monster, an already processed
                            // monster will take its place, causing all kinds
                            // of havoc. Delay the kill a bit.
                            dungeon_remove_monster_from_level(tile.creature_id as i32);
                        }

                        // in case compact_monster() is called, it needs monster_id.
                        *hack_monptr() = monster_id;
                        // Place_monster() may fail if monster list full.
                        let result = crate::monster_manager::monster_place_new(position, creature_id, false);
                        *hack_monptr() = -1;
                        if !result {
                            return false;
                        }

                        *monster_multiply_total() += 1;
                        return monster_make_visible(position);
                    }
                } else {
                    // All clear,  place a monster

                    // in case compact_monster() is called,it needs monster_id
                    *hack_monptr() = monster_id;
                    // Place_monster() may fail if monster list full.
                    let result = crate::monster_manager::monster_place_new(position, creature_id, false);
                    *hack_monptr() = -1;
                    if !result {
                        return false;
                    }

                    *monster_multiply_total() += 1;
                    return monster_make_visible(position);
                }
            }
        }
    }

    false
}

fn monster_multiply_critter(monster_id: i32, rcmove: &mut u32) {
    let monster = monsters()[monster_id as usize];

    let mut counter = 0;

    for y in (monster.pos.y - 1)..=(monster.pos.y + 1) {
        for x in (monster.pos.x - 1)..=(monster.pos.x + 1) {
            let coord = Coord::new(y, x);
            if coord_in_bounds(coord) && dg().floor[y as usize][x as usize].creature_id > 1 {
                counter += 1;
            }
        }
    }

    // can't call random_number with a value of zero, increment
    // counter to allow creature multiplication.
    if counter == 0 {
        counter += 1;
    }

    if counter < 4 && random_number(counter * config::monsters::MON_MULTIPLY_ADJUST as i32) == 1 {
        if monster_multiply(monster.pos, monster.creature_id as i32, monster_id) {
            *rcmove |= config::monsters::move_flags::CM_MULTIPLY;
        }
    }
}

fn monster_move_out_of_wall(monster_id: i32, rcmove: &mut u32) {
    let monster = monsters()[monster_id as usize];

    // If the monster is already dead, don't kill it again!
    // This can happen for monsters moving faster than the player. They
    // will get multiple moves, but should not if they die on the first
    // move.  This is only a problem for monsters stuck in rock.
    if monster.hp < 0 {
        return;
    }

    let mut id = 0;
    let mut dir = 1;
    let mut directions = [0i32; 9];

    // Note direction of for loops matches direction of keypad from 1 to 9
    // Do not allow attack against the player.
    let mut y = monster.pos.y + 1;
    while y >= monster.pos.y - 1 {
        for x in (monster.pos.x - 1)..=(monster.pos.x + 1) {
            if dir != 5 && dg().floor[y as usize][x as usize].feature_id <= MAX_OPEN_SPACE && dg().floor[y as usize][x as usize].creature_id != 1 {
                directions[id] = dir;
                id += 1;
            }
            dir += 1;
        }
        y -= 1;
    }

    if id != 0 {
        // put a random direction first
        let dir = (random_number(id as i32) - 1) as usize;

        directions.swap(0, dir);

        // this can only fail if directions[0] has a rune of protection
        make_move(monster_id, &directions, rcmove);
    }

    // if still in a wall, let it dig itself out, but also apply some more damage
    let pos = monsters()[monster_id as usize].pos;
    if dg().floor[pos.y as usize][pos.x as usize].feature_id >= MIN_CAVE_WALL {
        // in case the monster dies, may need to call fix1_delete_monster()
        // instead of delete_monsters()
        *hack_monptr() = monster_id;
        let i = monster_take_hit(monster_id, dice_roll(Dice::new(8, 8)));
        *hack_monptr() = -1;

        if i >= 0 {
            print_message(Some("You hear a scream muffled by rock!"));
            display_character_experience();
        } else {
            print_message(Some("A creature digs itself out from the rock!"));
            crate::player::player_tunnel_wall(pos, 1, 0);
        }
    }
}

// Undead only get confused from turn undead, so they should flee
fn monster_move_undead(monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];
    monster_get_move_direction(monster_id, &mut directions);

    directions[0] = 10 - directions[0];
    directions[1] = 10 - directions[1];
    directions[2] = 10 - directions[2];
    directions[3] = random_number(9); // May attack only if cornered
    directions[4] = random_number(9);

    // don't move if it's is not supposed to move!
    let movement = CREATURES_LIST[monsters()[monster_id as usize].creature_id as usize].movement;
    if (movement & config::monsters::move_flags::CM_ATTACK_ONLY) == 0 {
        make_move(monster_id, &directions, rcmove);
    }
}

fn monster_move_confused(monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];

    directions[0] = random_number(9);
    directions[1] = random_number(9);
    directions[2] = random_number(9);
    directions[3] = random_number(9);
    directions[4] = random_number(9);

    // don't move if it's is not supposed to move!
    let movement = CREATURES_LIST[monsters()[monster_id as usize].creature_id as usize].movement;
    if (movement & config::monsters::move_flags::CM_ATTACK_ONLY) == 0 {
        make_move(monster_id, &directions, rcmove);
    }
}

fn monster_do_move(monster_id: i32, rcmove: &mut u32) -> bool {
    let creature_id = monsters()[monster_id as usize].creature_id as usize;

    // Creature is confused or undead turned?
    if monsters()[monster_id as usize].confused_amount != 0 {
        if (CREATURES_LIST[creature_id].defenses & config::monsters::defense::CD_UNDEAD) != 0 {
            monster_move_undead(monster_id, rcmove);
        } else {
            monster_move_confused(monster_id, rcmove);
        }
        monsters()[monster_id as usize].confused_amount -= 1;
        return true;
    }

    // Creature may cast a spell
    if (CREATURES_LIST[creature_id].spells & config::monsters::spells::CS_FREQ) != 0 {
        return monster_cast_spell(monster_id);
    }

    false
}

fn monster_move_randomly(monster_id: i32, rcmove: &mut u32, randomness: u32) {
    let mut directions = [0i32; 9];

    directions[0] = random_number(9);
    directions[1] = random_number(9);
    directions[2] = random_number(9);
    directions[3] = random_number(9);
    directions[4] = random_number(9);

    *rcmove |= randomness;

    make_move(monster_id, &directions, rcmove);
}

fn monster_move_normally(monster_id: i32, rcmove: &mut u32) {
    let mut directions = [0i32; 9];

    if random_number(200) == 1 {
        directions[0] = random_number(9);
        directions[1] = random_number(9);
        directions[2] = random_number(9);
        directions[3] = random_number(9);
        directions[4] = random_number(9);
    } else {
        monster_get_move_direction(monster_id, &mut directions);
    }

    *rcmove |= config::monsters::move_flags::CM_MOVE_NORMAL;

    make_move(monster_id, &directions, rcmove);
}

fn monster_attack_without_moving(monster_id: i32, rcmove: &mut u32, distance_from_player: u8) {
    let mut directions = [0i32; 9];

    if distance_from_player < 2 {
        monster_get_move_direction(monster_id, &mut directions);
        make_move(monster_id, &directions, rcmove);
    } else {
        // Learn that the monster does does not move when
        // it should have moved, but didn't.
        *rcmove |= config::monsters::move_flags::CM_ATTACK_ONLY;
    }
}

// Move the critters about the dungeon -RAK-
fn monster_move(monster_id: i32, rcmove: &mut u32) {
    let creature_id = monsters()[monster_id as usize].creature_id as usize;
    let movement = CREATURES_LIST[creature_id].movement;

    // Does the critter multiply?
    // rest could be negative, to be safe, only use mod with positive values.
    let abs_rest_period = (py().flags.rest as i32).abs();
    if (movement & config::monsters::move_flags::CM_MULTIPLY) != 0
        && config::monsters::MON_MAX_MULTIPLY_PER_LEVEL as i16 >= *monster_multiply_total()
        && (abs_rest_period % config::monsters::MON_MULTIPLY_ADJUST as i32) == 0
    {
        monster_multiply_critter(monster_id, rcmove);
    }

    // if in wall, must immediately escape to a clear area
    // then monster movement finished
    let pos = monsters()[monster_id as usize].pos;
    if (movement & config::monsters::move_flags::CM_PHASE) == 0 && dg().floor[pos.y as usize][pos.x as usize].feature_id >= MIN_CAVE_WALL {
        monster_move_out_of_wall(monster_id, rcmove);
        return;
    }

    if monster_do_move(monster_id, rcmove) {
        return;
    }

    // 75% random movement
    if (movement & config::monsters::move_flags::CM_75_RANDOM) != 0 && random_number(100) < 75 {
        monster_move_randomly(monster_id, rcmove, config::monsters::move_flags::CM_75_RANDOM);
        return;
    }

    // 40% random movement
    if (movement & config::monsters::move_flags::CM_40_RANDOM) != 0 && random_number(100) < 40 {
        monster_move_randomly(monster_id, rcmove, config::monsters::move_flags::CM_40_RANDOM);
        return;
    }

    // 20% random movement
    if (movement & config::monsters::move_flags::CM_20_RANDOM) != 0 && random_number(100) < 20 {
        monster_move_randomly(monster_id, rcmove, config::monsters::move_flags::CM_20_RANDOM);
        return;
    }

    // Normal movement
    if (movement & config::monsters::move_flags::CM_MOVE_NORMAL) != 0 {
        monster_move_normally(monster_id, rcmove);
        return;
    }

    // Attack, but don't move
    if (movement & config::monsters::move_flags::CM_ATTACK_ONLY) != 0 {
        let distance = monsters()[monster_id as usize].distance_from_player;
        monster_attack_without_moving(monster_id, rcmove, distance);
        return;
    }

    if (movement & config::monsters::move_flags::CM_ONLY_MAGIC) != 0 && monsters()[monster_id as usize].distance_from_player < 2 {
        // A little hack for Quylthulgs, so that one will eventually
        // notice that they have no physical attacks.
        if creature_recall()[creature_id].attacks[0] < u8::MAX {
            creature_recall()[creature_id].attacks[0] += 1;
        }

        // Another little hack for Quylthulgs, so that one can
        // eventually learn their speed.
        if creature_recall()[creature_id].attacks[0] > 20 {
            creature_recall()[creature_id].movement |= config::monsters::move_flags::CM_ONLY_MAGIC;
        }
    }
}

fn memory_update_recall(monster_id: i32, wake: bool, ignore: bool, rcmove: u32) {
    let monster = &monsters()[monster_id as usize];

    if !monster.lit {
        return;
    }

    let memory = &mut creature_recall()[monster.creature_id as usize];

    if wake {
        if memory.wake < u8::MAX {
            memory.wake += 1;
        }
    } else if ignore && memory.ignore < u8::MAX {
        memory.ignore += 1;
    }

    memory.movement |= rcmove;
}

fn monster_attacking_update(monster_id: i32, moves: i32) {
    for _ in 0..moves {
        let mut wake = false;
        let mut ignore = false;

        let mut rcmove: u32 = 0;

        // Monsters trapped in rock must be given a turn also,
        // so that they will die/dig out immediately.
        let monster = monsters()[monster_id as usize];
        let creature_id = monster.creature_id as usize;

        if monster.lit
            || monster.distance_from_player <= CREATURES_LIST[creature_id].area_affect_radius
            || ((CREATURES_LIST[creature_id].movement & config::monsters::move_flags::CM_PHASE) == 0
                && dg().floor[monster.pos.y as usize][monster.pos.x as usize].feature_id >= MIN_CAVE_WALL)
        {
            if monsters()[monster_id as usize].sleep_count > 0 {
                if py().flags.aggravate {
                    monsters()[monster_id as usize].sleep_count = 0;
                } else if (py().flags.rest == 0 && py().flags.paralysis < 1) || random_number(50) == 1 {
                    let notice = random_number(1024);

                    if (notice as i64 * notice as i64 * notice as i64) <= (1i64 << (29 - py().misc.stealth_factor as i64)) {
                        let distance = monsters()[monster_id as usize].distance_from_player as i16;
                        monsters()[monster_id as usize].sleep_count -= 100 / distance;
                        if monsters()[monster_id as usize].sleep_count > 0 {
                            ignore = true;
                        } else {
                            wake = true;

                            // force it to be exactly zero
                            monsters()[monster_id as usize].sleep_count = 0;
                        }
                    }
                }
            }

            if monsters()[monster_id as usize].stunned_amount != 0 {
                // NOTE: Balrog = 100*100 = 10000, it always recovers instantly
                if random_number(5000) < (CREATURES_LIST[creature_id].level as i32) * (CREATURES_LIST[creature_id].level as i32) {
                    monsters()[monster_id as usize].stunned_amount = 0;
                } else {
                    monsters()[monster_id as usize].stunned_amount -= 1;
                }

                if monsters()[monster_id as usize].stunned_amount == 0 {
                    if monsters()[monster_id as usize].lit {
                        let msg = format!("The {} recovers and glares at you.", CREATURES_LIST[creature_id].name);
                        print_message(Some(&msg));
                    }
                }
            }
            if monsters()[monster_id as usize].sleep_count == 0 && monsters()[monster_id as usize].stunned_amount == 0 {
                monster_move(monster_id, &mut rcmove);
            }
        }

        monster_update_visibility(monster_id);
        memory_update_recall(monster_id, wake, ignore, rcmove);
    }
}

// Creatures movement and attacking are done from here -RAK-
pub fn update_monsters(attack: bool) {
    // Process the monsters
    let mut id = *next_free_monster_id() as i32 - 1;
    while id >= config::monsters::MON_MIN_INDEX_ID as i32 && !game().character_is_dead {
        // Get rid of an eaten/breathed on monster.  Note: Be sure not to
        // process this monster. This is necessary because we can't delete
        // monsters while scanning the monsters here.
        if monsters()[id as usize].hp < 0 {
            dungeon_delete_monster_record(id);
            id -= 1;
            continue;
        }

        let pos = monsters()[id as usize].pos;
        monsters()[id as usize].distance_from_player = coord_distance_between(py().pos, pos) as u8;

        // Attack is argument passed to CREATURE
        if attack {
            let moves = monster_movement_rate(monsters()[id as usize].speed);

            if moves <= 0 {
                monster_update_visibility(id);
            } else {
                monster_attacking_update(id, moves);
            }
        } else {
            monster_update_visibility(id);
        }

        // Get rid of an eaten/breathed on monster. This is necessary because
        // we can't delete monsters while scanning the monsters here.
        // This monster may have been killed during monster_move().
        if monsters()[id as usize].hp < 0 {
            dungeon_delete_monster_record(id);
        }

        id -= 1;
    }
}

// Decreases monsters hit points and deletes monster if needed.
// (Picking on my babies.) -RAK-
pub fn monster_take_hit(monster_id: i32, damage: i32) -> i32 {
    let creature_id;
    {
        let monster = &mut monsters()[monster_id as usize];
        monster.sleep_count = 0;
        monster.hp -= damage as i16;

        if monster.hp >= 0 {
            return -1;
        }

        creature_id = monster.creature_id as usize;
    }

    let monster = monsters()[monster_id as usize];
    let creature_movement = CREATURES_LIST[creature_id].movement;

    let mut treasure_flags = monster_death(monster.pos, creature_movement);

    if (py().flags.blind < 1 && monster.lit) || (creature_movement & config::monsters::move_flags::CM_WIN) != 0 {
        let memory = &mut creature_recall()[creature_id];

        let tmp = (memory.movement & config::monsters::move_flags::CM_TREASURE) >> config::monsters::move_flags::CM_TR_SHIFT;

        if tmp > ((treasure_flags & config::monsters::move_flags::CM_TREASURE) >> config::monsters::move_flags::CM_TR_SHIFT) {
            treasure_flags = (treasure_flags & !config::monsters::move_flags::CM_TREASURE) | (tmp << config::monsters::move_flags::CM_TR_SHIFT);
        }

        memory.movement = (memory.movement & !config::monsters::move_flags::CM_TREASURE) | treasure_flags;

        if memory.kills < u16::MAX {
            memory.kills += 1;
        }
    }

    crate::player::player_gain_kill_experience(creature_id);

    // can't call display_character_experience() here, as that would result in "new level"
    // message appearing before "monster dies" message.
    let m_take_hit = monster.creature_id as i32;

    // in case this is called from within update_monsters(), this is a horrible
    // hack, the monsters/update_monsters() code needs to be rewritten.
    if *hack_monptr() < monster_id {
        dungeon_delete_monster(monster_id);
    } else {
        dungeon_remove_monster_from_level(monster_id);
    }

    m_take_hit
}

fn monster_death_item_drop_type(flags: u32) -> i32 {
    let mut object = if (flags & config::monsters::move_flags::CM_CARRY_OBJ) != 0 { 1 } else { 0 };

    if (flags & config::monsters::move_flags::CM_CARRY_GOLD) != 0 {
        object += 2;
    }

    if (flags & config::monsters::move_flags::CM_SMALL_OBJ) != 0 {
        object += 4;
    }

    object
}

fn monster_death_item_drop_count(flags: u32) -> i32 {
    let mut count = 0;

    if (flags & config::monsters::move_flags::CM_60_RANDOM) != 0 && random_number(100) < 60 {
        count += 1;
    }

    if (flags & config::monsters::move_flags::CM_90_RANDOM) != 0 && random_number(100) < 90 {
        count += 1;
    }

    if (flags & config::monsters::move_flags::CM_1D2_OBJ) != 0 {
        count += random_number(2);
    }

    if (flags & config::monsters::move_flags::CM_2D2_OBJ) != 0 {
        count += dice_roll(Dice::new(2, 2));
    }

    if (flags & config::monsters::move_flags::CM_4D2_OBJ) != 0 {
        count += dice_roll(Dice::new(4, 2));
    }

    count
}

// Allocates objects upon a creatures death -RAK-
// Oh well,  another creature bites the dust. Reward the
// victor based on flags set in the main creature record.
//
// Returns a mask of bits from the given flags which indicates what the
// monster is seen to have dropped.  This may be added to monster memory.
pub fn monster_death(coord: Coord, flags: u32) -> u32 {
    let item_type = monster_death_item_drop_type(flags);
    let item_count = monster_death_item_drop_count(flags);

    let mut dropped_item_id: u32 = 0;

    if item_count > 0 {
        dropped_item_id = dungeon_summon_object(coord, item_count, item_type) as u32;
    }

    // maybe the player died in mid-turn
    if (flags & config::monsters::move_flags::CM_WIN) != 0 && !game().character_is_dead {
        game().total_winner = true;

        print_character_winner();

        print_message(Some("*** CONGRATULATIONS *** You have won the game."));
        print_message(Some("You cannot save this game, but you may retire when ready."));
    }

    if dropped_item_id == 0 {
        return 0;
    }

    let mut return_flags: u32 = 0;

    if (dropped_item_id & 255) != 0 {
        return_flags |= config::monsters::move_flags::CM_CARRY_OBJ;

        if (item_type & 0x04) != 0 {
            return_flags |= config::monsters::move_flags::CM_SMALL_OBJ;
        }
    }

    if dropped_item_id >= 256 {
        return_flags |= config::monsters::move_flags::CM_CARRY_GOLD;
    }

    let mut number_of_items = (dropped_item_id % 256) + (dropped_item_id / 256);
    number_of_items <<= config::monsters::move_flags::CM_TR_SHIFT;

    return_flags | number_of_items
}

pub fn print_monster_action_text(name: &str, action: &str) {
    print_message(Some(&format!("{} {}", name, action)));
}

pub fn monster_name_description(real_name: &str, is_lit: bool) -> String {
    if is_lit {
        return format!("The {}", real_name);
    }
    "It".to_string()
}

// Sleep creatures adjacent to player -RAK-
pub fn monster_sleep(coord: Coord) -> bool {
    let mut asleep = false;

    let mut y = coord.y - 1;
    while y <= coord.y + 1 && y < crate::dungeon::MAX_HEIGHT {
        let mut x = coord.x - 1;
        while x <= coord.x + 1 && x < crate::dungeon::MAX_WIDTH {
            let monster_id = dg().floor[y as usize][x as usize].creature_id as usize;

            if monster_id <= 1 {
                x += 1;
                continue;
            }

            let creature_id = monsters()[monster_id].creature_id as usize;
            let creature_level = CREATURES_LIST[creature_id].level;
            let creature_defenses = CREATURES_LIST[creature_id].defenses;

            let name = monster_name_description(CREATURES_LIST[creature_id].name, monsters()[monster_id].lit);

            if random_number(MON_MAX_LEVELS as i32) < creature_level as i32 || (creature_defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                if monsters()[monster_id].lit && (creature_defenses & config::monsters::defense::CD_NO_SLEEP) != 0 {
                    creature_recall()[creature_id].defenses |= config::monsters::defense::CD_NO_SLEEP;
                }

                print_monster_action_text(&name, "is unaffected.");
            } else {
                monsters()[monster_id].sleep_count = 500;
                asleep = true;

                print_monster_action_text(&name, "falls asleep.");
            }

            x += 1;
        }
        y += 1;
    }

    asleep
}

fn execute_attack_on_player(creature_level: u8, monster_hp: &mut i16, monster_id: i32, attack_type: i32, damage: i32, death_description: &str, noticed: bool) -> bool {
    let mut noticed = noticed;
    let mut damage = damage;

    match attack_type {
        1 => {
            // Normal attack
            // round half-way case down
            damage -= ((py().misc.ac as i32 + py().misc.magical_ac as i32) * damage) / 200;
            crate::player::player_takes_hit(damage, death_description);
        }
        2 => {
            // Lose Strength
            crate::player::player_takes_hit(damage, death_description);
            if py().flags.sustain_str {
                print_message(Some("You feel weaker for a moment, but it passes."));
            } else if random_number(2) == 1 {
                print_message(Some("You feel weaker."));
                crate::player::player_stat_random_decrease(A_STR);
            } else {
                noticed = false;
            }
        }
        3 => {
            // Confusion attack
            crate::player::player_takes_hit(damage, death_description);
            if random_number(2) == 1 {
                if py().flags.confused < 1 {
                    print_message(Some("You feel confused."));
                    py().flags.confused += random_number(creature_level as i32) as i16;
                } else {
                    noticed = false;
                }
                py().flags.confused += 3;
            } else {
                noticed = false;
            }
        }
        4 => {
            // Fear attack
            crate::player::player_takes_hit(damage, death_description);
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects!"));
            } else if py().flags.afraid < 1 {
                print_message(Some("You are suddenly afraid!"));
                py().flags.afraid += 3 + random_number(creature_level as i32) as i16;
            } else {
                py().flags.afraid += 3;
                noticed = false;
            }
        }
        5 => {
            // Fire attack
            print_message(Some("You are enveloped in flames!"));
            crate::inventory::damage_fire(damage, death_description);
        }
        6 => {
            // Acid attack
            print_message(Some("You are covered in acid!"));
            crate::inventory::damage_acid(damage, death_description);
        }
        7 => {
            // Cold attack
            print_message(Some("You are covered with frost!"));
            crate::inventory::damage_cold(damage, death_description);
        }
        8 => {
            // Lightning attack
            print_message(Some("Lightning strikes you!"));
            crate::inventory::damage_lightning_bolt(damage, death_description);
        }
        9 => {
            // Corrosion attack
            print_message(Some("A stinging red gas swirls about you."));
            crate::inventory::damage_corroding_gas(death_description);
            crate::player::player_takes_hit(damage, death_description);
        }
        10 => {
            // Blindness attack
            crate::player::player_takes_hit(damage, death_description);
            if py().flags.blind < 1 {
                py().flags.blind += 10 + random_number(creature_level as i32) as i16;
                print_message(Some("Your eyes begin to sting."));
            } else {
                py().flags.blind += 5;
                noticed = false;
            }
        }
        11 => {
            // Paralysis attack
            crate::player::player_takes_hit(damage, death_description);
            if crate::player::player_saving_throw() {
                print_message(Some("You resist the effects!"));
            } else if py().flags.paralysis < 1 {
                if py().flags.free_action {
                    print_message(Some("You are unaffected."));
                } else {
                    py().flags.paralysis = (random_number(creature_level as i32) + 3) as i16;
                    print_message(Some("You are paralyzed."));
                }
            } else {
                noticed = false;
            }
        }
        12 => {
            // Steal Money
            if py().flags.paralysis < 1 && random_number(124) < py().stats.used[A_DEX] as i32 {
                print_message(Some("You quickly protect your money pouch!"));
            } else {
                let gold = (py().misc.au / 10) + random_number(25);
                if gold > py().misc.au {
                    py().misc.au = 0;
                } else {
                    py().misc.au -= gold;
                }
                print_message(Some("Your purse feels lighter."));
                print_character_gold_value();
            }
            if random_number(2) == 1 {
                print_message(Some("There is a puff of smoke!"));
                crate::spells::spell_teleport_away_monster(monster_id, config::monsters::MON_MAX_SIGHT as i32);
            }
        }
        13 => {
            // Steal Object
            if py().flags.paralysis < 1 && random_number(124) < py().stats.used[A_DEX] as i32 {
                print_message(Some("You grab hold of your backpack!"));
            } else {
                let item_id = (random_number(py().pack.unique_items as i32) - 1) as usize;
                crate::inventory::inventory_destroy_item(item_id);
                print_message(Some("Your backpack feels lighter."));
            }
            if random_number(2) == 1 {
                print_message(Some("There is a puff of smoke!"));
                crate::spells::spell_teleport_away_monster(monster_id, config::monsters::MON_MAX_SIGHT as i32);
            }
        }
        14 => {
            // Poison
            crate::player::player_takes_hit(damage, death_description);
            print_message(Some("You feel very sick."));
            py().flags.poisoned += (random_number(creature_level as i32) + 5) as i16;
        }
        15 => {
            // Lose dexterity
            crate::player::player_takes_hit(damage, death_description);
            if py().flags.sustain_dex {
                print_message(Some("You feel clumsy for a moment, but it passes."));
            } else {
                print_message(Some("You feel more clumsy."));
                crate::player::player_stat_random_decrease(A_DEX);
            }
        }
        16 => {
            // Lose constitution
            crate::player::player_takes_hit(damage, death_description);
            if py().flags.sustain_con {
                print_message(Some("Your body resists the effects of the disease."));
            } else {
                print_message(Some("Your health is damaged!"));
                crate::player::player_stat_random_decrease(A_CON);
            }
        }
        17 => {
            // Lose intelligence
            crate::player::player_takes_hit(damage, death_description);
            print_message(Some("You have trouble thinking clearly."));
            if py().flags.sustain_int {
                print_message(Some("But your mind quickly clears."));
            } else {
                crate::player::player_stat_random_decrease(A_INT);
            }
        }
        18 => {
            // Lose wisdom
            crate::player::player_takes_hit(damage, death_description);
            if py().flags.sustain_wis {
                print_message(Some("Your wisdom is sustained."));
            } else {
                print_message(Some("Your wisdom is drained."));
                crate::player::player_stat_random_decrease(A_WIS);
            }
        }
        19 => {
            // Lose experience
            print_message(Some("You feel your life draining away!"));
            crate::spells::spell_lose_exp(damage + (py().misc.exp / 100) * config::monsters::MON_PLAYER_EXP_DRAINED_PER_HIT as i32);
        }
        20 => {
            // Aggravate monster
            crate::spells::spell_aggravate_monsters(20);
        }
        21 => {
            // Disenchant
            if crate::inventory::execute_disenchant_attack() {
                print_message(Some("There is a static feeling in the air."));
                crate::player::player_recalculate_bonuses();
            } else {
                noticed = false;
            }
        }
        22 => {
            // Eat food
            let mut item_pos_start = 0;
            let mut item_pos_end = 0;
            if crate::inventory::inventory_find_range(TV_FOOD as i32, TV_NEVER as i32, &mut item_pos_start, &mut item_pos_end) {
                crate::inventory::inventory_destroy_item(item_pos_start as usize);
                print_message(Some("It got at your rations!"));
            } else {
                noticed = false;
            }
        }
        23 => {
            // Eat light
            noticed = crate::inventory::inventory_diminish_light_attack(noticed);
        }
        24 => {
            // Eat charges
            noticed = crate::inventory::inventory_diminish_charges_attack(creature_level, monster_hp, noticed);
        }
        _ => {
            noticed = false;
        }
    }

    noticed
}
