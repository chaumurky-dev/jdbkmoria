// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Magical treasure

use crate::config;
use crate::dice::max_dice_roll;
use crate::game::{game, random_number, random_number_normal_distribution};
use crate::identification::SpecialNameIds;
use crate::inventory::Inventory;
use crate::treasure::*;

// Should the object be enchanted -RAK-
fn magic_should_be_enchanted(chance: i32) -> bool {
    random_number(100) <= chance
}

// Enchant a bonus based on degree desired -RAK-
fn magic_enchantment_bonus(base: i32, max_standard: i32, level: i32) -> i32 {
    let mut stand_deviation = (config::treasure::LEVEL_STD_OBJECT_ADJUST as i32 * level / 100) + config::treasure::LEVEL_MIN_OBJECT_STD as i32;

    // Check for level > max_standard since that may have generated an overflow.
    if stand_deviation > max_standard || level > max_standard {
        stand_deviation = max_standard;
    }

    let abs_distribution = random_number_normal_distribution(0, stand_deviation).abs();
    let bonus = (abs_distribution / 10) + base;

    if bonus < base {
        return base;
    }

    bonus
}

fn magical_armor(item: &mut Inventory, special: i32, level: i32) {
    item.to_ac += magic_enchantment_bonus(1, 30, level) as i16;

    if !magic_should_be_enchanted(special) {
        return;
    }

    match random_number(9) {
        1 => {
            item.flags |= config::treasure::flags::TR_RES_LIGHT
                | config::treasure::flags::TR_RES_COLD
                | config::treasure::flags::TR_RES_ACID
                | config::treasure::flags::TR_RES_FIRE;
            item.special_name_id = SpecialNameIds::SnR as u8;
            item.to_ac += 5;
            item.cost += 2500;
        }
        2 => {
            // Resist Acid
            item.flags |= config::treasure::flags::TR_RES_ACID;
            item.special_name_id = SpecialNameIds::SnRa as u8;
            item.cost += 1000;
        }
        3 | 4 => {
            // Resist Fire
            item.flags |= config::treasure::flags::TR_RES_FIRE;
            item.special_name_id = SpecialNameIds::SnRf as u8;
            item.cost += 600;
        }
        5 | 6 => {
            // Resist Cold
            item.flags |= config::treasure::flags::TR_RES_COLD;
            item.special_name_id = SpecialNameIds::SnRc as u8;
            item.cost += 600;
        }
        7..=9 => {
            // Resist Lightning
            item.flags |= config::treasure::flags::TR_RES_LIGHT;
            item.special_name_id = SpecialNameIds::SnRl as u8;
            item.cost += 500;
        }
        _ => {
            // Do not apply any special magic
        }
    }
}

fn cursed_armor(item: &mut Inventory, level: i32) {
    item.to_ac -= magic_enchantment_bonus(1, 40, level) as i16;
    item.cost = 0;
    item.flags |= config::treasure::flags::TR_CURSED;
}

