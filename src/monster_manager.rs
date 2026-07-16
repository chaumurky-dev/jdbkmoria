// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Monster management: generation, placement, cleanup

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::dice::{dice_roll, max_dice_roll};
use crate::dungeon::{coord_distance_between, coord_in_bounds, dg, dungeon_delete_monster, dungeon_remove_monster_from_level};
use crate::dungeon_tile::{MAX_OPEN_SPACE, MIN_CLOSED_SPACE};
use crate::game::{game, random_number, random_number_normal_distribution};
use crate::monster::{hack_monptr, monster_levels, monsters, next_free_monster_id, MON_MAX_LEVELS, MON_TOTAL_ALLOCATIONS};
use crate::player::py;
use crate::tr;
use crate::types::Coord;
use crate::ui_io::print_message;

// Returns a pointer to next free space -RAK-
// Returns -1 if could not allocate a monster.
fn popm() -> i32 {
    if *next_free_monster_id() as usize == MON_TOTAL_ALLOCATIONS {
        if !compact_monsters() {
            return -1;
        }
    }
    let id = *next_free_monster_id();
    *next_free_monster_id() += 1;
    id as i32
}

// Places a monster at given location -RAK-
pub fn monster_place_new(coord: Coord, creature_id: i32, sleeping: bool) -> bool {
    let monster_id = popm();

    // Check for case where could not allocate space for the monster
    if monster_id == -1 {
        return false;
    }

    let creature = &CREATURES_LIST[creature_id as usize];

    let hp = if (creature.defenses & config::monsters::defense::CD_MAX_HP) != 0 {
        max_dice_roll(creature.hit_die) as i16
    } else {
        dice_roll(creature.hit_die) as i16
    };

    let sleep_count = if sleeping {
        if creature.sleep_counter == 0 {
            0
        } else {
            (creature.sleep_counter as i32 * 2 + random_number(creature.sleep_counter as i32 * 10)) as i16
        }
    } else {
        0
    };

    let monster = &mut monsters()[monster_id as usize];

    monster.pos = coord;
    monster.creature_id = creature_id as u16;
    monster.hp = hp;

    // the creatures_list[] speed value is 10 greater, so that it can be a uint8_t
    monster.speed = creature.speed as i16 - 10 + py().flags.speed;
    monster.stunned_amount = 0;
    monster.distance_from_player = coord_distance_between(py().pos, coord).min(255) as u8;
    monster.lit = false;
    monster.sleep_count = sleep_count;

    dg().tile_mut(coord).creature_id = monster_id as u16;

    true
}

// Places a monster at given location -RAK-
pub fn monster_place_winning() {
    if game().total_winner {
        return;
    }

    let mut coord;

    loop {
        coord = Coord::new(random_number(dg().height as i32 - 2), random_number(dg().width as i32 - 2));

        let tile = dg().tile(coord);
        if tile.feature_id < MIN_CLOSED_SPACE
            && tile.creature_id == 0
            && tile.treasure_id == 0
            && coord_distance_between(coord, py().pos) > config::monsters::MON_MAX_SIGHT as i32
        {
            break;
        }
    }

    let creature_id = random_number(config::monsters::MON_ENDGAME_MONSTERS as i32) - 1 + monster_levels()[MON_MAX_LEVELS] as i32;

    // The following code is now exactly the same as monster_place_new() except here
    // we abort on failed placement, and do not set `monster.lit = false`.

    let monster_id = popm();

    // Check for case where could not allocate space for the win monster, this should never happen.
    if monster_id == -1 {
        std::process::abort();
    }

    let creature = &CREATURES_LIST[creature_id as usize];

    let hp = if (creature.defenses & config::monsters::defense::CD_MAX_HP) != 0 {
        max_dice_roll(creature.hit_die) as i16
    } else {
        dice_roll(creature.hit_die) as i16
    };

    let monster = &mut monsters()[monster_id as usize];

    monster.pos = coord;
    monster.creature_id = creature_id as u16;
    monster.hp = hp;

    // the creatures_list speed value is 10 greater, so that it can be a uint8_t
    monster.speed = creature.speed as i16 - 10 + py().flags.speed;
    monster.stunned_amount = 0;
    monster.distance_from_player = coord_distance_between(py().pos, coord).min(255) as u8;

    dg().tile_mut(coord).creature_id = monster_id as u16;

    monster.sleep_count = 0;
}

