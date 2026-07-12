// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Functions related to Player stats

use crate::config;
use crate::data_player::CLASSES;
use crate::data_tables::BLOWS_TABLE;
use crate::game::random_number;
use crate::player::{
    py, player_calculate_allowed_spells_count, player_gain_mana, player_recalculate_bonuses,
    A_CHR, A_CON, A_DEX, A_INT, A_STR, A_WIS, PLAYER_MAX_LEVEL,
};
use crate::ui::display_character_stats;

// I don't really like this, but for now, it's better than being a global -MRC-
pub fn player_initialize_base_experience_levels() {
    let levels: [u32; PLAYER_MAX_LEVEL] = [
        10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1100, 1400, 1800, 2300, 2900, 3600, 4400, 5400,
        6800, 8400, 10200, 12500, 17500, 25000, 35000, 50000, 75000, 100000, 150000, 200000, 300000, 400000, 500000, 750000, 1500000, 2500000, 5000000, 10000000,
    ];

    py().base_exp_levels = levels;
}

// Calculate the players hit points
pub fn player_calculate_hit_points() {
    let mut hp = py().base_hp_levels[py().misc.level as usize - 1] as i32 + (player_stat_adjustment_constitution() * py().misc.level as i32);

    // Always give at least one point per level + 1
    if hp < py().misc.level as i32 + 1 {
        hp = py().misc.level as i32 + 1;
    }

    if (py().flags.status & config::player::status::PY_HERO) != 0 {
        hp += 10;
    }

    if (py().flags.status & config::player::status::PY_SHERO) != 0 {
        hp += 20;
    }

    // MHP can equal zero while character is being created
    if hp != py().misc.max_hp as i32 && py().misc.max_hp != 0 {
        // Change current hit points proportionately to change of MHP,
        // divide first to avoid overflow, little loss of accuracy
        let value = (((py().misc.current_hp as i32) << 16) + py().misc.current_hp_fraction as i32) / py().misc.max_hp as i32 * hp;
        py().misc.current_hp = (value >> 16) as i16;
        py().misc.current_hp_fraction = (value & 0xFFFF) as u16;
        py().misc.max_hp = hp as i16;

        // can't print hit points here, may be in store or inventory mode
        py().flags.status |= config::player::status::PY_HP;
    }
}

fn player_attack_blows_dexterity(dexterity: i32) -> usize {
    if dexterity < 10 {
        0
    } else if dexterity < 19 {
        1
    } else if dexterity < 68 {
        2
    } else if dexterity < 108 {
        3
    } else if dexterity < 118 {
        4
    } else {
        5
    }
}

fn player_attack_blows_strength(strength: i32, weight: i32) -> usize {
    let adj_weight = strength * 10 / weight;

    if adj_weight < 2 {
        0
    } else if adj_weight < 3 {
        1
    } else if adj_weight < 4 {
        2
    } else if adj_weight < 5 {
        3
    } else if adj_weight < 7 {
        4
    } else if adj_weight < 9 {
        5
    } else {
        6
    }
}

// Weapon weight VS strength and dexterity -RAK-
pub fn player_attack_blows(weight: i32, weight_to_hit: &mut i32) -> i32 {
    *weight_to_hit = 0;

    let player_strength = py().stats.used[A_STR] as i32;

    if player_strength * 15 < weight {
        *weight_to_hit = player_strength * 15 - weight;
        return 1;
    }

    let dexterity = player_attack_blows_dexterity(py().stats.used[A_DEX] as i32);
    let strength = player_attack_blows_strength(player_strength, weight);

    BLOWS_TABLE[strength][dexterity] as i32
}

// Adjustment for wisdom/intelligence -JWT-
pub fn player_stat_adjustment_wisdom_intelligence(stat: usize) -> i32 {
    let value = py().stats.used[stat] as i32;

    if value > 117 {
        7
    } else if value > 107 {
        6
    } else if value > 87 {
        5
    } else if value > 67 {
        4
    } else if value > 17 {
        3
    } else if value > 14 {
        2
    } else if value > 7 {
        1
    } else {
        0
    }
}