fn magical_sword(item: &mut Inventory, special: i32, level: i32) {
    item.to_hit += magic_enchantment_bonus(0, 40, level) as i16;

    // Magical damage bonus now proportional to weapon base damage
    let damage_bonus = max_dice_roll(item.damage);

    item.to_damage += magic_enchantment_bonus(0, 4 * damage_bonus, damage_bonus * level / 10) as i16;

    // the 3*special/2 is needed because weapons are not as common as
    // before change to treasure distribution, this helps keep same
    // number of ego weapons same as before, see also missiles
    if magic_should_be_enchanted(3 * special / 2) {
        match random_number(16) {
            1 => {
                // Holy Avenger
                item.flags |= config::treasure::flags::TR_SEE_INVIS
                    | config::treasure::flags::TR_SUST_STAT
                    | config::treasure::flags::TR_SLAY_UNDEAD
                    | config::treasure::flags::TR_SLAY_EVIL
                    | config::treasure::flags::TR_STR;
                item.to_hit += 5;
                item.to_damage += 5;
                item.to_ac += random_number(4) as i16;

                // the value in `misc_use` is used for strength increase
                // `misc_use` is also used for sustain stat
                item.misc_use = random_number(4) as i16;
                item.special_name_id = SpecialNameIds::SnHa as u8;
                item.cost += item.misc_use as i32 * 500;
                item.cost += 10000;
            }
            2 => {
                // Defender
                item.flags |= config::treasure::flags::TR_FFALL
                    | config::treasure::flags::TR_RES_LIGHT
                    | config::treasure::flags::TR_SEE_INVIS
                    | config::treasure::flags::TR_FREE_ACT
                    | config::treasure::flags::TR_RES_COLD
                    | config::treasure::flags::TR_RES_ACID
                    | config::treasure::flags::TR_RES_FIRE
                    | config::treasure::flags::TR_REGEN
                    | config::treasure::flags::TR_STEALTH;
                item.to_hit += 3;
                item.to_damage += 3;
                item.to_ac += 5 + random_number(5) as i16;
                item.special_name_id = SpecialNameIds::SnDf as u8;

                // the value in `misc_use` is used for stealth
                item.misc_use = random_number(3) as i16;
                item.cost += item.misc_use as i32 * 500;
                item.cost += 7500;
            }
            3 | 4 => {
                // Slay Animal
                item.flags |= config::treasure::flags::TR_SLAY_ANIMAL;
                item.to_hit += 2;
                item.to_damage += 2;
                item.special_name_id = SpecialNameIds::SnSa as u8;
                item.cost += 3000;
            }
            5 | 6 => {
                // Slay Dragon
                item.flags |= config::treasure::flags::TR_SLAY_DRAGON;
                item.to_hit += 3;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnSd as u8;
                item.cost += 4000;
            }
            7 | 8 => {
                // Slay Evil
                item.flags |= config::treasure::flags::TR_SLAY_EVIL;
                item.to_hit += 3;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnSe as u8;
                item.cost += 4000;
            }
            9 | 10 => {
                // Slay Undead
                item.flags |= config::treasure::flags::TR_SEE_INVIS | config::treasure::flags::TR_SLAY_UNDEAD;
                item.to_hit += 3;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnSu as u8;
                item.cost += 5000;
            }
            11..=13 => {
                // Flame Tongue
                item.flags |= config::treasure::flags::TR_FLAME_TONGUE;
                item.to_hit += 1;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnFt as u8;
                item.cost += 2000;
            }
            14..=16 => {
                // Frost Brand
                item.flags |= config::treasure::flags::TR_FROST_BRAND;
                item.to_hit += 1;
                item.to_damage += 1;
                item.special_name_id = SpecialNameIds::SnFb as u8;
                item.cost += 1200;
            }
            _ => {}
        }
    }
}

fn cursed_sword(item: &mut Inventory, level: i32) {
    item.to_hit -= magic_enchantment_bonus(1, 55, level) as i16;

    // Magical damage bonus now proportional to weapon base damage
    let damage_bonus = max_dice_roll(item.damage);

    item.to_damage -= magic_enchantment_bonus(1, 11 * damage_bonus / 2, damage_bonus * level / 10) as i16;
    item.flags |= config::treasure::flags::TR_CURSED;
    item.cost = 0;
}

fn magical_bow(item: &mut Inventory, level: i32) {
    item.to_hit += magic_enchantment_bonus(1, 30, level) as i16;

    // add damage. -CJS-
    item.to_damage += magic_enchantment_bonus(1, 20, level) as i16;
}

fn cursed_bow(item: &mut Inventory, level: i32) {
    item.to_hit -= magic_enchantment_bonus(1, 50, level) as i16;

    // add damage. -CJS-
    item.to_damage -= magic_enchantment_bonus(1, 30, level) as i16;

    item.flags |= config::treasure::flags::TR_CURSED;
    item.cost = 0;
}

fn magical_digging_tool(item: &mut Inventory, level: i32) {
    item.misc_use += magic_enchantment_bonus(0, 25, level) as i16;
}

fn cursed_digging_tool(item: &mut Inventory, level: i32) {
    item.misc_use = -(magic_enchantment_bonus(1, 30, level) as i16);
    item.cost = 0;
    item.flags |= config::treasure::flags::TR_CURSED;
}

