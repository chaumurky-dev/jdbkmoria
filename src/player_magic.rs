// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Player magic functions

use crate::config;
use crate::data_creatures::CREATURES_LIST;
use crate::game::random_number;
use crate::inventory::Inventory;
use crate::player::py;
use crate::recall_data::creature_recall;
use crate::treasure::{TV_ARROW, TV_FLASK, TV_HAFTED, TV_SLING_AMMO, TV_SWORD};

// Cure players confusion -RAK-
pub fn player_cure_confusion() -> bool {
    if py().flags.confused > 1 {
        py().flags.confused = 1;
        return true;
    }
    false
}

// Cure players blindness -RAK-
pub fn player_cure_blindness() -> bool {
    if py().flags.blind > 1 {
        py().flags.blind = 1;
        return true;
    }
    false
}

// Cure poisoning -RAK-
pub fn player_cure_poison() -> bool {
    if py().flags.poisoned > 1 {
        py().flags.poisoned = 1;
        return true;
    }
    false
}

// Cure the players fear -RAK-
pub fn player_remove_fear() -> bool {
    if py().flags.afraid > 1 {
        py().flags.afraid = 1;
        return true;
    }
    false
}

// Evil creatures don't like this. -RAK-
pub fn player_protect_evil() -> bool {
    let is_protected = py().flags.protect_evil == 0;

    py().flags.protect_evil += (random_number(25) + 3 * py().misc.level as i32) as i16;

    is_protected
}

// Bless -RAK-
pub fn player_bless(adjustment: i32) {
    py().flags.blessed += adjustment as i16;
}

// Detect Invisible for period of time -RAK-
pub fn player_detect_invisible(adjustment: i32) {
    py().flags.detect_invisible += adjustment as i16;
}

// Special damage due to magical abilities of object -RAK-
pub fn item_magic_ability_damage(item: &Inventory, total_damage: i32, monster_id: usize) -> i32 {
    let is_ego_weapon = (item.flags & config::treasure::flags::TR_EGO_WEAPON) != 0;
    let is_projectile = item.category_id >= TV_SLING_AMMO && item.category_id <= TV_ARROW;
    let is_hafted_sword = item.category_id >= TV_HAFTED && item.category_id <= TV_SWORD;
    let is_flask = item.category_id == TV_FLASK;

    if is_ego_weapon && (is_projectile || is_hafted_sword || is_flask) {
        let creature = &CREATURES_LIST[monster_id];
        let memory = &mut creature_recall()[monster_id];

        // Slay Dragon
        if (creature.defenses & config::monsters::defense::CD_DRAGON) != 0 && (item.flags & config::treasure::flags::TR_SLAY_DRAGON) != 0 {
            memory.defenses |= config::monsters::defense::CD_DRAGON;
            return total_damage * 4;
        }

        // Slay Undead
        if (creature.defenses & config::monsters::defense::CD_UNDEAD) != 0 && (item.flags & config::treasure::flags::TR_SLAY_UNDEAD) != 0 {
            memory.defenses |= config::monsters::defense::CD_UNDEAD;
            return total_damage * 3;
        }

        // Slay Animal
        if (creature.defenses & config::monsters::defense::CD_ANIMAL) != 0 && (item.flags & config::treasure::flags::TR_SLAY_ANIMAL) != 0 {
            memory.defenses |= config::monsters::defense::CD_ANIMAL;
            return total_damage * 2;
        }

        // Slay Evil
        if (creature.defenses & config::monsters::defense::CD_EVIL) != 0 && (item.flags & config::treasure::flags::TR_SLAY_EVIL) != 0 {
            memory.defenses |= config::monsters::defense::CD_EVIL;
            return total_damage * 2;
        }

        // Frost
        if (creature.defenses & config::monsters::defense::CD_FROST) != 0 && (item.flags & config::treasure::flags::TR_FROST_BRAND) != 0 {
            memory.defenses |= config::monsters::defense::CD_FROST;
            return total_damage * 3 / 2;
        }

        // Fire
        if (creature.defenses & config::monsters::defense::CD_FIRE) != 0 && (item.flags & config::treasure::flags::TR_FLAME_TONGUE) != 0 {
            memory.defenses |= config::monsters::defense::CD_FIRE;
            return total_damage * 3 / 2;
        }
    }

    total_damage
}
