// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player functions related to traps

use crate::config;
use crate::data_player::CLASS_LEVEL_ADJ;
use crate::dice::{dice_roll, Dice};
use crate::dungeon::{dg, dungeon_delete_object};
use crate::game::{game, get_direction_with_memory, random_number};
use crate::identification::{
    object_blocked_by_monster, spell_item_identified,
    spell_item_identify_and_remove_random_inscription, SpecialNameIds,
};
use crate::player::{
    py, player_move_position, player_no_light, player_takes_hit, A_INT, A_STR, CLASS_DISARM,
};
use crate::player_move::player_move;
use crate::player_stats::{player_disarm_adjustment, player_stat_adjustment_wisdom_intelligence, player_stat_random_decrease};
use crate::treasure::{TV_CHEST, TV_VIS_TRAP};
use crate::types::Coord;
use crate::ui::display_character_experience;
use crate::ui_io::{print_message, print_message_no_command_interrupt};

fn player_trap_disarm_ability() -> i32 {
    let mut ability = py().misc.disarm as i32;
    ability += 2;
    ability *= player_disarm_adjustment() as i32;
    ability += player_stat_adjustment_wisdom_intelligence(A_INT);
    ability += CLASS_LEVEL_ADJ[py().misc.class_id as usize][CLASS_DISARM] as i32 * py().misc.level as i32 / 3;

    if py().flags.blind > 0 || player_no_light() {
        ability /= 10;
    }

    if py().flags.confused > 0 {
        ability /= 10;
    }

    if py().flags.image > 0 {
        ability /= 10;
    }

    ability
}

fn player_disarm_floor_trap(coord: Coord, total: i32, level: i32, dir: i32, misc_use: i16) {
    let confused = py().flags.confused;

    if total + 100 - level > random_number(100) {
        print_message(Some("You have disarmed the trap."));
        py().misc.exp += misc_use as i32;
        dungeon_delete_object(coord);

        // make sure we move onto the trap even if confused
        py().flags.confused = 0;
        player_move(dir, false);
        py().flags.confused = confused;

        display_character_experience();
        return;
    }

    // avoid random_number(0) call
    if total > 5 && random_number(total) > 5 {
        print_message_no_command_interrupt("You failed to disarm the trap.");
        return;
    }

    print_message(Some("You set the trap off!"));

    // make sure we move onto the trap even if confused
    py().flags.confused = 0;
    player_move(dir, false);
    py().flags.confused += confused;
}

fn player_disarm_chest_trap(coord: Coord, total: i32, treasure_id: usize) {
    if !spell_item_identified(&game().treasure.list[treasure_id]) {
        game().player_free_turn = true;
        print_message(Some("I don't see a trap."));

        return;
    }

    if (game().treasure.list[treasure_id].flags & config::treasure::chests::CH_TRAPPED) != 0 {
        let level = game().treasure.list[treasure_id].depth_first_found as i32;

        if (total - level) > random_number(100) {
            let item = &mut game().treasure.list[treasure_id];
            item.flags &= !config::treasure::chests::CH_TRAPPED;

            if (item.flags & config::treasure::chests::CH_LOCKED) != 0 {
                item.special_name_id = SpecialNameIds::SnLocked as u8;
            } else {
                item.special_name_id = SpecialNameIds::SnDisarmed as u8;
            }

            print_message(Some("You have disarmed the chest."));

            spell_item_identify_and_remove_random_inscription(&mut game().treasure.list[treasure_id]);
            py().misc.exp += level;

            display_character_experience();
        } else if total > 5 && random_number(total) > 5 {
            print_message_no_command_interrupt("You failed to disarm the chest.");
        } else {
            print_message(Some("You set a trap off!"));
            spell_item_identify_and_remove_random_inscription(&mut game().treasure.list[treasure_id]);
            chest_trap(coord);
        }
        return;
    }

    print_message(Some("The chest was not trapped."));
    game().player_free_turn = true;
}

// Disarms a trap -RAK-
pub fn player_disarm_trap() {
    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = py().pos;
    player_move_position(dir, &mut coord);

    let tile = *dg().tile(coord);

    let mut no_disarm = false;

    let category_id = game().treasure.list[tile.treasure_id as usize].category_id;

    if tile.creature_id > 1 && tile.treasure_id != 0 && (category_id == TV_VIS_TRAP || category_id == TV_CHEST) {
        object_blocked_by_monster(tile.creature_id as usize);
    } else if tile.treasure_id != 0 {
        let disarm_ability = player_trap_disarm_ability();

        let treasure_id = tile.treasure_id as usize;
        let item = game().treasure.list[treasure_id];

        if item.category_id == TV_VIS_TRAP {
            player_disarm_floor_trap(coord, disarm_ability, item.depth_first_found as i32, dir, item.misc_use);
        } else if item.category_id == TV_CHEST {
            player_disarm_chest_trap(coord, disarm_ability, treasure_id);
        } else {
            no_disarm = true;
        }
    } else {
        no_disarm = true;
    }

    if no_disarm {
        print_message(Some("I do not see anything to disarm there."));
        game().player_free_turn = true;
    }
}

fn chest_loose_strength() {
    print_message(Some("A small needle has pricked you!"));

    if py().flags.sustain_str {
        print_message(Some("You are unaffected."));
        return;
    }

    player_stat_random_decrease(A_STR);

    player_takes_hit(dice_roll(Dice::new(1, 4)), "a poison needle");

    print_message(Some("You feel weakened!"));
}

fn chest_poison() {
    print_message(Some("A small needle has pricked you!"));

    player_takes_hit(dice_roll(Dice::new(1, 6)), "a poison needle");

    py().flags.poisoned += (10 + random_number(20)) as i16;
}

fn chest_paralysed() {
    print_message(Some("A puff of yellow gas surrounds you!"));

    if py().flags.free_action {
        print_message(Some("You are unaffected."));
        return;
    }

    print_message(Some("You choke and pass out."));
    py().flags.paralysis = (10 + random_number(20)) as i16;
}

fn chest_summon_monster(coord: Coord) {
    for _ in 0..3 {
        let mut position = coord;
        crate::monster_manager::monster_summon(&mut position, false);
    }
}

fn chest_explode(coord: Coord) {
    print_message(Some("There is a sudden explosion!"));

    dungeon_delete_object(coord);

    player_takes_hit(dice_roll(Dice::new(5, 8)), "an exploding chest");
}

// Chests have traps too. -RAK-
// Note: Chest traps are based on the FLAGS value
pub fn chest_trap(coord: Coord) {
    let flags = game().treasure.list[dg().tile(coord).treasure_id as usize].flags;

    if (flags & config::treasure::chests::CH_LOSE_STR) != 0 {
        chest_loose_strength();
    }

    if (flags & config::treasure::chests::CH_POISON) != 0 {
        chest_poison();
    }

    if (flags & config::treasure::chests::CH_PARALYSED) != 0 {
        chest_paralysed();
    }

    if (flags & config::treasure::chests::CH_SUMMON) != 0 {
        chest_summon_monster(coord);
    }

    if (flags & config::treasure::chests::CH_EXPLODE) != 0 {
        chest_explode(coord);
    }
}