fn magical_gloves(item: &mut Inventory, special: i32, level: i32) {
    item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;

    if !magic_should_be_enchanted(special) {
        return;
    }

    if random_number(2) == 1 {
        item.flags |= config::treasure::flags::TR_FREE_ACT;
        item.special_name_id = SpecialNameIds::SnFreeAction as u8;
        item.cost += 1000;
    } else {
        item.identification |= config::identification::ID_SHOW_HIT_DAM;
        item.to_hit += 1 + random_number(3) as i16;
        item.to_damage += 1 + random_number(3) as i16;
        item.special_name_id = SpecialNameIds::SnSlaying as u8;
        item.cost += (item.to_hit as i32 + item.to_damage as i32) * 250;
    }
}

fn cursed_gloves(item: &mut Inventory, special: i32, level: i32) {
    if magic_should_be_enchanted(special) {
        if random_number(2) == 1 {
            item.flags |= config::treasure::flags::TR_DEX;
            item.special_name_id = SpecialNameIds::SnClumsiness as u8;
        } else {
            item.flags |= config::treasure::flags::TR_STR;
            item.special_name_id = SpecialNameIds::SnWeakness as u8;
        }
        item.identification |= config::identification::ID_SHOW_P1;
        item.misc_use = -(magic_enchantment_bonus(1, 10, level) as i16);
    }

    item.to_ac -= magic_enchantment_bonus(1, 40, level) as i16;
    item.flags |= config::treasure::flags::TR_CURSED;
    item.cost = 0;
}

fn magical_boots(item: &mut Inventory, special: i32, level: i32) {
    item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;

    if !magic_should_be_enchanted(special) {
        return;
    }

    let magic_type = random_number(12);

    if magic_type > 5 {
        item.flags |= config::treasure::flags::TR_FFALL;
        item.special_name_id = SpecialNameIds::SnSlowDescent as u8;
        item.cost += 250;
    } else if magic_type == 1 {
        item.flags |= config::treasure::flags::TR_SPEED;
        item.special_name_id = SpecialNameIds::SnSpeed as u8;
        item.identification |= config::identification::ID_SHOW_P1;
        item.misc_use = 1;
        item.cost += 5000;
    } else {
        // 2 - 5
        item.flags |= config::treasure::flags::TR_STEALTH;
        item.identification |= config::identification::ID_SHOW_P1;
        item.misc_use = random_number(3) as i16;
        item.special_name_id = SpecialNameIds::SnStealth as u8;
        item.cost += 500;
    }
}

fn cursed_boots(item: &mut Inventory, level: i32) {
    let magic_type = random_number(3);

    match magic_type {
        1 => {
            item.flags |= config::treasure::flags::TR_SPEED;
            item.special_name_id = SpecialNameIds::SnSlowness as u8;
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = -1;
        }
        2 => {
            item.flags |= config::treasure::flags::TR_AGGRAVATE;
            item.special_name_id = SpecialNameIds::SnNoise as u8;
        }
        _ => {
            item.special_name_id = SpecialNameIds::SnGreatMass as u8;
            item.weight *= 5;
        }
    }

    item.cost = 0;
    item.to_ac -= magic_enchantment_bonus(2, 45, level) as i16;
    item.flags |= config::treasure::flags::TR_CURSED;
}