// Adjustment for charisma -RAK-
// Percent decrease or increase in price of goods
pub fn player_stat_adjustment_charisma() -> i32 {
    let charisma = py().stats.used[A_CHR] as i32;

    if charisma > 117 {
        return 90;
    }

    if charisma > 107 {
        return 92;
    }

    if charisma > 87 {
        return 94;
    }

    if charisma > 67 {
        return 96;
    }

    if charisma > 18 {
        return 98;
    }

    match charisma {
        18 => 100,
        17 => 101,
        16 => 102,
        15 => 103,
        14 => 104,
        13 => 106,
        12 => 108,
        11 => 110,
        10 => 112,
        9 => 114,
        8 => 116,
        7 => 118,
        6 => 120,
        5 => 122,
        4 => 125,
        3 => 130,
        _ => 100,
    }
}

// Returns a character's adjustment to hit points -JWT-
pub fn player_stat_adjustment_constitution() -> i32 {
    let con = py().stats.used[A_CON] as i32;

    if con < 7 {
        return con - 7;
    }

    if con < 17 {
        return 0;
    }

    if con == 17 {
        return 1;
    }

    if con < 94 {
        return 2;
    }

    if con < 117 {
        return 3;
    }

    4
}

fn player_modify_stat(stat: usize, amount: i16) -> u8 {
    let mut new_stat = py().stats.current[stat] as i32;

    let loop_count = amount.abs();

    for _ in 0..loop_count {
        if amount > 0 {
            if new_stat < 18 {
                new_stat += 1;
            } else if new_stat < 108 {
                new_stat += 10;
            } else {
                new_stat = 118;
            }
        } else if new_stat > 27 {
            new_stat -= 10;
        } else if new_stat > 18 {
            new_stat = 18;
        } else if new_stat > 3 {
            new_stat -= 1;
        }
    }

    new_stat as u8
}

// Set the value of the stat which is actually used. -CJS-
pub fn player_set_and_use_stat(stat: usize) {
    py().stats.used[stat] = player_modify_stat(stat, py().stats.modified[stat]);

    if stat == A_STR {
        py().flags.status |= config::player::status::PY_STR_WGT;
        player_recalculate_bonuses();
    } else if stat == A_DEX {
        player_recalculate_bonuses();
    } else if stat == A_INT && CLASSES[py().misc.class_id as usize].class_to_use_mage_spells == config::spells::SPELL_TYPE_MAGE {
        player_calculate_allowed_spells_count(A_INT);
        player_gain_mana(A_INT);
    } else if stat == A_WIS && CLASSES[py().misc.class_id as usize].class_to_use_mage_spells == config::spells::SPELL_TYPE_PRIEST {
        player_calculate_allowed_spells_count(A_WIS);
        player_gain_mana(A_WIS);
    } else if stat == A_CON {
        player_calculate_hit_points();
    }
}

// Increases a stat by one randomized level -RAK-
pub fn player_stat_random_increase(stat: usize) -> bool {
    let mut new_stat = py().stats.current[stat] as i32;

    if new_stat >= 118 {
        return false;
    }

    if new_stat >= 18 && new_stat < 116 {
        // stat increases by 1/6 to 1/3 of difference from max
        let gain = ((118 - new_stat) / 3 + 1) >> 1;

        new_stat += random_number(gain) + gain;
    } else {
        new_stat += 1;
    }

    py().stats.current[stat] = new_stat as u8;

    if new_stat > py().stats.max[stat] as i32 {
        py().stats.max[stat] = new_stat as u8;
    }

    player_set_and_use_stat(stat);
    display_character_stats(stat);

    true
}

