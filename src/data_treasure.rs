// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Treasure data: dungeon object definitions.
//
// See the original umoria `data_treasure.cpp` for a full description of
// every field of the object definitions below.

use crate::dice::Dice;
use crate::dungeon::DungeonObject;
use crate::game::MAX_OBJECTS_IN_GAME;
use crate::identification::SN_ARRAY_SIZE;
use crate::treasure::*;

#[allow(clippy::too_many_arguments)]
const fn o(
    name: &'static str,
    flags: u32,
    category_id: u8,
    sprite: u8,
    misc_use: i16,
    cost: i32,
    sub_category_id: u8,
    items_count: u8,
    weight: u16,
    to_hit: i16,
    to_damage: i16,
    ac: i16,
    to_ac: i16,
    damage: (u8, u8),
    depth_first_found: u8,
) -> DungeonObject {
    DungeonObject {
        name,
        flags,
        category_id,
        sprite,
        misc_use,
        cost,
        sub_category_id,
        items_count,
        weight,
        to_hit,
        to_damage,
        ac,
        to_ac,
        damage: Dice::new(damage.0, damage.1),
        depth_first_found,
    }
}

// Dungeon items from 0 to MAX_DUNGEON_OBJECTS
pub static GAME_OBJECTS: [DungeonObject; MAX_OBJECTS_IN_GAME] = [
    o("Poison", 0x00000001, TV_FOOD, b',', 500, 0, 64, 1, 1, 0, 0, 0, 0, (0, 0), 7), // 0
    o("Blindness", 0x00000002, TV_FOOD, b',', 500, 0, 65, 1, 1, 0, 0, 0, 0, (0, 0), 9), // 1
    o("Paranoia", 0x00000004, TV_FOOD, b',', 500, 0, 66, 1, 1, 0, 0, 0, 0, (0, 0), 9), // 2
    o("Confusion", 0x00000008, TV_FOOD, b',', 500, 0, 67, 1, 1, 0, 0, 0, 0, (0, 0), 7), // 3
    o("Hallucination", 0x00000010, TV_FOOD, b',', 500, 0, 68, 1, 1, 0, 0, 0, 0, (0, 0), 13), // 4
    o("Cure Poison", 0x00000020, TV_FOOD, b',', 500, 60, 69, 1, 1, 0, 0, 0, 0, (0, 0), 8), // 5
    o("Cure Blindness", 0x00000040, TV_FOOD, b',', 500, 50, 70, 1, 1, 0, 0, 0, 0, (0, 0), 10), // 6
    o("Cure Paranoia", 0x00000080, TV_FOOD, b',', 500, 25, 71, 1, 1, 0, 0, 0, 0, (0, 0), 12), // 7
    o("Cure Confusion", 0x00000100, TV_FOOD, b',', 500, 50, 72, 1, 1, 0, 0, 0, 0, (0, 0), 6), // 8
    o("Weakness", 0x04000200, TV_FOOD, b',', 500, 0, 73, 1, 1, 0, 0, 0, 0, (0, 0), 7), // 9
    o("Unhealth", 0x04000400, TV_FOOD, b',', 500, 50, 74, 1, 1, 0, 0, 0, 0, (10, 10), 15), // 10
    o("Restore Constitution", 0x00010000, TV_FOOD, b',', 500, 350, 75, 1, 1, 0, 0, 0, 0, (0, 0), 20), // 11
    o("First-Aid", 0x00200000, TV_FOOD, b',', 500, 5, 76, 1, 1, 0, 0, 0, 0, (0, 0), 6), // 12
    o("Minor Cures", 0x00400000, TV_FOOD, b',', 500, 20, 77, 1, 1, 0, 0, 0, 0, (0, 0), 7), // 13
    o("Light Cures", 0x00800000, TV_FOOD, b',', 500, 30, 78, 1, 1, 0, 0, 0, 0, (0, 0), 10), // 14
    o("Restoration", 0x001F8000, TV_FOOD, b',', 500, 1000, 79, 1, 1, 0, 0, 0, 0, (0, 0), 30), // 15
    o("Poison", 0x00000001, TV_FOOD, b',', 1200, 0, 80, 1, 1, 0, 0, 0, 0, (0, 0), 15), // 16
    o("Hallucination", 0x00000010, TV_FOOD, b',', 1200, 0, 81, 1, 1, 0, 0, 0, 0, (0, 0), 18), // 17
    o("Cure Poison", 0x00000020, TV_FOOD, b',', 1200, 75, 82, 1, 1, 0, 0, 0, 0, (0, 0), 19), // 18
    o("Unhealth", 0x04000400, TV_FOOD, b',', 1200, 75, 83, 1, 1, 0, 0, 0, 0, (10, 12), 28), // 19
    o("Major Cures", 0x02000000, TV_FOOD, b',', 1200, 75, 84, 1, 2, 0, 0, 0, 0, (0, 0), 16), // 20
    o("& Ration~ of Food", 0x00000000, TV_FOOD, b',', 5000, 3, 90, 1, 10, 0, 0, 0, 0, (0, 0), 0), // 21
    o("& Ration~ of Food", 0x00000000, TV_FOOD, b',', 5000, 3, 90, 1, 10, 0, 0, 0, 0, (0, 0), 5), // 22
    o("& Ration~ of Food", 0x00000000, TV_FOOD, b',', 5000, 3, 90, 1, 10, 0, 0, 0, 0, (0, 0), 10), // 23
    o("& Slime Mold~", 0x00000000, TV_FOOD, b',', 3000, 2, 91, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 24
    o("& Piece~ of Elvish Waybread", 0x02000020, TV_FOOD, b',', 7500, 25, 92, 1, 3, 0, 0, 0, 0, (0, 0), 6), // 25
    o("& Piece~ of Elvish Waybread", 0x02000020, TV_FOOD, b',', 7500, 25, 92, 1, 3, 0, 0, 0, 0, (0, 0), 12), // 26
    o("& Piece~ of Elvish Waybread", 0x02000020, TV_FOOD, b',', 7500, 25, 92, 1, 3, 0, 0, 0, 0, (0, 0), 20), // 27
    o("& Dagger (Main Gauche)", 0x00000000, TV_SWORD, b'|', 0, 25, 1, 1, 30, 0, 0, 0, 0, (1, 5), 2), // 28
    o("& Dagger (Misericorde)", 0x00000000, TV_SWORD, b'|', 0, 10, 2, 1, 15, 0, 0, 0, 0, (1, 4), 0), // 29
    o("& Dagger (Stiletto)", 0x00000000, TV_SWORD, b'|', 0, 10, 3, 1, 12, 0, 0, 0, 0, (1, 4), 0), // 30
    o("& Dagger (Bodkin)", 0x00000000, TV_SWORD, b'|', 0, 10, 4, 1, 20, 0, 0, 0, 0, (1, 4), 1), // 31
    o("& Broken Dagger", 0x00000000, TV_SWORD, b'|', 0, 0, 5, 1, 15, -2, -2, 0, 0, (1, 1), 0), // 32
    o("& Backsword", 0x00000000, TV_SWORD, b'|', 0, 150, 6, 1, 95, 0, 0, 0, 0, (1, 9), 7), // 33
    o("& Bastard Sword", 0x00000000, TV_SWORD, b'|', 0, 350, 7, 1, 140, 0, 0, 0, 0, (3, 4), 14), // 34
    o("& Thrusting Sword (Bilbo)", 0x00000000, TV_SWORD, b'|', 0, 60, 8, 1, 80, 0, 0, 0, 0, (1, 6), 4), // 35
    o("& Thrusting Sword (Baselard)", 0x00000000, TV_SWORD, b'|', 0, 80, 9, 1, 100, 0, 0, 0, 0, (1, 7), 5), // 36
    o("& Broadsword", 0x00000000, TV_SWORD, b'|', 0, 255, 10, 1, 150, 0, 0, 0, 0, (2, 5), 9), // 37
    o("& Two-Handed Sword (Claymore)", 0x00000000, TV_SWORD, b'|', 0, 775, 11, 1, 200, 0, 0, 0, 0, (3, 6), 30), // 38
    o("& Cutlass", 0x00000000, TV_SWORD, b'|', 0, 85, 12, 1, 110, 0, 0, 0, 0, (1, 7), 7), // 39
    o("& Two-Handed Sword (Espadon)", 0x00000000, TV_SWORD, b'|', 0, 655, 13, 1, 180, 0, 0, 0, 0, (3, 6), 35), // 40
    o("& Executioner's Sword", 0x00000000, TV_SWORD, b'|', 0, 850, 14, 1, 260, 0, 0, 0, 0, (4, 5), 40), // 41
    o("& Two-Handed Sword (Flamberge)", 0x00000000, TV_SWORD, b'|', 0, 1000, 15, 1, 240, 0, 0, 0, 0, (4, 5), 45), // 42
    o("& Foil", 0x00000000, TV_SWORD, b'|', 0, 35, 16, 1, 30, 0, 0, 0, 0, (1, 5), 2), // 43
    o("& Katana", 0x00000000, TV_SWORD, b'|', 0, 400, 17, 1, 120, 0, 0, 0, 0, (3, 4), 18), // 44
    o("& Longsword", 0x00000000, TV_SWORD, b'|', 0, 200, 18, 1, 130, 0, 0, 0, 0, (1, 10), 12), // 45
    o("& Two-Handed Sword (No-Dachi)", 0x00000000, TV_SWORD, b'|', 0, 675, 19, 1, 200, 0, 0, 0, 0, (4, 4), 45), // 46
    o("& Rapier", 0x00000000, TV_SWORD, b'|', 0, 42, 20, 1, 40, 0, 0, 0, 0, (1, 6), 4), // 47
    o("& Sabre", 0x00000000, TV_SWORD, b'|', 0, 50, 21, 1, 50, 0, 0, 0, 0, (1, 7), 5), // 48
    o("& Small Sword", 0x00000000, TV_SWORD, b'|', 0, 48, 22, 1, 75, 0, 0, 0, 0, (1, 6), 5), // 49
    o("& Two-Handed Sword (Zweihander)", 0x00000000, TV_SWORD, b'|', 0, 1500, 23, 1, 280, 0, 0, 0, 0, (4, 6), 50), // 50
    o("& Broken Sword", 0x00000000, TV_SWORD, b'|', 0, 0, 24, 1, 75, -2, -2, 0, 0, (1, 1), 0), // 51
    o("& Ball and Chain", 0x00000000, TV_HAFTED, b'\\', 0, 200, 1, 1, 150, 0, 0, 0, 0, (2, 4), 20), // 52
    o("& Cat-o'-Nine-Tails", 0x00000000, TV_HAFTED, b'\\', 0, 14, 2, 1, 40, 0, 0, 0, 0, (1, 4), 3), // 53
    o("& Wooden Club", 0x00000000, TV_HAFTED, b'\\', 0, 10, 3, 1, 100, 0, 0, 0, 0, (1, 3), 0), // 54
    o("& Flail", 0x00000000, TV_HAFTED, b'\\', 0, 353, 4, 1, 150, 0, 0, 0, 0, (2, 6), 12), // 55
    o("& Two-Handed Great Flail", 0x00000000, TV_HAFTED, b'\\', 0, 590, 5, 1, 280, 0, 0, 0, 0, (3, 6), 45), // 56
    o("& Morningstar", 0x00000000, TV_HAFTED, b'\\', 0, 396, 6, 1, 150, 0, 0, 0, 0, (2, 6), 10), // 57
    o("& Mace", 0x00000000, TV_HAFTED, b'\\', 0, 130, 7, 1, 120, 0, 0, 0, 0, (2, 4), 6), // 58
    o("& War Hammer", 0x00000000, TV_HAFTED, b'\\', 0, 225, 8, 1, 120, 0, 0, 0, 0, (3, 3), 5), // 59
    o("& Lead-Filled Mace", 0x00000000, TV_HAFTED, b'\\', 0, 502, 9, 1, 180, 0, 0, 0, 0, (3, 4), 15), // 60
    o("& Awl-Pike", 0x00000000, TV_POLEARM, b'/', 0, 200, 1, 1, 160, 0, 0, 0, 0, (1, 8), 8), // 61
    o("& Beaked Axe", 0x00000000, TV_POLEARM, b'/', 0, 408, 2, 1, 180, 0, 0, 0, 0, (2, 6), 15), // 62
    o("& Fauchard", 0x00000000, TV_POLEARM, b'/', 0, 326, 3, 1, 170, 0, 0, 0, 0, (1, 10), 17), // 63
    o("& Glaive", 0x00000000, TV_POLEARM, b'/', 0, 363, 4, 1, 190, 0, 0, 0, 0, (2, 6), 20), // 64
    o("& Halberd", 0x00000000, TV_POLEARM, b'/', 0, 430, 5, 1, 190, 0, 0, 0, 0, (3, 4), 22), // 65
    o("& Lucerne Hammer", 0x00000000, TV_POLEARM, b'/', 0, 376, 6, 1, 120, 0, 0, 0, 0, (2, 5), 11), // 66
    o("& Pike", 0x00000000, TV_POLEARM, b'/', 0, 358, 7, 1, 160, 0, 0, 0, 0, (2, 5), 15), // 67
    o("& Spear", 0x00000000, TV_POLEARM, b'/', 0, 36, 8, 1, 50, 0, 0, 0, 0, (1, 6), 5), // 68
    o("& Lance", 0x00000000, TV_POLEARM, b'/', 0, 230, 9, 1, 300, 0, 0, 0, 0, (2, 8), 10), // 69
    o("& Javelin", 0x00000000, TV_POLEARM, b'/', 0, 18, 10, 1, 30, 0, 0, 0, 0, (1, 4), 4), // 70
    o("& Battle Axe (Balestarius)", 0x00000000, TV_POLEARM, b'/', 0, 500, 11, 1, 180, 0, 0, 0, 0, (2, 8), 30), // 71
    o("& Battle Axe (European)", 0x00000000, TV_POLEARM, b'/', 0, 334, 12, 1, 170, 0, 0, 0, 0, (3, 4), 13), // 72
    o("& Broad Axe", 0x00000000, TV_POLEARM, b'/', 0, 304, 13, 1, 160, 0, 0, 0, 0, (2, 6), 17), // 73
    o("& Short Bow", 0x00000000, TV_BOW, b'}', 2, 50, 1, 1, 30, 0, 0, 0, 0, (0, 0), 3), // 74
    o("& Long Bow", 0x00000000, TV_BOW, b'}', 3, 120, 2, 1, 40, 0, 0, 0, 0, (0, 0), 10), // 75
    o("& Composite Bow", 0x00000000, TV_BOW, b'}', 4, 240, 3, 1, 40, 0, 0, 0, 0, (0, 0), 40), // 76
    o("& Light Crossbow", 0x00000000, TV_BOW, b'}', 5, 140, 10, 1, 110, 0, 0, 0, 0, (0, 0), 15), // 77
    o("& Heavy Crossbow", 0x00000000, TV_BOW, b'}', 6, 300, 11, 1, 200, 0, 0, 0, 0, (1, 1), 30), // 78
    o("& Sling", 0x00000000, TV_BOW, b'}', 1, 5, 20, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 79
    o("& Arrow~", 0x00000000, TV_ARROW, b'{', 0, 1, 193, 1, 2, 0, 0, 0, 0, (1, 4), 2), // 80
    o("& Bolt~", 0x00000000, TV_BOLT, b'{', 0, 2, 193, 1, 3, 0, 0, 0, 0, (1, 5), 2), // 81
    o("& Rounded Pebble~", 0x00000000, TV_SLING_AMMO, b'{', 0, 1, 193, 1, 4, 0, 0, 0, 0, (1, 2), 0), // 82
    o("& Iron Shot~", 0x00000000, TV_SLING_AMMO, b'{', 0, 2, 194, 1, 5, 0, 0, 0, 0, (1, 3), 3), // 83
    o("& Iron Spike~", 0x00000000, TV_SPIKE, b'~', 0, 1, 193, 1, 10, 0, 0, 0, 0, (1, 1), 1), // 84
    o("& Brass Lantern~", 0x00000000, TV_LIGHT, b'~', 7500, 35, 1, 1, 50, 0, 0, 0, 0, (1, 1), 1), // 85
    o("& Wooden Torch~", 0x00000000, TV_LIGHT, b'~', 4000, 2, 193, 1, 30, 0, 0, 0, 0, (1, 1), 1), // 86
    o("& Orcish Pick", 0x20000000, TV_DIGGING, b'\\', 2, 500, 2, 1, 180, 0, 0, 0, 0, (1, 3), 20), // 87
    o("& Dwarven Pick", 0x20000000, TV_DIGGING, b'\\', 3, 1200, 3, 1, 200, 0, 0, 0, 0, (1, 4), 50), // 88
    o("& Gnomish Shovel", 0x20000000, TV_DIGGING, b'\\', 1, 100, 5, 1, 50, 0, 0, 0, 0, (1, 2), 20), // 89
    o("& Dwarven Shovel", 0x20000000, TV_DIGGING, b'\\', 2, 250, 6, 1, 120, 0, 0, 0, 0, (1, 3), 40), // 90
    o("& Pair of Soft Leather Shoes", 0x00000000, TV_BOOTS, b']', 0, 4, 1, 1, 5, 0, 0, 1, 0, (0, 0), 1), // 91
    o("& Pair of Soft Leather Boots", 0x00000000, TV_BOOTS, b']', 0, 7, 2, 1, 20, 0, 0, 2, 0, (1, 1), 4), // 92
    o("& Pair of Hard Leather Boots", 0x00000000, TV_BOOTS, b']', 0, 12, 3, 1, 40, 0, 0, 3, 0, (1, 1), 6), // 93
    o("& Soft Leather Cap", 0x00000000, TV_HELM, b']', 0, 4, 1, 1, 10, 0, 0, 1, 0, (0, 0), 2), // 94
    o("& Hard Leather Cap", 0x00000000, TV_HELM, b']', 0, 12, 2, 1, 15, 0, 0, 2, 0, (0, 0), 4), // 95
    o("& Metal Cap", 0x00000000, TV_HELM, b']', 0, 30, 3, 1, 20, 0, 0, 3, 0, (1, 1), 7), // 96
    o("& Iron Helm", 0x00000000, TV_HELM, b']', 0, 75, 4, 1, 75, 0, 0, 5, 0, (1, 3), 20), // 97
    o("& Steel Helm", 0x00000000, TV_HELM, b']', 0, 200, 5, 1, 60, 0, 0, 6, 0, (1, 3), 40), // 98
    o("& Silver Crown", 0x00000000, TV_HELM, b']', 0, 500, 6, 1, 20, 0, 0, 0, 0, (1, 1), 44), // 99
    o("& Golden Crown", 0x00000000, TV_HELM, b']', 0, 1000, 7, 1, 30, 0, 0, 0, 0, (1, 2), 47), // 100
    o("& Jewel-Encrusted Crown", 0x00000000, TV_HELM, b']', 0, 2000, 8, 1, 40, 0, 0, 0, 0, (1, 3), 50), // 101
    o("& Robe", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 4, 1, 1, 20, 0, 0, 2, 0, (0, 0), 1), // 102
    o("Soft Leather Armor", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 18, 2, 1, 80, 0, 0, 4, 0, (0, 0), 2), // 103
    o("Soft Studded Leather", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 35, 3, 1, 90, 0, 0, 5, 0, (1, 1), 3), // 104
    o("Hard Leather Armor", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 55, 4, 1, 100, -1, 0, 6, 0, (1, 1), 5), // 105
    o("Hard Studded Leather", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 100, 5, 1, 110, -1, 0, 7, 0, (1, 2), 7), // 106
    o("Woven Cord Armor", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 45, 6, 1, 150, -1, 0, 6, 0, (0, 0), 7), // 107
    o("Soft Leather Ring Mail", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 160, 7, 1, 130, -1, 0, 6, 0, (1, 2), 10), // 108
    o("Hard Leather Ring Mail", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 230, 8, 1, 150, -2, 0, 8, 0, (1, 3), 12), // 109
    o("Leather Scale Mail", 0x00000000, TV_SOFT_ARMOR, b'(', 0, 330, 9, 1, 140, -1, 0, 11, 0, (1, 1), 14), // 110
    o("Metal Scale Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 430, 1, 1, 250, -2, 0, 13, 0, (1, 4), 24), // 111
    o("Chain Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 530, 2, 1, 220, -2, 0, 14, 0, (1, 4), 26), // 112
    o("Rusty Chain Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 0, 3, 1, 220, -5, 0, 14, -8, (1, 4), 26), // 113
    o("Double Chain Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 630, 4, 1, 260, -2, 0, 15, 0, (1, 4), 28), // 114
    o("Augmented Chain Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 675, 5, 1, 270, -2, 0, 16, 0, (1, 4), 30), // 115
    o("Bar Chain Mail", 0x00000000, TV_HARD_ARMOR, b'[', 0, 720, 6, 1, 280, -2, 0, 18, 0, (1, 4), 34), // 116
    o("Metal Brigandine Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 775, 7, 1, 290, -3, 0, 19, 0, (1, 4), 36), // 117
    o("Laminated Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 825, 8, 1, 300, -3, 0, 20, 0, (1, 4), 38), // 118
    o("Partial Plate Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 900, 9, 1, 320, -3, 0, 22, 0, (1, 6), 42), // 119
    o("Metal Lamellar Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 950, 10, 1, 340, -3, 0, 23, 0, (1, 6), 44), // 120
    o("Full Plate Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 1050, 11, 1, 380, -3, 0, 25, 0, (2, 4), 48), // 121
    o("Ribbed Plate Armor", 0x00000000, TV_HARD_ARMOR, b'[', 0, 1200, 12, 1, 380, -3, 0, 28, 0, (2, 4), 50), // 122
    o("& Cloak", 0x00000000, TV_CLOAK, b'(', 0, 3, 1, 1, 10, 0, 0, 1, 0, (0, 0), 1), // 123
    o("& Set of Leather Gloves", 0x00000000, TV_GLOVES, b']', 0, 3, 1, 1, 5, 0, 0, 1, 0, (0, 0), 1), // 124
    o("& Set of Gauntlets", 0x00000000, TV_GLOVES, b']', 0, 35, 2, 1, 25, 0, 0, 2, 0, (1, 1), 12), // 125
    o("& Small Leather Shield", 0x00000000, TV_SHIELD, b')', 0, 30, 1, 1, 50, 0, 0, 2, 0, (1, 1), 3), // 126
    o("& Medium Leather Shield", 0x00000000, TV_SHIELD, b')', 0, 60, 2, 1, 75, 0, 0, 3, 0, (1, 2), 8), // 127
    o("& Large Leather Shield", 0x00000000, TV_SHIELD, b')', 0, 120, 3, 1, 100, 0, 0, 4, 0, (1, 2), 15), // 128
    o("& Small Metal Shield", 0x00000000, TV_SHIELD, b')', 0, 50, 4, 1, 65, 0, 0, 3, 0, (1, 2), 10), // 129
    o("& Medium Metal Shield", 0x00000000, TV_SHIELD, b')', 0, 125, 5, 1, 90, 0, 0, 4, 0, (1, 3), 20), // 130
    o("& Large Metal Shield", 0x00000000, TV_SHIELD, b')', 0, 200, 6, 1, 120, 0, 0, 5, 0, (1, 3), 30), // 131
    o("Strength", 0x00000001, TV_RING, b'=', 0, 400, 0, 1, 2, 0, 0, 0, 0, (0, 0), 30), // 132
    o("Dexterity", 0x00000008, TV_RING, b'=', 0, 400, 1, 1, 2, 0, 0, 0, 0, (0, 0), 30), // 133
    o("Constitution", 0x00000010, TV_RING, b'=', 0, 400, 2, 1, 2, 0, 0, 0, 0, (0, 0), 30), // 134
    o("Intelligence", 0x00000002, TV_RING, b'=', 0, 400, 3, 1, 2, 0, 0, 0, 0, (0, 0), 30), // 135
    o("Speed", 0x00001000, TV_RING, b'=', 0, 3000, 4, 1, 2, 0, 0, 0, 0, (0, 0), 50), // 136
    o("Searching", 0x00000040, TV_RING, b'=', 0, 250, 5, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 137
    o("Teleportation", 0x80000400, TV_RING, b'=', 0, 0, 6, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 138
    o("Slow Digestion", 0x00000080, TV_RING, b'=', 0, 200, 7, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 139
    o("Resist Fire", 0x00080000, TV_RING, b'=', 0, 250, 8, 1, 2, 0, 0, 0, 0, (0, 0), 14), // 140
    o("Resist Cold", 0x00200000, TV_RING, b'=', 0, 250, 9, 1, 2, 0, 0, 0, 0, (0, 0), 14), // 141
    o("Feather Falling", 0x04000000, TV_RING, b'=', 0, 200, 10, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 142
    o("Adornment", 0x00000000, TV_RING, b'=', 0, 20, 11, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 143
    // was a ring of adornment, sub_category_id = 12 here
    o("& Arrow~", 0x00000000, TV_ARROW, b'{', 0, 1, 193, 1, 2, 0, 0, 0, 0, (1, 4), 15), // 144
    o("Weakness", 0x80000001, TV_RING, b'=', -5, 0, 13, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 145
    o("Lordly Protection (FIRE)", 0x00080000, TV_RING, b'=', 0, 1200, 14, 1, 2, 0, 0, 0, 5, (0, 0), 50), // 146
    o("Lordly Protection (ACID)", 0x00100000, TV_RING, b'=', 0, 1200, 15, 1, 2, 0, 0, 0, 5, (0, 0), 50), // 147
    o("Lordly Protection (COLD)", 0x00200000, TV_RING, b'=', 0, 1200, 16, 1, 2, 0, 0, 0, 5, (0, 0), 50), // 148
    o("WOE", 0x80000644, TV_RING, b'=', -5, 0, 17, 1, 2, 0, 0, 0, -3, (0, 0), 50), // 149
    o("Stupidity", 0x80000002, TV_RING, b'=', -5, 0, 18, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 150
    o("Increase Damage", 0x00000000, TV_RING, b'=', 0, 100, 19, 1, 2, 0, 0, 0, 0, (0, 0), 20), // 151
    o("Increase To-Hit", 0x00000000, TV_RING, b'=', 0, 100, 20, 1, 2, 0, 0, 0, 0, (0, 0), 20), // 152
    o("Protection", 0x00000000, TV_RING, b'=', 0, 100, 21, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 153
    o("Aggravate Monster", 0x80000200, TV_RING, b'=', 0, 0, 22, 1, 2, 0, 0, 0, 0, (0, 0), 7), // 154
    o("See Invisible", 0x01000000, TV_RING, b'=', 0, 500, 23, 1, 2, 0, 0, 0, 0, (0, 0), 40), // 155
    o("Sustain Strength", 0x00400000, TV_RING, b'=', 1, 750, 24, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 156
    o("Sustain Intelligence", 0x00400000, TV_RING, b'=', 2, 600, 25, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 157
    o("Sustain Wisdom", 0x00400000, TV_RING, b'=', 3, 600, 26, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 158
    o("Sustain Constitution", 0x00400000, TV_RING, b'=', 4, 750, 27, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 159
    o("Sustain Dexterity", 0x00400000, TV_RING, b'=', 5, 750, 28, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 160
    o("Sustain Charisma", 0x00400000, TV_RING, b'=', 6, 500, 29, 1, 2, 0, 0, 0, 0, (0, 0), 44), // 161
    o("Slaying", 0x00000000, TV_RING, b'=', 0, 1000, 30, 1, 2, 0, 0, 0, 0, (0, 0), 50), // 162
    o("Wisdom", 0x00000004, TV_AMULET, b'"', 0, 300, 0, 1, 3, 0, 0, 0, 0, (0, 0), 20), // 163
    o("Charisma", 0x00000020, TV_AMULET, b'"', 0, 250, 1, 1, 3, 0, 0, 0, 0, (0, 0), 20), // 164
    o("Searching", 0x00000040, TV_AMULET, b'"', 0, 250, 2, 1, 3, 0, 0, 0, 0, (0, 0), 14), // 165
    o("Teleportation", 0x80000400, TV_AMULET, b'"', 0, 0, 3, 1, 3, 0, 0, 0, 0, (0, 0), 14), // 166
    o("Slow Digestion", 0x00000080, TV_AMULET, b'"', 0, 200, 4, 1, 3, 0, 0, 0, 0, (0, 0), 14), // 167
    o("Resist Acid", 0x00100000, TV_AMULET, b'"', 0, 250, 5, 1, 3, 0, 0, 0, 0, (0, 0), 24), // 168
    o("Adornment", 0x00000000, TV_AMULET, b'"', 0, 20, 6, 1, 3, 0, 0, 0, 0, (0, 0), 16), // 169
    // was an amulet of adornment here, sub_category_id = 7
    o("& Bolt~", 0x00000000, TV_BOLT, b'{', 0, 2, 193, 1, 3, 0, 0, 0, 0, (1, 5), 25), // 170
    o("the Magi", 0x01800040, TV_AMULET, b'"', 0, 5000, 8, 1, 3, 0, 0, 0, 3, (0, 0), 50), // 171
    o("DOOM", 0x8000007F, TV_AMULET, b'"', -5, 0, 9, 1, 3, 0, 0, 0, 0, (0, 0), 50), // 172
    o("Enchant Weapon To-Hit", 0x00000001, TV_SCROLL1, b'?', 0, 125, 64, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 173
    o("Enchant Weapon To-Dam", 0x00000002, TV_SCROLL1, b'?', 0, 125, 65, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 174
    o("Enchant Armor", 0x00000004, TV_SCROLL1, b'?', 0, 125, 66, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 175
    o("Identify", 0x00000008, TV_SCROLL1, b'?', 0, 50, 67, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 176
    o("Identify", 0x00000008, TV_SCROLL1, b'?', 0, 50, 67, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 177
    o("Identify", 0x00000008, TV_SCROLL1, b'?', 0, 50, 67, 1, 5, 0, 0, 0, 0, (0, 0), 10), // 178
    o("Identify", 0x00000008, TV_SCROLL1, b'?', 0, 50, 67, 1, 5, 0, 0, 0, 0, (0, 0), 30), // 179
    o("Remove Curse", 0x00000010, TV_SCROLL1, b'?', 0, 100, 68, 1, 5, 0, 0, 0, 0, (0, 0), 7), // 180
    o("Light", 0x00000020, TV_SCROLL1, b'?', 0, 15, 69, 1, 5, 0, 0, 0, 0, (0, 0), 0), // 181
    o("Light", 0x00000020, TV_SCROLL1, b'?', 0, 15, 69, 1, 5, 0, 0, 0, 0, (0, 0), 3), // 182
    o("Light", 0x00000020, TV_SCROLL1, b'?', 0, 15, 69, 1, 5, 0, 0, 0, 0, (0, 0), 7), // 183
    o("Summon Monster", 0x00000040, TV_SCROLL1, b'?', 0, 0, 70, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 184
    o("Phase Door", 0x00000080, TV_SCROLL1, b'?', 0, 15, 71, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 185
    o("Teleport", 0x00000100, TV_SCROLL1, b'?', 0, 40, 72, 1, 5, 0, 0, 0, 0, (0, 0), 10), // 186
    o("Teleport Level", 0x00000200, TV_SCROLL1, b'?', 0, 50, 73, 1, 5, 0, 0, 0, 0, (0, 0), 20), // 187
    o("Monster Confusion", 0x00000400, TV_SCROLL1, b'?', 0, 30, 74, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 188
    o("Magic Mapping", 0x00000800, TV_SCROLL1, b'?', 0, 40, 75, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 189
    o("Sleep Monster", 0x00001000, TV_SCROLL1, b'?', 0, 35, 76, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 190
    o("Rune of Protection", 0x00002000, TV_SCROLL1, b'?', 0, 500, 77, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 191
    o("Treasure Detection", 0x00004000, TV_SCROLL1, b'?', 0, 15, 78, 1, 5, 0, 0, 0, 0, (0, 0), 0), // 192
    o("Object Detection", 0x00008000, TV_SCROLL1, b'?', 0, 15, 79, 1, 5, 0, 0, 0, 0, (0, 0), 0), // 193
    o("Trap Detection", 0x00010000, TV_SCROLL1, b'?', 0, 35, 80, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 194
    o("Trap Detection", 0x00010000, TV_SCROLL1, b'?', 0, 35, 80, 1, 5, 0, 0, 0, 0, (0, 0), 8), // 195
    o("Trap Detection", 0x00010000, TV_SCROLL1, b'?', 0, 35, 80, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 196
    o("Door/Stair Location", 0x00020000, TV_SCROLL1, b'?', 0, 35, 81, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 197
    o("Door/Stair Location", 0x00020000, TV_SCROLL1, b'?', 0, 35, 81, 1, 5, 0, 0, 0, 0, (0, 0), 10), // 198
    o("Door/Stair Location", 0x00020000, TV_SCROLL1, b'?', 0, 35, 81, 1, 5, 0, 0, 0, 0, (0, 0), 15), // 199
    o("Mass Genocide", 0x00040000, TV_SCROLL1, b'?', 0, 1000, 82, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 200
    o("Detect Invisible", 0x00080000, TV_SCROLL1, b'?', 0, 15, 83, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 201
    o("Aggravate Monster", 0x00100000, TV_SCROLL1, b'?', 0, 0, 84, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 202
    o("Trap Creation", 0x00200000, TV_SCROLL1, b'?', 0, 0, 85, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 203
    o("Trap/Door Destruction", 0x00400000, TV_SCROLL1, b'?', 0, 50, 86, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 204
    o("Door Creation", 0x00800000, TV_SCROLL1, b'?', 0, 100, 87, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 205
    o("Recharging", 0x01000000, TV_SCROLL1, b'?', 0, 200, 88, 1, 5, 0, 0, 0, 0, (0, 0), 40), // 206
    o("Genocide", 0x02000000, TV_SCROLL1, b'?', 0, 750, 89, 1, 5, 0, 0, 0, 0, (0, 0), 35), // 207
    o("Darkness", 0x04000000, TV_SCROLL1, b'?', 0, 0, 90, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 208
    o("Protection from Evil", 0x08000000, TV_SCROLL1, b'?', 0, 100, 91, 1, 5, 0, 0, 0, 0, (0, 0), 30), // 209
    o("Create Food", 0x10000000, TV_SCROLL1, b'?', 0, 10, 92, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 210
    o("Dispel Undead", 0x20000000, TV_SCROLL1, b'?', 0, 200, 93, 1, 5, 0, 0, 0, 0, (0, 0), 40), // 211
    o("*Enchant Weapon*", 0x00000001, TV_SCROLL2, b'?', 0, 500, 94, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 212
    o("Curse Weapon", 0x00000002, TV_SCROLL2, b'?', 0, 0, 95, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 213
    o("*Enchant Armor*", 0x00000004, TV_SCROLL2, b'?', 0, 500, 96, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 214
    o("Curse Armor", 0x00000008, TV_SCROLL2, b'?', 0, 0, 97, 1, 5, 0, 0, 0, 0, (0, 0), 50), // 215
    o("Summon Undead", 0x00000010, TV_SCROLL2, b'?', 0, 0, 98, 1, 5, 0, 0, 0, 0, (0, 0), 15), // 216
    o("Blessing", 0x00000020, TV_SCROLL2, b'?', 0, 15, 99, 1, 5, 0, 0, 0, 0, (0, 0), 1), // 217
    o("Holy Chant", 0x00000040, TV_SCROLL2, b'?', 0, 40, 100, 1, 5, 0, 0, 0, 0, (0, 0), 12), // 218
    o("Holy Prayer", 0x00000080, TV_SCROLL2, b'?', 0, 80, 101, 1, 5, 0, 0, 0, 0, (0, 0), 24), // 219
    o("Word-of-Recall", 0x00000100, TV_SCROLL2, b'?', 0, 150, 102, 1, 5, 0, 0, 0, 0, (0, 0), 5), // 220
    o("*Destruction*", 0x00000200, TV_SCROLL2, b'?', 0, 750, 103, 1, 5, 0, 0, 0, 0, (0, 0), 40), // 221
    // SMJ, AJ, Water must be sub_category_id 64-66 resp. for itemDescription to work
    o("Slime Mold Juice", 0x30000000, TV_POTION1, b'!', 400, 2, 64, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 222
    o("Apple Juice", 0x00000000, TV_POTION1, b'!', 250, 1, 65, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 223
    o("Water", 0x00000000, TV_POTION1, b'!', 200, 0, 66, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 224
    o("Strength", 0x00000001, TV_POTION1, b'!', 50, 300, 67, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 225
    o("Weakness", 0x00000002, TV_POTION1, b'!', 0, 0, 68, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 226
    o("Restore Strength", 0x00000004, TV_POTION1, b'!', 0, 300, 69, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 227
    o("Intelligence", 0x00000008, TV_POTION1, b'!', 0, 300, 70, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 228
    o("Lose Intelligence", 0x00000010, TV_POTION1, b'!', 0, 0, 71, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 229
    o("Restore Intelligence", 0x00000020, TV_POTION1, b'!', 0, 300, 72, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 230
    o("Wisdom", 0x00000040, TV_POTION1, b'!', 0, 300, 73, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 231
    o("Lose Wisdom", 0x00000080, TV_POTION1, b'!', 0, 0, 74, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 232
    o("Restore Wisdom", 0x00000100, TV_POTION1, b'!', 0, 300, 75, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 233
    o("Charisma", 0x00000200, TV_POTION1, b'!', 0, 300, 76, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 234
    o("Ugliness", 0x00000400, TV_POTION1, b'!', 0, 0, 77, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 235
    o("Restore Charisma", 0x00000800, TV_POTION1, b'!', 0, 300, 78, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 236
    o("Cure Light Wounds", 0x10001000, TV_POTION1, b'!', 50, 15, 79, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 237
    o("Cure Light Wounds", 0x10001000, TV_POTION1, b'!', 50, 15, 79, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 238
    o("Cure Light Wounds", 0x10001000, TV_POTION1, b'!', 50, 15, 79, 1, 4, 0, 0, 0, 0, (1, 1), 2), // 239
    o("Cure Serious Wounds", 0x30002000, TV_POTION1, b'!', 100, 40, 80, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 240
    o("Cure Critical Wounds", 0x70004000, TV_POTION1, b'!', 100, 100, 81, 1, 4, 0, 0, 0, 0, (1, 1), 5), // 241
    o("Healing", 0x70008000, TV_POTION1, b'!', 200, 200, 82, 1, 4, 0, 0, 0, 0, (1, 1), 12), // 242
    o("Constitution", 0x00010000, TV_POTION1, b'!', 50, 300, 83, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 243
    o("Gain Experience", 0x00020000, TV_POTION1, b'!', 0, 2500, 84, 1, 4, 0, 0, 0, 0, (1, 1), 50), // 244
    o("Sleep", 0x00040000, TV_POTION1, b'!', 100, 0, 85, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 245
    o("Blindness", 0x00080000, TV_POTION1, b'!', 0, 0, 86, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 246
    o("Confusion", 0x00100000, TV_POTION1, b'!', 50, 0, 87, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 247
    o("Poison", 0x00200000, TV_POTION1, b'!', 0, 0, 88, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 248
    o("Haste Self", 0x00400000, TV_POTION1, b'!', 0, 75, 89, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 249
    o("Slowness", 0x00800000, TV_POTION1, b'!', 50, 0, 90, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 250
    o("Dexterity", 0x02000000, TV_POTION1, b'!', 0, 300, 91, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 251
    o("Restore Dexterity", 0x04000000, TV_POTION1, b'!', 0, 300, 92, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 252
    o("Restore Constitution", 0x68000000, TV_POTION1, b'!', 0, 300, 93, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 253
    o("Lose Experience", 0x00000002, TV_POTION2, b'!', 0, 0, 95, 1, 4, 0, 0, 0, 0, (1, 1), 10), // 254
    o("Salt Water", 0x00000004, TV_POTION2, b'!', 0, 0, 96, 1, 4, 0, 0, 0, 0, (1, 1), 0), // 255
    o("Invulnerability", 0x00000008, TV_POTION2, b'!', 0, 1000, 97, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 256
    o("Heroism", 0x00000010, TV_POTION2, b'!', 0, 35, 98, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 257
    o("Super Heroism", 0x00000020, TV_POTION2, b'!', 0, 100, 99, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 258
    o("Boldness", 0x00000040, TV_POTION2, b'!', 0, 10, 100, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 259
    o("Restore Life Levels", 0x00000080, TV_POTION2, b'!', 0, 400, 101, 1, 4, 0, 0, 0, 0, (1, 1), 40), // 260
    o("Resist Heat", 0x00000100, TV_POTION2, b'!', 0, 30, 102, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 261
    o("Resist Cold", 0x00000200, TV_POTION2, b'!', 0, 30, 103, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 262
    o("Detect Invisible", 0x00000400, TV_POTION2, b'!', 0, 50, 104, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 263
    o("Slow Poison", 0x00000800, TV_POTION2, b'!', 0, 25, 105, 1, 4, 0, 0, 0, 0, (1, 1), 1), // 264
    o("Neutralize Poison", 0x00001000, TV_POTION2, b'!', 0, 75, 106, 1, 4, 0, 0, 0, 0, (1, 1), 5), // 265
    o("Restore Mana", 0x00002000, TV_POTION2, b'!', 0, 350, 107, 1, 4, 0, 0, 0, 0, (1, 1), 25), // 266
    o("Infra-Vision", 0x00004000, TV_POTION2, b'!', 0, 20, 108, 1, 4, 0, 0, 0, 0, (1, 1), 3), // 267
    o("& Flask~ of Oil", 0x00040000, TV_FLASK, b'!', 7500, 3, 64, 1, 10, 0, 0, 0, 0, (2, 6), 1), // 268
    o("Light", 0x00000001, TV_WAND, b'-', 0, 200, 0, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 269
    o("Lightning Bolts", 0x00000002, TV_WAND, b'-', 0, 600, 1, 1, 10, 0, 0, 0, 0, (1, 1), 15), // 270
    o("Frost Bolts", 0x00000004, TV_WAND, b'-', 0, 800, 2, 1, 10, 0, 0, 0, 0, (1, 1), 20), // 271
    o("Fire Bolts", 0x00000008, TV_WAND, b'-', 0, 1000, 3, 1, 10, 0, 0, 0, 0, (1, 1), 30), // 272
    o("Stone-to-Mud", 0x00000010, TV_WAND, b'-', 0, 300, 4, 1, 10, 0, 0, 0, 0, (1, 1), 12), // 273
    o("Polymorph", 0x00000020, TV_WAND, b'-', 0, 400, 5, 1, 10, 0, 0, 0, 0, (1, 1), 20), // 274
    o("Heal Monster", 0x00000040, TV_WAND, b'-', 0, 0, 6, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 275
    o("Haste Monster", 0x00000080, TV_WAND, b'-', 0, 0, 7, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 276
    o("Slow Monster", 0x00000100, TV_WAND, b'-', 0, 500, 8, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 277
    o("Confuse Monster", 0x00000200, TV_WAND, b'-', 0, 400, 9, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 278
    o("Sleep Monster", 0x00000400, TV_WAND, b'-', 0, 500, 10, 1, 10, 0, 0, 0, 0, (1, 1), 7), // 279
    o("Drain Life", 0x00000800, TV_WAND, b'-', 0, 1200, 11, 1, 10, 0, 0, 0, 0, (1, 1), 50), // 280
    o("Trap/Door Destruction", 0x00001000, TV_WAND, b'-', 0, 500, 12, 1, 10, 0, 0, 0, 0, (1, 1), 12), // 281
    o("Magic Missile", 0x00002000, TV_WAND, b'-', 0, 200, 13, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 282
    o("Wall Building", 0x00004000, TV_WAND, b'-', 0, 400, 14, 1, 10, 0, 0, 0, 0, (1, 1), 25), // 283
    o("Clone Monster", 0x00008000, TV_WAND, b'-', 0, 0, 15, 1, 10, 0, 0, 0, 0, (1, 1), 15), // 284
    o("Teleport Away", 0x00010000, TV_WAND, b'-', 0, 350, 16, 1, 10, 0, 0, 0, 0, (1, 1), 20), // 285
    o("Disarming", 0x00020000, TV_WAND, b'-', 0, 500, 17, 1, 10, 0, 0, 0, 0, (1, 1), 20), // 286
    o("Lightning Balls", 0x00040000, TV_WAND, b'-', 0, 1200, 18, 1, 10, 0, 0, 0, 0, (1, 1), 35), // 287
    o("Cold Balls", 0x00080000, TV_WAND, b'-', 0, 1500, 19, 1, 10, 0, 0, 0, 0, (1, 1), 40), // 288
    o("Fire Balls", 0x00100000, TV_WAND, b'-', 0, 1800, 20, 1, 10, 0, 0, 0, 0, (1, 1), 50), // 289
    o("Stinking Cloud", 0x00200000, TV_WAND, b'-', 0, 400, 21, 1, 10, 0, 0, 0, 0, (1, 1), 5), // 290
    o("Acid Balls", 0x00400000, TV_WAND, b'-', 0, 1650, 22, 1, 10, 0, 0, 0, 0, (1, 1), 48), // 291
    o("Wonder", 0x00800000, TV_WAND, b'-', 0, 250, 23, 1, 10, 0, 0, 0, 0, (1, 1), 2), // 292
    o("Light", 0x00000001, TV_STAFF, b'_', 0, 250, 0, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 293
    o("Door/Stair Location", 0x00000002, TV_STAFF, b'_', 0, 350, 1, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 294
    o("Trap Location", 0x00000004, TV_STAFF, b'_', 0, 350, 2, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 295
    o("Treasure Location", 0x00000008, TV_STAFF, b'_', 0, 200, 3, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 296
    o("Object Location", 0x00000010, TV_STAFF, b'_', 0, 200, 4, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 297
    o("Teleportation", 0x00000020, TV_STAFF, b'_', 0, 800, 5, 1, 50, 0, 0, 0, 0, (1, 2), 20), // 298
    o("Earthquakes", 0x00000040, TV_STAFF, b'_', 0, 350, 6, 1, 50, 0, 0, 0, 0, (1, 2), 40), // 299
    o("Summoning", 0x00000080, TV_STAFF, b'_', 0, 0, 7, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 300
    o("Summoning", 0x00000080, TV_STAFF, b'_', 0, 0, 7, 1, 50, 0, 0, 0, 0, (1, 2), 50), // 301
    o("*Destruction*", 0x00000200, TV_STAFF, b'_', 0, 2500, 8, 1, 50, 0, 0, 0, 0, (1, 2), 50), // 302
    o("Starlight", 0x00000400, TV_STAFF, b'_', 0, 400, 9, 1, 50, 0, 0, 0, 0, (1, 2), 20), // 303
    o("Haste Monsters", 0x00000800, TV_STAFF, b'_', 0, 0, 10, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 304
    o("Slow Monsters", 0x00001000, TV_STAFF, b'_', 0, 800, 11, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 305
    o("Sleep Monsters", 0x00002000, TV_STAFF, b'_', 0, 700, 12, 1, 50, 0, 0, 0, 0, (1, 2), 10), // 306
    o("Cure Light Wounds", 0x00004000, TV_STAFF, b'_', 0, 200, 13, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 307
    o("Detect Invisible", 0x00008000, TV_STAFF, b'_', 0, 200, 14, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 308
    o("Speed", 0x00010000, TV_STAFF, b'_', 0, 1000, 15, 1, 50, 0, 0, 0, 0, (1, 2), 40), // 309
    o("Slowness", 0x00020000, TV_STAFF, b'_', 0, 0, 16, 1, 50, 0, 0, 0, 0, (1, 2), 40), // 310
    o("Mass Polymorph", 0x00040000, TV_STAFF, b'_', 0, 750, 17, 1, 50, 0, 0, 0, 0, (1, 2), 46), // 311
    o("Remove Curse", 0x00080000, TV_STAFF, b'_', 0, 500, 18, 1, 50, 0, 0, 0, 0, (1, 2), 47), // 312
    o("Detect Evil", 0x00100000, TV_STAFF, b'_', 0, 350, 19, 1, 50, 0, 0, 0, 0, (1, 2), 20), // 313
    o("Curing", 0x00200000, TV_STAFF, b'_', 0, 1000, 20, 1, 50, 0, 0, 0, 0, (1, 2), 25), // 314
    o("Dispel Evil", 0x00400000, TV_STAFF, b'_', 0, 1200, 21, 1, 50, 0, 0, 0, 0, (1, 2), 49), // 315
    o("Darkness", 0x01000000, TV_STAFF, b'_', 0, 0, 22, 1, 50, 0, 0, 0, 0, (1, 2), 50), // 316
    o("Darkness", 0x01000000, TV_STAFF, b'_', 0, 0, 22, 1, 50, 0, 0, 0, 0, (1, 2), 5), // 317
    o("[Beginners-Magick]", 0x0000007F, TV_MAGIC_BOOK, b'?', 0, 25, 64, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 318
    o("[Magick I]", 0x0000FF80, TV_MAGIC_BOOK, b'?', 0, 100, 65, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 319
    o("[Magick II]", 0x00FF0000, TV_MAGIC_BOOK, b'?', 0, 400, 66, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 320
    o("[The Mages' Guide to Power]", 0x7F000000, TV_MAGIC_BOOK, b'?', 0, 800, 67, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 321
    o("[Beginners Handbook]", 0x000000FF, TV_PRAYER_BOOK, b'?', 0, 25, 64, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 322
    o("[Words of Wisdom]", 0x0000FF00, TV_PRAYER_BOOK, b'?', 0, 100, 65, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 323
    o("[Chants and Blessings]", 0x01FF0000, TV_PRAYER_BOOK, b'?', 0, 400, 66, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 324
    o("[Exorcisms and Dispellings]", 0x7E000000, TV_PRAYER_BOOK, b'?', 0, 800, 67, 1, 30, 0, 0, 0, 0, (1, 1), 40), // 325
    o("& Small Wooden Chest", 0x13800000, TV_CHEST, b'&', 0, 20, 1, 1, 250, 0, 0, 0, 0, (2, 3), 7), // 326
    o("& Large Wooden Chest", 0x17800000, TV_CHEST, b'&', 0, 60, 4, 1, 500, 0, 0, 0, 0, (2, 5), 15), // 327
    o("& Small Iron Chest", 0x17800000, TV_CHEST, b'&', 0, 100, 7, 1, 500, 0, 0, 0, 0, (2, 4), 25), // 328
    o("& Large Iron Chest", 0x23800000, TV_CHEST, b'&', 0, 150, 10, 1, 1000, 0, 0, 0, 0, (2, 6), 35), // 329
    o("& Small Steel Chest", 0x1B800000, TV_CHEST, b'&', 0, 200, 13, 1, 500, 0, 0, 0, 0, (2, 4), 45), // 330
    o("& Large Steel Chest", 0x33800000, TV_CHEST, b'&', 0, 250, 16, 1, 1000, 0, 0, 0, 0, (2, 6), 50), // 331
    o("& Rat Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 1, 1, 10, 0, 0, 0, 0, (1, 1), 1), // 332
    o("& Giant Centipede Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 2, 1, 25, 0, 0, 0, 0, (1, 1), 1), // 333
    o("some Filthy Rags", 0x00000000, TV_SOFT_ARMOR, b'~', 0, 0, 63, 1, 20, 0, 0, 1, 0, (0, 0), 0), // 334
    o("& empty bottle", 0x00000000, TV_MISC, b'!', 0, 0, 4, 1, 2, 0, 0, 0, 0, (1, 1), 0), // 335
    o("some shards of pottery", 0x00000000, TV_MISC, b'~', 0, 0, 5, 1, 5, 0, 0, 0, 0, (1, 1), 0), // 336
    o("& Human Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 7, 1, 60, 0, 0, 0, 0, (1, 1), 1), // 337
    o("& Dwarf Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 8, 1, 50, 0, 0, 0, 0, (1, 1), 1), // 338
    o("& Elf Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 9, 1, 40, 0, 0, 0, 0, (1, 1), 1), // 339
    o("& Gnome Skeleton", 0x00000000, TV_MISC, b's', 0, 0, 10, 1, 25, 0, 0, 0, 0, (1, 1), 1), // 340
    o("& broken set of teeth", 0x00000000, TV_MISC, b's', 0, 0, 11, 1, 3, 0, 0, 0, 0, (1, 1), 0), // 341
    o("& large broken bone", 0x00000000, TV_MISC, b's', 0, 0, 12, 1, 2, 0, 0, 0, 0, (1, 1), 0), // 342
    o("& broken stick", 0x00000000, TV_MISC, b'~', 0, 0, 13, 1, 3, 0, 0, 0, 0, (1, 1), 0), // 343
    // end of Dungeon items
    // Store items, which are not also dungeon items, some of these
    // can be found above, except that the number is >1 below.
    o("& Ration~ of Food", 0x00000000, TV_FOOD, b',', 5000, 3, 90, 5, 10, 0, 0, 0, 0, (0, 0), 0), // 344
    o("& Hard Biscuit~", 0x00000000, TV_FOOD, b',', 500, 1, 93, 5, 2, 0, 0, 0, 0, (0, 0), 0), // 345
    o("& Strip~ of Beef Jerky", 0x00000000, TV_FOOD, b',', 1750, 2, 94, 5, 4, 0, 0, 0, 0, (0, 0), 0), // 346
    o("& Pint~ of Fine Ale", 0x00000000, TV_FOOD, b',', 500, 1, 95, 3, 10, 0, 0, 0, 0, (0, 0), 0), // 347
    o("& Pint~ of Fine Wine", 0x00000000, TV_FOOD, b',', 400, 2, 96, 1, 10, 0, 0, 0, 0, (0, 0), 0), // 348
    o("& Pick", 0x20000000, TV_DIGGING, b'\\', 1, 50, 1, 1, 150, 0, 0, 0, 0, (1, 3), 0), // 349
    o("& Shovel", 0x20000000, TV_DIGGING, b'\\', 0, 15, 4, 1, 60, 0, 0, 0, 0, (1, 2), 0), // 350
    o("Identify", 0x00000008, TV_SCROLL1, b'?', 0, 50, 67, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 351
    o("Light", 0x00000020, TV_SCROLL1, b'?', 0, 15, 69, 3, 5, 0, 0, 0, 0, (0, 0), 0), // 352
    o("Phase Door", 0x00000080, TV_SCROLL1, b'?', 0, 15, 71, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 353
    o("Magic Mapping", 0x00000800, TV_SCROLL1, b'?', 0, 40, 75, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 354
    o("Treasure Detection", 0x00004000, TV_SCROLL1, b'?', 0, 15, 78, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 355
    o("Object Detection", 0x00008000, TV_SCROLL1, b'?', 0, 15, 79, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 356
    o("Detect Invisible", 0x00080000, TV_SCROLL1, b'?', 0, 15, 83, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 357
    o("Blessing", 0x00000020, TV_SCROLL2, b'?', 0, 15, 99, 2, 5, 0, 0, 0, 0, (0, 0), 0), // 358
    o("Word-of-Recall", 0x00000100, TV_SCROLL2, b'?', 0, 150, 102, 3, 5, 0, 0, 0, 0, (0, 0), 0), // 359
    o("Cure Light Wounds", 0x10001000, TV_POTION1, b'!', 50, 15, 79, 2, 4, 0, 0, 0, 0, (1, 1), 0), // 360
    o("Heroism", 0x00000010, TV_POTION2, b'!', 0, 35, 98, 2, 4, 0, 0, 0, 0, (1, 1), 0), // 361
    o("Boldness", 0x00000040, TV_POTION2, b'!', 0, 10, 100, 2, 4, 0, 0, 0, 0, (1, 1), 0), // 362
    o("Slow Poison", 0x00000800, TV_POTION2, b'!', 0, 25, 105, 2, 4, 0, 0, 0, 0, (1, 1), 0), // 363
    o("& Brass Lantern~", 0x00000000, TV_LIGHT, b'~', 7500, 35, 0, 1, 50, 0, 0, 0, 0, (1, 1), 1), // 364
    o("& Wooden Torch~", 0x00000000, TV_LIGHT, b'~', 4000, 2, 192, 5, 30, 0, 0, 0, 0, (1, 1), 1), // 365
    o("& Flask~ of Oil", 0x00040000, TV_FLASK, b'!', 7500, 3, 64, 5, 10, 0, 0, 0, 0, (2, 6), 1), // 366
    // end store items
    // start doors
    // Secret door must have same sub_category_id as closed door in
    // TRAP_LISTB.  See CHANGE_TRAP. Must use & because of stone_to_mud.
    o("& open door", 0x00000000, TV_OPEN_DOOR, b'\'', 0, 0, 1, 1, 0, 0, 0, 0, 0, (1, 1), 0), // 367
    o("& closed door", 0x00000000, TV_CLOSED_DOOR, b'+', 0, 0, 19, 1, 0, 0, 0, 0, 0, (1, 1), 0), // 368
    o("& secret door", 0x00000000, TV_SECRET_DOOR, b'#', 0, 0, 19, 1, 0, 0, 0, 0, 0, (1, 1), 0), // 369
    // end doors
    // stairs
    o("an up staircase", 0x00000000, TV_UP_STAIR, b'<', 0, 0, 1, 1, 0, 0, 0, 0, 0, (1, 1), 0), // 370
    o("a down staircase", 0x00000000, TV_DOWN_STAIR, b'>', 0, 0, 1, 1, 0, 0, 0, 0, 0, (1, 1), 0), // 371
    // store door
    // Stores are just special traps
    o("General Store", 0x00000000, TV_STORE_DOOR, b'1', 0, 0, 101, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 372
    o("Armory", 0x00000000, TV_STORE_DOOR, b'2', 0, 0, 102, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 373
    o("Weapon Smiths", 0x00000000, TV_STORE_DOOR, b'3', 0, 0, 103, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 374
    o("Temple", 0x00000000, TV_STORE_DOOR, b'4', 0, 0, 104, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 375
    o("Alchemy Shop", 0x00000000, TV_STORE_DOOR, b'5', 0, 0, 105, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 376
    o("Magic Shop", 0x00000000, TV_STORE_DOOR, b'6', 0, 0, 106, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 377
    // end store door
    // Traps are just Nasty treasures.
    // Traps: Level represents the relative difficulty of disarming;
    // and `misc_use` represents the experienced gained when disarmed
    o("an open pit", 0x00000000, TV_VIS_TRAP, b' ', 1, 0, 1, 1, 0, 0, 0, 0, 0, (2, 6), 50), // 378
    o("an arrow trap", 0x00000000, TV_INVIS_TRAP, b'^', 3, 0, 2, 1, 0, 0, 0, 0, 0, (1, 8), 90), // 379
    o("a covered pit", 0x00000000, TV_INVIS_TRAP, b'^', 2, 0, 3, 1, 0, 0, 0, 0, 0, (2, 6), 60), // 380
    o("a trap door", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 4, 1, 0, 0, 0, 0, 0, (2, 8), 75), // 381
    o("a gas trap", 0x00000000, TV_INVIS_TRAP, b'^', 3, 0, 5, 1, 0, 0, 0, 0, 0, (1, 4), 95), // 382
    o("a loose rock", 0x00000000, TV_INVIS_TRAP, b';', 0, 0, 6, 1, 0, 0, 0, 0, 0, (0, 0), 10), // 383
    o("a dart trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 7, 1, 0, 0, 0, 0, 0, (1, 4), 110), // 384
    o("a strange rune", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 8, 1, 0, 0, 0, 0, 0, (0, 0), 90), // 385
    o("some loose rock", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 9, 1, 0, 0, 0, 0, 0, (2, 6), 90), // 386
    o("a gas trap", 0x00000000, TV_INVIS_TRAP, b'^', 10, 0, 10, 1, 0, 0, 0, 0, 0, (1, 4), 105), // 387
    o("a strange rune", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 11, 1, 0, 0, 0, 0, 0, (0, 0), 90), // 388
    o("a blackened spot", 0x00000000, TV_INVIS_TRAP, b'^', 10, 0, 12, 1, 0, 0, 0, 0, 0, (4, 6), 110), // 389
    o("some corroded rock", 0x00000000, TV_INVIS_TRAP, b'^', 10, 0, 13, 1, 0, 0, 0, 0, 0, (4, 6), 110), // 390
    o("a gas trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 14, 1, 0, 0, 0, 0, 0, (2, 6), 105), // 391
    o("a gas trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 15, 1, 0, 0, 0, 0, 0, (1, 4), 110), // 392
    o("a gas trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 16, 1, 0, 0, 0, 0, 0, (1, 8), 105), // 393
    o("a dart trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 17, 1, 0, 0, 0, 0, 0, (1, 8), 110), // 394
    o("a dart trap", 0x00000000, TV_INVIS_TRAP, b'^', 5, 0, 18, 1, 0, 0, 0, 0, 0, (1, 8), 110), // 395
    // rubble
    o("some rubble", 0x00000000, TV_RUBBLE, b':', 0, 0, 1, 1, 0, 0, 0, 0, 0, (0, 0), 0), // 396
    // mush
    o("& Pint~ of Fine Grade Mush", 0x00000000, TV_FOOD, b',', 1500, 1, 97, 1, 1, 0, 0, 0, 0, (1, 1), 1), // 397
    // Special trap
    o("a strange rune", 0x00000000, TV_VIS_TRAP, b'^', 0, 0, 99, 1, 0, 0, 0, 0, 0, (0, 0), 10), // 398
    // Gold list (All types of gold and gems are defined here)
    o("copper", 0x00000000, TV_GOLD, b'$', 0, 3, 1, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 399
    o("copper", 0x00000000, TV_GOLD, b'$', 0, 4, 2, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 400
    o("copper", 0x00000000, TV_GOLD, b'$', 0, 5, 3, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 401
    o("silver", 0x00000000, TV_GOLD, b'$', 0, 6, 4, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 402
    o("silver", 0x00000000, TV_GOLD, b'$', 0, 7, 5, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 403
    o("silver", 0x00000000, TV_GOLD, b'$', 0, 8, 6, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 404
    o("garnets", 0x00000000, TV_GOLD, b'*', 0, 9, 7, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 405
    o("garnets", 0x00000000, TV_GOLD, b'*', 0, 10, 8, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 406
    o("gold", 0x00000000, TV_GOLD, b'$', 0, 12, 9, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 407
    o("gold", 0x00000000, TV_GOLD, b'$', 0, 14, 10, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 408
    o("gold", 0x00000000, TV_GOLD, b'$', 0, 16, 11, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 409
    o("opals", 0x00000000, TV_GOLD, b'*', 0, 18, 12, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 410
    o("sapphires", 0x00000000, TV_GOLD, b'*', 0, 20, 13, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 411
    o("gold", 0x00000000, TV_GOLD, b'$', 0, 24, 14, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 412
    o("rubies", 0x00000000, TV_GOLD, b'*', 0, 28, 15, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 413
    o("diamonds", 0x00000000, TV_GOLD, b'*', 0, 32, 16, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 414
    o("emeralds", 0x00000000, TV_GOLD, b'*', 0, 40, 17, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 415
    o("mithril", 0x00000000, TV_GOLD, b'$', 0, 80, 18, 1, 0, 0, 0, 0, 0, (0, 0), 1), // 416
    // nothing, used as inventory place holder
    // must be stackable, so that can be picked up by inventoryCarryItem
    o("nothing", 0x00000000, TV_NOTHING, b' ', 0, 0, 64, 0, 0, 0, 0, 0, 0, (0, 0), 0), // 417
    // these next two are needed only for the names
    o("& ruined chest", 0x00000000, TV_CHEST, b'&', 0, 0, 0, 1, 250, 0, 0, 0, 0, (0, 0), 0), // 418
    o("", 0x00000000, TV_NOTHING, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, (0, 0), 0), // 419
];

// note: the first entry was CNIL (null) in the original source
pub static SPECIAL_ITEM_NAMES: [&str; SN_ARRAY_SIZE] = [
    "", "(R)", "(RA)",
    "(RF)", "(RC)", "(RL)",
    "(HA)", "(DF)", "(SA)",
    "(SD)", "(SE)", "(SU)",
    "(FT)", "(FB)", "of Free Action",
    "of Slaying", "of Clumsiness", "of Weakness",
    "of Slow Descent", "of Speed", "of Stealth",
    "of Slowness", "of Noise", "of Great Mass",
    "of Intelligence", "of Wisdom", "of Infra-Vision",
    "of Might", "of Lordliness", "of the Magi",
    "of Beauty", "of Seeing", "of Regeneration",
    "of Stupidity", "of Dullness", "of Blindness",
    "of Timidness", "of Teleportation", "of Ugliness",
    "of Protection", "of Irritation", "of Vulnerability",
    "of Enveloping", "of Fire", "of Slay Evil",
    "of Dragon Slaying", "(Empty)", "(Locked)",
    "(Poison Needle)", "(Gas Trap)", "(Explosion Device)",
    "(Summoning Runes)", "(Multiple Traps)", "(Disarmed)",
    "(Unlocked)", "of Slay Animal",
];