fn magical_helms(item: &mut Inventory, special: i32, level: i32) {
    item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;

    if !magic_should_be_enchanted(special) {
        return;
    }

    if item.sub_category_id < 6 {
        item.identification |= config::identification::ID_SHOW_P1;

        let magic_type = random_number(3);

        match magic_type {
            1 => {
                item.misc_use = random_number(2) as i16;
                item.flags |= config::treasure::flags::TR_INT;
                item.special_name_id = SpecialNameIds::SnIntelligence as u8;
                item.cost += item.misc_use as i32 * 500;
            }
            2 => {
                item.misc_use = random_number(2) as i16;
                item.flags |= config::treasure::flags::TR_WIS;
                item.special_name_id = SpecialNameIds::SnWisdom as u8;
                item.cost += item.misc_use as i32 * 500;
            }
            _ => {
                item.misc_use = 1 + random_number(4) as i16;
                item.flags |= config::treasure::flags::TR_INFRA;
                item.special_name_id = SpecialNameIds::SnInfravision as u8;
                item.cost += item.misc_use as i32 * 250;
            }
        }
        return;
    }

    match random_number(6) {
        1 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = random_number(3) as i16;
            item.flags |= config::treasure::flags::TR_FREE_ACT
                | config::treasure::flags::TR_CON
                | config::treasure::flags::TR_DEX
                | config::treasure::flags::TR_STR;
            item.special_name_id = SpecialNameIds::SnMight as u8;
            item.cost += 1000 + item.misc_use as i32 * 500;
        }
        2 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = random_number(3) as i16;
            item.flags |= config::treasure::flags::TR_CHR | config::treasure::flags::TR_WIS;
            item.special_name_id = SpecialNameIds::SnLordliness as u8;
            item.cost += 1000 + item.misc_use as i32 * 500;
        }
        3 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = random_number(3) as i16;
            item.flags |= config::treasure::flags::TR_RES_LIGHT
                | config::treasure::flags::TR_RES_COLD
                | config::treasure::flags::TR_RES_ACID
                | config::treasure::flags::TR_RES_FIRE
                | config::treasure::flags::TR_INT;
            item.special_name_id = SpecialNameIds::SnMagi as u8;
            item.cost += 3000 + item.misc_use as i32 * 500;
        }
        4 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = random_number(3) as i16;
            item.flags |= config::treasure::flags::TR_CHR;
            item.special_name_id = SpecialNameIds::SnBeauty as u8;
            item.cost += 750;
        }
        5 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = (5 * (1 + random_number(4))) as i16;
            item.flags |= config::treasure::flags::TR_SEE_INVIS | config::treasure::flags::TR_SEARCH;
            item.special_name_id = SpecialNameIds::SnSeeing as u8;
            item.cost += 1000 + item.misc_use as i32 * 100;
        }
        6 => {
            item.flags |= config::treasure::flags::TR_REGEN;
            item.special_name_id = SpecialNameIds::SnRegeneration as u8;
            item.cost += 1500;
        }
        _ => {}
    }
}

fn cursed_helms(item: &mut Inventory, special: i32, level: i32) {
    item.to_ac -= magic_enchantment_bonus(1, 45, level) as i16;
    item.flags |= config::treasure::flags::TR_CURSED;
    item.cost = 0;

    if !magic_should_be_enchanted(special) {
        return;
    }

    match random_number(7) {
        1 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = -(random_number(5) as i16);
            item.flags |= config::treasure::flags::TR_INT;
            item.special_name_id = SpecialNameIds::SnStupidity as u8;
        }
        2 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = -(random_number(5) as i16);
            item.flags |= config::treasure::flags::TR_WIS;
            item.special_name_id = SpecialNameIds::SnDullness as u8;
        }
        3 => {
            item.flags |= config::treasure::flags::TR_BLIND;
            item.special_name_id = SpecialNameIds::SnBlindness as u8;
        }
        4 => {
            item.flags |= config::treasure::flags::TR_TIMID;
            item.special_name_id = SpecialNameIds::SnTimidness as u8;
        }
        5 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = -(random_number(5) as i16);
            item.flags |= config::treasure::flags::TR_STR;
            item.special_name_id = SpecialNameIds::SnWeakness as u8;
        }
        6 => {
            item.flags |= config::treasure::flags::TR_TELEPORT;
            item.special_name_id = SpecialNameIds::SnTeleportation as u8;
        }
        7 => {
            item.identification |= config::identification::ID_SHOW_P1;
            item.misc_use = -(random_number(5) as i16);
            item.flags |= config::treasure::flags::TR_CHR;
            item.special_name_id = SpecialNameIds::SnUgliness as u8;
        }
        _ => {}
    }
}