// Decreases a stat by one randomized level -RAK-
pub fn player_stat_random_decrease(stat: usize) -> bool {
    let mut new_stat = py().stats.current[stat] as i32;

    if new_stat <= 3 {
        return false;
    }

    if new_stat >= 19 && new_stat < 117 {
        let loss = (((118 - new_stat) >> 1) + 1) >> 1;
        new_stat += -random_number(loss) - loss;

        if new_stat < 18 {
            new_stat = 18;
        }
    } else {
        new_stat -= 1;
    }

    py().stats.current[stat] = new_stat as u8;

    player_set_and_use_stat(stat);
    display_character_stats(stat);

    true
}

// Restore a stat.  Return true only if this actually makes a difference.
pub fn player_stat_restore(stat: usize) -> bool {
    let new_stat = py().stats.max[stat] as i32 - py().stats.current[stat] as i32;

    if new_stat == 0 {
        return false;
    }

    py().stats.current[stat] = (py().stats.current[stat] as i32 + new_stat) as u8;

    player_set_and_use_stat(stat);
    display_character_stats(stat);

    true
}

// Boost a stat artificially (by wearing something). If the display
// argument is true, then increase is shown on the screen.
pub fn player_stat_boost(stat: usize, amount: i32) {
    py().stats.modified[stat] += amount as i16;

    player_set_and_use_stat(stat);

    // can not call display_character_stats() here:
    //   might be in a store,
    //   might be in inventory_execute_command()
    py().flags.status |= config::player::status::PY_STR << stat;
}

// Returns a character's adjustment to hit. -JWT-
pub fn player_to_hit_adjustment() -> i16 {
    let mut total: i16;

    let dexterity = py().stats.used[A_DEX] as i32;
    if dexterity < 4 {
        total = -3;
    } else if dexterity < 6 {
        total = -2;
    } else if dexterity < 8 {
        total = -1;
    } else if dexterity < 16 {
        total = 0;
    } else if dexterity < 17 {
        total = 1;
    } else if dexterity < 18 {
        total = 2;
    } else if dexterity < 69 {
        total = 3;
    } else if dexterity < 118 {
        total = 4;
    } else {
        total = 5;
    }

    let strength = py().stats.used[A_STR] as i32;
    if strength < 4 {
        total -= 3;
    } else if strength < 5 {
        total -= 2;
    } else if strength < 7 {
        total -= 1;
    } else if strength < 18 {
        total -= 0;
    } else if strength < 94 {
        total += 1;
    } else if strength < 109 {
        total += 2;
    } else if strength < 117 {
        total += 3;
    } else {
        total += 4;
    }

    total
}

// Returns a character's adjustment to armor class -JWT-
pub fn player_armor_class_adjustment() -> i16 {
    let stat = py().stats.used[A_DEX] as i32;

    if stat < 4 {
        -4
    } else if stat == 4 {
        -3
    } else if stat == 5 {
        -2
    } else if stat == 6 {
        -1
    } else if stat < 15 {
        0
    } else if stat < 18 {
        1
    } else if stat < 59 {
        2
    } else if stat < 94 {
        3
    } else if stat < 117 {
        4
    } else {
        5
    }
}

// Returns a character's adjustment to disarm -RAK-
pub fn player_disarm_adjustment() -> i16 {
    let stat = py().stats.used[A_DEX] as i32;

    if stat < 4 {
        -8
    } else if stat == 4 {
        -6
    } else if stat == 5 {
        -4
    } else if stat == 6 {
        -2
    } else if stat == 7 {
        -1
    } else if stat < 13 {
        0
    } else if stat < 16 {
        1
    } else if stat < 18 {
        2
    } else if stat < 59 {
        4
    } else if stat < 94 {
        5
    } else if stat < 117 {
        6
    } else {
        8
    }
}

// Returns a character's adjustment to damage -JWT-
pub fn player_damage_adjustment() -> i16 {
    let stat = py().stats.used[A_STR] as i32;

    if stat < 4 {
        -2
    } else if stat < 5 {
        -1
    } else if stat < 16 {
        0
    } else if stat < 17 {
        1
    } else if stat < 18 {
        2
    } else if stat < 94 {
        3
    } else if stat < 109 {
        4
    } else if stat < 117 {
        5
    } else {
        6
    }
}