// Return a monster suitable to be placed at a given level. This
// makes high level monsters (up to the given level) slightly more
// common than low level monsters at any given level. -CJS-
fn monster_get_one_suitable_for_level(level: i32) -> i32 {
    if level == 0 {
        return random_number(monster_levels()[0] as i32) - 1;
    }

    let mut level = level;

    if level > MON_MAX_LEVELS as i32 {
        level = MON_MAX_LEVELS as i32;
    }

    if random_number(config::monsters::MON_CHANCE_OF_NASTY as i32) == 1 {
        let abs_distribution = random_number_normal_distribution(0, 4).abs();
        level += abs_distribution + 1;
        if level > MON_MAX_LEVELS as i32 {
            level = MON_MAX_LEVELS as i32;
        }
    } else {
        // This code has been added to make it slightly more likely to get
        // the higher level monsters. Originally a uniform distribution over
        // all monsters of level less than or equal to the dungeon level.
        // This distribution makes a level n monster occur approx 2/n% of the
        // time on level n, and 1/n*n% are 1st level.
        let num = monster_levels()[level as usize] as i32 - monster_levels()[0] as i32;
        let mut i = random_number(num) - 1;
        let j = random_number(num) - 1;
        if j > i {
            i = j;
        }
        level = CREATURES_LIST[(i + monster_levels()[0] as i32) as usize].level as i32;
    }

    random_number(monster_levels()[level as usize] as i32 - monster_levels()[(level - 1) as usize] as i32) - 1 + monster_levels()[(level - 1) as usize] as i32
}

// Allocates a random monster -RAK-
pub fn monster_place_new_within_distance(number: i32, distance_from_source: i32, sleeping: bool) {
    for _ in 0..number {
        let mut position;

        loop {
            position = Coord::new(random_number(dg().height as i32 - 2), random_number(dg().width as i32 - 2));
            let tile = dg().tile(position);
            if tile.feature_id < MIN_CLOSED_SPACE && tile.creature_id == 0 && coord_distance_between(position, py().pos) > distance_from_source {
                break;
            }
        }

        let l = monster_get_one_suitable_for_level(dg().current_level as i32);

        // Dragons are always created sleeping here,
        // so as to give the player a sporting chance.
        let mut sleeping = sleeping;
        if CREATURES_LIST[l as usize].sprite == b'd' || CREATURES_LIST[l as usize].sprite == b'D' {
            sleeping = true;
        }

        // Place_monster() should always return true here.
        // It does not matter if it fails though.
        monster_place_new(position, l, sleeping);
    }
}

// (pub for the jdbkmoria paintings extension: creatures break out of a canvas)
pub fn place_monster_adjacent_to(monster_id: i32, coord: &mut Coord, slp: bool) -> bool {
    let mut placed = false;

    let mut i = 0;
    while i <= 9 {
        let position = Coord::new(coord.y - 2 + random_number(3), coord.x - 2 + random_number(3));

        if coord_in_bounds(position) {
            let tile = dg().tile(position);
            if tile.feature_id <= MAX_OPEN_SPACE && tile.creature_id == 0 {
                // Place_monster() should always return true here.
                if !monster_place_new(position, monster_id, slp) {
                    return false;
                }

                *coord = position;

                placed = true;
                i = 9;
            }
        }
        i += 1;
    }

    placed
}

// Places creature adjacent to given location -RAK-
pub fn monster_summon(coord: &mut Coord, sleeping: bool) -> bool {
    let monster_id = monster_get_one_suitable_for_level(dg().current_level as i32 + config::monsters::MON_SUMMONED_LEVEL_ADJUST as i32);
    place_monster_adjacent_to(monster_id, coord, sleeping)
}

// Places undead adjacent to given location -RAK-
pub fn monster_summon_undead(coord: &mut Coord) -> bool {
    let mut monster_id;
    let mut max_levels = monster_levels()[MON_MAX_LEVELS] as i32;

    loop {
        monster_id = random_number(max_levels) - 1;
        let mut i = 0;
        while i <= 19 {
            if (CREATURES_LIST[monster_id as usize].defenses & config::monsters::defense::CD_UNDEAD) != 0 {
                i = 20;
                max_levels = 0;
            } else {
                monster_id += 1;
                if monster_id > max_levels {
                    i = 20;
                } else {
                    i += 1;
                }
            }
        }

        if max_levels == 0 {
            break;
        }
    }

    place_monster_adjacent_to(monster_id, coord, false)
}

// Compact monsters -RAK-
// Return true if any monsters were deleted, false if could not delete any monsters.
pub fn compact_monsters() -> bool {
    print_message(Some(tr!("Compacting monsters...")));

    let mut cur_dis = 66;
    let mut delete_any = false;

    while !delete_any {
        let mut i = *next_free_monster_id() as i32 - 1;
        while i >= config::monsters::MON_MIN_INDEX_ID as i32 {
            if cur_dis < monsters()[i as usize].distance_from_player as i32 && random_number(3) == 1 {
                let movement = CREATURES_LIST[monsters()[i as usize].creature_id as usize].movement;

                if (movement & config::monsters::move_flags::CM_WIN) != 0 {
                    // Never compact away the Balrog!!
                } else if *hack_monptr() < i {
                    // In case this is called from within update_monsters().
                    // This is a horrible hack, the monsters/update_monsters() code needs to be rewritten.
                    dungeon_delete_monster(i);
                    delete_any = true;
                } else {
                    // dungeon_remove_monster_from_level() does not decrement next_free_monster_id,
                    // so don't set delete_any if this was called.
                    dungeon_remove_monster_from_level(i);
                }
            }
            i -= 1;
        }

        if !delete_any {
            cur_dis -= 6;

            // Can't delete any monsters, return failure.
            if cur_dis < 0 {
                return false;
            }
        }
    }

    true
}