fn process_rings(item: &mut Inventory, level: i32, cursed: i32) {
    match item.sub_category_id {
        0..=3 => {
            if magic_should_be_enchanted(cursed) {
                item.misc_use = -(magic_enchantment_bonus(1, 20, level) as i16);
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            } else {
                item.misc_use = magic_enchantment_bonus(1, 10, level) as i16;
                item.cost += item.misc_use as i32 * 100;
            }
        }
        4 => {
            if magic_should_be_enchanted(cursed) {
                item.misc_use = -(random_number(3) as i16);
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            } else {
                item.misc_use = 1;
            }
        }
        5 => {
            item.misc_use = (5 * magic_enchantment_bonus(1, 20, level)) as i16;
            item.cost += item.misc_use as i32 * 50;
            if magic_should_be_enchanted(cursed) {
                item.misc_use = -item.misc_use;
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            }
        }
        19 => {
            // Increase damage
            item.to_damage += magic_enchantment_bonus(1, 20, level) as i16;
            item.cost += item.to_damage as i32 * 100;
            if magic_should_be_enchanted(cursed) {
                item.to_damage = -item.to_damage;
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            }
        }
        20 => {
            // Increase To-Hit
            item.to_hit += magic_enchantment_bonus(1, 20, level) as i16;
            item.cost += item.to_hit as i32 * 100;
            if magic_should_be_enchanted(cursed) {
                item.to_hit = -item.to_hit;
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            }
        }
        21 => {
            // Protection
            item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;
            item.cost += item.to_ac as i32 * 100;
            if magic_should_be_enchanted(cursed) {
                item.to_ac = -item.to_ac;
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            }
        }
        24..=29 => {
            item.identification |= config::identification::ID_NO_SHOW_P1;
        }
        30 => {
            // Slaying
            item.identification |= config::identification::ID_SHOW_HIT_DAM;
            item.to_damage += magic_enchantment_bonus(1, 25, level) as i16;
            item.to_hit += magic_enchantment_bonus(1, 25, level) as i16;
            item.cost += (item.to_hit as i32 + item.to_damage as i32) * 100;
            if magic_should_be_enchanted(cursed) {
                item.to_hit = -item.to_hit;
                item.to_damage = -item.to_damage;
                item.flags |= config::treasure::flags::TR_CURSED;
                item.cost = -item.cost;
            }
        }
        _ => {}
    }
}

fn process_amulets(item: &mut Inventory, level: i32, cursed: i32) {
    if item.sub_category_id < 2 {
        if magic_should_be_enchanted(cursed) {
            item.misc_use = -(magic_enchantment_bonus(1, 20, level) as i16);
            item.flags |= config::treasure::flags::TR_CURSED;
            item.cost = -item.cost;
        } else {
            item.misc_use = magic_enchantment_bonus(1, 10, level) as i16;
            item.cost += item.misc_use as i32 * 100;
        }
    } else if item.sub_category_id == 2 {
        item.misc_use = (5 * magic_enchantment_bonus(1, 25, level)) as i16;
        if magic_should_be_enchanted(cursed) {
            item.misc_use = -item.misc_use;
            item.cost = -item.cost;
            item.flags |= config::treasure::flags::TR_CURSED;
        } else {
            item.cost += 50 * item.misc_use as i32;
        }
    } else if item.sub_category_id == 8 {
        // amulet of the magi is never cursed
        item.misc_use = (5 * magic_enchantment_bonus(1, 25, level)) as i16;
        item.cost += 20 * item.misc_use as i32;
    }
}

fn wand_magic(id: u8) -> i32 {
    match id {
        0 => random_number(10) + 6,
        1 => random_number(8) + 6,
        2 => random_number(5) + 6,
        3 => random_number(8) + 6,
        4 => random_number(4) + 3,
        5 => random_number(8) + 6,
        6 | 7 => random_number(20) + 12,
        8 => random_number(10) + 6,
        9 => random_number(12) + 6,
        10 => random_number(10) + 12,
        11 => random_number(3) + 3,
        12 => random_number(8) + 6,
        13 => random_number(10) + 6,
        14 | 15 => random_number(5) + 3,
        16 => random_number(5) + 6,
        17 => random_number(5) + 4,
        18 => random_number(8) + 4,
        19 => random_number(6) + 2,
        20 => random_number(4) + 2,
        21 => random_number(8) + 6,
        22 => random_number(5) + 2,
        23 => random_number(12) + 12,
        _ => -1,
    }
}

fn staff_magic(id: u8) -> i32 {
    match id {
        0 => random_number(20) + 12,
        1 => random_number(8) + 6,
        2 => random_number(5) + 6,
        3 => random_number(20) + 12,
        4 => random_number(15) + 6,
        5 => random_number(4) + 5,
        6 => random_number(5) + 3,
        7 | 8 => random_number(3) + 1,
        9 => random_number(5) + 6,
        10 => random_number(10) + 12,
        11..=13 => random_number(5) + 6,
        14 => random_number(10) + 12,
        15 => random_number(3) + 4,
        16 | 17 => random_number(5) + 6,
        18 => random_number(3) + 4,
        19 => random_number(10) + 12,
        20 | 21 => random_number(3) + 4,
        22 => random_number(10) + 6,
        _ => -1,
    }
}

fn magical_cloak(item: &mut Inventory, special: i32, level: i32) {
    if !magic_should_be_enchanted(special) {
        item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;
        return;
    }

    if random_number(2) == 1 {
        item.special_name_id = SpecialNameIds::SnProtection as u8;
        item.to_ac += magic_enchantment_bonus(2, 40, level) as i16;
        item.cost += 250;
        return;
    }

    item.to_ac += magic_enchantment_bonus(1, 20, level) as i16;
    item.identification |= config::identification::ID_SHOW_P1;
    item.misc_use = random_number(3) as i16;
    item.flags |= config::treasure::flags::TR_STEALTH;
    item.special_name_id = SpecialNameIds::SnStealth as u8;
    item.cost += 500;
}

fn cursed_cloak(item: &mut Inventory, level: i32) {
    let magic_type = random_number(3);

    match magic_type {
        1 => {
            item.flags |= config::treasure::flags::TR_AGGRAVATE;
            item.special_name_id = SpecialNameIds::SnIrritation as u8;
            item.to_ac -= magic_enchantment_bonus(1, 10, level) as i16;
            item.identification |= config::identification::ID_SHOW_HIT_DAM;
            item.to_hit -= magic_enchantment_bonus(1, 10, level) as i16;
            item.to_damage -= magic_enchantment_bonus(1, 10, level) as i16;
            item.cost = 0;
        }
        2 => {
            item.special_name_id = SpecialNameIds::SnVulnerability as u8;
            item.to_ac -= magic_enchantment_bonus(10, 100, level + 50) as i16;
            item.cost = 0;
        }
        _ => {
            item.special_name_id = SpecialNameIds::SnEnveloping as u8;
            item.to_ac -= magic_enchantment_bonus(1, 10, level) as i16;
            item.identification |= config::identification::ID_SHOW_HIT_DAM;
            item.to_hit -= magic_enchantment_bonus(2, 40, level + 10) as i16;
            item.to_damage -= magic_enchantment_bonus(2, 40, level + 10) as i16;
            item.cost = 0;
        }
    }

    item.flags |= config::treasure::flags::TR_CURSED;
}

fn magical_chests(item: &mut Inventory, level: i32) {
    let magic_type = random_number(level + 4);

    match magic_type {
        1 => {
            item.flags = 0;
            item.special_name_id = SpecialNameIds::SnEmpty as u8;
        }
        2 => {
            item.flags |= config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnLocked as u8;
        }
        3 | 4 => {
            item.flags |= config::treasure::chests::CH_LOSE_STR | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnPoisonNeedle as u8;
        }
        5 | 6 => {
            item.flags |= config::treasure::chests::CH_POISON | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnPoisonNeedle as u8;
        }
        7..=9 => {
            item.flags |= config::treasure::chests::CH_PARALYSED | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnGasTrap as u8;
        }
        10 | 11 => {
            item.flags |= config::treasure::chests::CH_EXPLODE | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnExplosionDevice as u8;
        }
        12..=14 => {
            item.flags |= config::treasure::chests::CH_SUMMON | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnSummoningRunes as u8;
        }
        15..=17 => {
            item.flags |= config::treasure::chests::CH_PARALYSED
                | config::treasure::chests::CH_POISON
                | config::treasure::chests::CH_LOSE_STR
                | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnMultipleTraps as u8;
        }
        _ => {
            item.flags |= config::treasure::chests::CH_SUMMON | config::treasure::chests::CH_EXPLODE | config::treasure::chests::CH_LOCKED;
            item.special_name_id = SpecialNameIds::SnMultipleTraps as u8;
        }
    }
}

fn magical_projectile_adjustment(item: &mut Inventory, special: i32, level: i32) {
    item.to_hit += magic_enchantment_bonus(1, 35, level) as i16;
    item.to_damage += magic_enchantment_bonus(1, 35, level) as i16;

    // see comment for weapons
    if magic_should_be_enchanted(3 * special / 2) {
        match random_number(10) {
            1..=3 => {
                item.special_name_id = SpecialNameIds::SnSlaying as u8;
                item.to_hit += 5;
                item.to_damage += 5;
                item.cost += 20;
            }
            4 | 5 => {
                item.flags |= config::treasure::flags::TR_FLAME_TONGUE;
                item.to_hit += 2;
                item.to_damage += 4;
                item.special_name_id = SpecialNameIds::SnFire as u8;
                item.cost += 25;
            }
            6 | 7 => {
                item.flags |= config::treasure::flags::TR_SLAY_EVIL;
                item.to_hit += 3;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnSlayEvil as u8;
                item.cost += 25;
            }
            8 | 9 => {
                item.flags |= config::treasure::flags::TR_SLAY_ANIMAL;
                item.to_hit += 2;
                item.to_damage += 2;
                item.special_name_id = SpecialNameIds::SnSlayAnimal as u8;
                item.cost += 30;
            }
            10 => {
                item.flags |= config::treasure::flags::TR_SLAY_DRAGON;
                item.to_hit += 3;
                item.to_damage += 3;
                item.special_name_id = SpecialNameIds::SnDragonSlaying as u8;
                item.cost += 35;
            }
            _ => {}
        }
    }
}

fn cursed_projectile_adjustment(item: &mut Inventory, level: i32) {
    item.to_hit -= magic_enchantment_bonus(5, 55, level) as i16;
    item.to_damage -= magic_enchantment_bonus(5, 55, level) as i16;
    item.flags |= config::treasure::flags::TR_CURSED;
    item.cost = 0;
}

fn magical_projectile(item: &mut Inventory, special: i32, level: i32, chance: i32, cursed: i32) {
    if item.category_id == TV_SLING_AMMO || item.category_id == TV_BOLT || item.category_id == TV_ARROW {
        // always show to_hit/to_damage values if identified
        item.identification |= config::identification::ID_SHOW_HIT_DAM;

        if magic_should_be_enchanted(chance) {
            magical_projectile_adjustment(item, special, level);
        } else if magic_should_be_enchanted(cursed) {
            cursed_projectile_adjustment(item, level);
        }
    }

    item.items_count = 0;

    for _ in 0..7 {
        item.items_count += random_number(6) as u8;
    }

    let counter = crate::treasure::missiles_counter();
    if *counter == i16::MAX {
        *counter = i16::MIN;
    } else {
        *counter += 1;
    }

    item.misc_use = *counter;
}

// Chance of treasure having magic abilities -RAK-
// Chance increases with each dungeon level
pub fn magic_treasure_magical_ability(item_id: i32, level: i32) {
    let mut chance = config::treasure::OBJECT_BASE_MAGIC as i32 + level;
    if chance > config::treasure::OBJECT_MAX_BASE_MAGIC as i32 {
        chance = config::treasure::OBJECT_MAX_BASE_MAGIC as i32;
    }

    let mut special = chance / config::treasure::OBJECT_CHANCE_SPECIAL as i32;
    let cursed = (10 * chance) / config::treasure::OBJECT_CHANCE_CURSED as i32;

    let item = &mut game().treasure.list[item_id as usize];

    // some objects appear multiple times in the game_objects with different
    // levels, this is to make the object occur more often, however, for
    // consistency, must set the level of these duplicates to be the same
    // as the object with the lowest level

    // Depending on treasure type, it can have certain magical properties
    match item.category_id {
        TV_SHIELD | TV_HARD_ARMOR | TV_SOFT_ARMOR => {
            if magic_should_be_enchanted(chance) {
                magical_armor(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_armor(item, level);
            }
        }
        TV_HAFTED | TV_POLEARM | TV_SWORD => {
            // always show to_hit/to_damage values if identified
            item.identification |= config::identification::ID_SHOW_HIT_DAM;

            if magic_should_be_enchanted(chance) {
                magical_sword(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_sword(item, level);
            }
        }
        TV_BOW => {
            // always show to_hit/to_damage values if identified
            item.identification |= config::identification::ID_SHOW_HIT_DAM;

            if magic_should_be_enchanted(chance) {
                magical_bow(item, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_bow(item, level);
            }
        }
        TV_DIGGING => {
            // always show to_hit/to_damage values if identified
            item.identification |= config::identification::ID_SHOW_HIT_DAM;

            if magic_should_be_enchanted(chance) {
                if random_number(3) < 3 {
                    magical_digging_tool(item, level);
                } else {
                    cursed_digging_tool(item, level);
                }
            }
        }
        TV_GLOVES => {
            if magic_should_be_enchanted(chance) {
                magical_gloves(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_gloves(item, special, level);
            }
        }
        TV_BOOTS => {
            if magic_should_be_enchanted(chance) {
                magical_boots(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_boots(item, level);
            }
        }
        TV_HELM => {
            // give crowns a higher chance for magic
            if item.sub_category_id >= 6 && item.sub_category_id <= 8 {
                chance += item.cost / 100;
                special += special;
            }

            if magic_should_be_enchanted(chance) {
                magical_helms(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_helms(item, special, level);
            }
        }
        TV_RING => process_rings(item, level, cursed),
        TV_AMULET => process_amulets(item, level, cursed),
        TV_LIGHT => {
            // `sub_category_id` should be even for store, odd for dungeon
            // Dungeon found ones will be partially charged
            if (item.sub_category_id % 2) == 1 {
                item.misc_use = random_number(item.misc_use as i32) as i16;
                item.sub_category_id -= 1;
            }
        }
        TV_WAND => {
            let magic_amount = wand_magic(item.sub_category_id);
            if magic_amount != -1 {
                item.misc_use = magic_amount as i16;
            }
        }
        TV_STAFF => {
            let magic_amount = staff_magic(item.sub_category_id);
            if magic_amount != -1 {
                item.misc_use = magic_amount as i16;
            }

            // Change the level the items was first found on value
            if item.sub_category_id == 7 {
                item.depth_first_found = 10;
            } else if item.sub_category_id == 22 {
                item.depth_first_found = 5;
            }
        }
        TV_CLOAK => {
            if magic_should_be_enchanted(chance) {
                magical_cloak(item, special, level);
            } else if magic_should_be_enchanted(cursed) {
                cursed_cloak(item, level);
            }
        }
        TV_CHEST => magical_chests(item, level),
        TV_SLING_AMMO | TV_SPIKE | TV_BOLT | TV_ARROW => {
            magical_projectile(item, special, level, chance, cursed);
        }
        TV_FOOD => {
            // make sure all food rations have the same level
            if item.sub_category_id == 90 {
                item.depth_first_found = 0;
            }

            // give all Elvish waybread the same level
            if item.sub_category_id == 92 {
                item.depth_first_found = 6;
            }
        }
        TV_SCROLL1 => {
            if item.sub_category_id == 67 {
                // give all identify scrolls the same level
                item.depth_first_found = 1;
            } else if item.sub_category_id == 69 {
                // scroll of light
                item.depth_first_found = 0;
            } else if item.sub_category_id == 80 {
                // scroll of trap detection
                item.depth_first_found = 5;
            } else if item.sub_category_id == 81 {
                // scroll of door/stair location
                item.depth_first_found = 5;
            }
        }
        TV_POTION1 => {
            // cure light
            if item.sub_category_id == 76 {
                item.depth_first_found = 0;
            }
        }
        _ => {}
    }
}
