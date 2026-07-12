// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Basic Configuration

// Some constants below are never referenced by the game logic; they were
// unused in the original C++ as well (e.g. the individual CS_* monster
// spell bits, which the code decodes positionally, and the PY_INT..PY_CHR
// status bits, addressed as `PY_STR << stat`). They are kept to document
// the bit layouts and to stay faithful to `config.cpp`.
#![allow(dead_code)]

// Data files used by Umoria
// NOTE: use relative paths to the executable binary.
pub mod files {
    use crate::globals::RacyCell;

    pub const SPLASH_SCREEN: &str = "data/splash.txt";
    pub const WELCOME_SCREEN: &str = "data/welcome.txt";
    pub const LICENSE: &str = "LICENSE";
    pub const VERSIONS_HISTORY: &str = "data/versions.txt";
    pub const HELP: &str = "data/help.txt";
    pub const HELP_WIZARD: &str = "data/help_wizard.txt";
    pub const HELP_ROGUELIKE: &str = "data/rl_help.txt";
    pub const HELP_ROGUELIKE_WIZARD: &str = "data/rl_help_wizard.txt";
    pub const DEATH_TOMB: &str = "data/death_tomb.txt";
    pub const DEATH_ROYAL: &str = "data/death_royal.txt";
    pub const SCORES: &str = "scores.dat";

    static SAVE_GAME: RacyCell<String> = RacyCell::new(String::new());

    pub fn save_game() -> String {
        let name = SAVE_GAME.get();
        if name.is_empty() {
            "game.sav".to_string()
        } else {
            name.clone()
        }
    }

    pub fn set_save_game(filename: &str) {
        *SAVE_GAME.get() = filename.to_string();
    }
}

// Game options as set on startup and with `=` set options command -CJS-
pub mod options {
    use crate::globals::RacyCell;

    pub struct Options {
        pub display_counts: bool,          // Display rest/repeat counts
        pub find_bound: bool,              // Print yourself on a run (slower)
        pub run_cut_corners: bool,         // Cut corners while running
        pub run_examine_corners: bool,     // Check corners while running
        pub run_ignore_doors: bool,        // Run through open doors
        pub run_print_self: bool,          // Stop running when the map shifts
        pub highlight_seams: bool,         // Highlight magma and quartz veins
        pub prompt_to_pickup: bool,        // Prompt to pick something up
        pub use_roguelike_keys: bool,      // Use classic Roguelike keys
        pub show_inventory_weights: bool,  // Display weights in inventory
        pub error_beep_sound: bool,        // Beep for invalid characters
    }

    static OPTIONS: RacyCell<Options> = RacyCell::new(Options {
        display_counts: true,
        find_bound: false,
        run_cut_corners: true,
        run_examine_corners: true,
        run_ignore_doors: false,
        run_print_self: false,
        highlight_seams: false,
        prompt_to_pickup: false,
        use_roguelike_keys: false,
        show_inventory_weights: false,
        error_beep_sound: true,
    });

    pub fn options() -> &'static mut Options {
        OPTIONS.get()
    }
}

// Dungeon generation values
// Note: The entire design of dungeon can be changed by only slight adjustments here.
pub mod dungeon {
    pub const DUN_RANDOM_DIR: u8 = 9; // 1/Chance of Random direction
    pub const DUN_DIR_CHANGE: u8 = 70; // Chance of changing direction (99 max)
    pub const DUN_TUNNELING: u8 = 15; // Chance of extra tunneling
    pub const DUN_ROOMS_MEAN: u8 = 32; // Mean of # of rooms, standard dev2
    pub const DUN_ROOM_DOORS: u8 = 25; // % chance of room doors
    pub const DUN_TUNNEL_DOORS: u8 = 15; // % chance of doors at tunnel junctions
    pub const DUN_STREAMER_DENSITY: u8 = 5; // Density of streamers
    pub const DUN_STREAMER_WIDTH: u8 = 2; // Width of streamers
    pub const DUN_MAGMA_STREAMER: u8 = 3; // Number of magma streamers
    pub const DUN_MAGMA_TREASURE: u8 = 90; // 1/x chance of treasure per magma
    pub const DUN_QUARTZ_STREAMER: u8 = 2; // Number of quartz streamers
    pub const DUN_QUARTZ_TREASURE: u8 = 40; // 1/x chance of treasure per quartz
    pub const DUN_UNUSUAL_ROOMS: u16 = 300; // Level/x chance of unusual room

    pub mod objects {
        pub const OBJ_OPEN_DOOR: u16 = 367;
        pub const OBJ_CLOSED_DOOR: u16 = 368;
        pub const OBJ_SECRET_DOOR: u16 = 369;
        pub const OBJ_UP_STAIR: u16 = 370;
        pub const OBJ_DOWN_STAIR: u16 = 371;
        pub const OBJ_STORE_DOOR: u16 = 372;
        pub const OBJ_TRAP_LIST: u16 = 378;
        pub const OBJ_RUBBLE: u16 = 396;
        pub const OBJ_MUSH: u16 = 397;
        pub const OBJ_SCARE_MON: u16 = 398;
        pub const OBJ_GOLD_LIST: u16 = 399;
        pub const OBJ_NOTHING: u16 = 417;
        pub const OBJ_RUINED_CHEST: u16 = 418;
        pub const OBJ_WIZARD: u16 = 419;

        pub const MAX_GOLD_TYPES: u8 = 18; // Number of different types of gold
        pub const MAX_TRAPS: u8 = 18; // Number of defined traps

        pub const LEVEL_OBJECTS_PER_ROOM: u8 = 7; // Amount of objects for rooms
        pub const LEVEL_OBJECTS_PER_CORRIDOR: u8 = 2; // Amount of objects for corridors
        pub const LEVEL_TOTAL_GOLD_AND_GEMS: u8 = 2; // Amount of gold (and gems)
    }
}

// Note: Number of special objects, and degree of enchantments can be adjusted here.
pub mod treasure {
    pub const MIN_TREASURE_LIST_ID: u8 = 1; // Minimum treasure_list index used
    pub const TREASURE_CHANCE_OF_GREAT_ITEM: u8 = 12; // 1/n Chance of item being a Great Item

    // Magic Treasure Generation constants
    pub const LEVEL_STD_OBJECT_ADJUST: u8 = 125; // Adjust STD per level * 100
    pub const LEVEL_MIN_OBJECT_STD: u8 = 7; // Minimum STD
    pub const LEVEL_TOWN_OBJECTS: u8 = 7; // Town object generation level
    pub const OBJECT_BASE_MAGIC: u8 = 15; // Base amount of magic
    pub const OBJECT_MAX_BASE_MAGIC: u8 = 70; // Max amount of magic
    pub const OBJECT_CHANCE_SPECIAL: u8 = 6; // magic_chance/# special magic
    pub const OBJECT_CHANCE_CURSED: u8 = 13; // 10*magic_chance/# cursed items

    // Constants describing limits of certain objects
    pub const OBJECT_LAMP_MAX_CAPACITY: u16 = 15000; // Maximum amount that lamp can be filled
    pub const OBJECT_BOLTS_MAX_RANGE: u8 = 18; // Maximum range of bolts and balls
    pub const OBJECTS_RUNE_PROTECTION: u16 = 3000; // Rune of protection resistance

    // definitions for objects that can be worn
    pub mod flags {
        pub const TR_STATS: u32 = 0x0000003F; // the stats must be the low 6 bits
        pub const TR_STR: u32 = 0x00000001;
        pub const TR_INT: u32 = 0x00000002;
        pub const TR_WIS: u32 = 0x00000004;
        pub const TR_DEX: u32 = 0x00000008;
        pub const TR_CON: u32 = 0x00000010;
        pub const TR_CHR: u32 = 0x00000020;
        pub const TR_SEARCH: u32 = 0x00000040;
        pub const TR_SLOW_DIGEST: u32 = 0x00000080;
        pub const TR_STEALTH: u32 = 0x00000100;
        pub const TR_AGGRAVATE: u32 = 0x00000200;
        pub const TR_TELEPORT: u32 = 0x00000400;
        pub const TR_REGEN: u32 = 0x00000800;
        pub const TR_SPEED: u32 = 0x00001000;

        pub const TR_EGO_WEAPON: u32 = 0x0007E000;
        pub const TR_SLAY_DRAGON: u32 = 0x00002000;
        pub const TR_SLAY_ANIMAL: u32 = 0x00004000;
        pub const TR_SLAY_EVIL: u32 = 0x00008000;
        pub const TR_SLAY_UNDEAD: u32 = 0x00010000;
        pub const TR_FROST_BRAND: u32 = 0x00020000;
        pub const TR_FLAME_TONGUE: u32 = 0x00040000;

        pub const TR_RES_FIRE: u32 = 0x00080000;
        pub const TR_RES_ACID: u32 = 0x00100000;
        pub const TR_RES_COLD: u32 = 0x00200000;
        pub const TR_SUST_STAT: u32 = 0x00400000;
        pub const TR_FREE_ACT: u32 = 0x00800000;
        pub const TR_SEE_INVIS: u32 = 0x01000000;
        pub const TR_RES_LIGHT: u32 = 0x02000000;
        pub const TR_FFALL: u32 = 0x04000000;
        pub const TR_BLIND: u32 = 0x08000000;
        pub const TR_TIMID: u32 = 0x10000000;
        pub const TR_TUNNEL: u32 = 0x20000000;
        pub const TR_INFRA: u32 = 0x40000000;
        pub const TR_CURSED: u32 = 0x80000000;
    }

    // definitions for chests
    pub mod chests {
        pub const CH_LOCKED: u32 = 0x00000001;
        pub const CH_TRAPPED: u32 = 0x000001F0;
        pub const CH_LOSE_STR: u32 = 0x00000010;
        pub const CH_POISON: u32 = 0x00000020;
        pub const CH_PARALYSED: u32 = 0x00000040;
        pub const CH_EXPLODE: u32 = 0x00000080;
        pub const CH_SUMMON: u32 = 0x00000100;
    }
}

pub mod monsters {
    pub const MON_CHANCE_OF_NEW: u8 = 160; // 1/x chance of new monster each round
    pub const MON_MAX_SIGHT: u8 = 20; // Maximum dis a creature can be seen
    pub const MON_MAX_SPELL_CAST_DISTANCE: u8 = 20; // Maximum dis creature spell can be cast
    pub const MON_MAX_MULTIPLY_PER_LEVEL: u8 = 75; // Maximum reproductions on a level
    pub const MON_MULTIPLY_ADJUST: u8 = 7; // High value slows multiplication
    pub const MON_CHANCE_OF_NASTY: u8 = 50; // 1/x chance of high level creature
    pub const MON_MIN_PER_LEVEL: u8 = 14; // Minimum number of monsters/level
    pub const MON_MIN_TOWNSFOLK_DAY: u8 = 4; // Number of people on town level (day)
    pub const MON_MIN_TOWNSFOLK_NIGHT: u8 = 8; // Number of people on town level (night)
    pub const MON_ENDGAME_MONSTERS: u8 = 2; // Total number of "win" creatures
    pub const MON_ENDGAME_LEVEL: u8 = 50; // Level where winning creatures begin
    pub const MON_SUMMONED_LEVEL_ADJUST: u8 = 2; // Adjust level of summoned creatures
    pub const MON_PLAYER_EXP_DRAINED_PER_HIT: u8 = 2; // Percent of player exp drained per hit
    pub const MON_MIN_INDEX_ID: u8 = 2; // Minimum index in m_list (1 = py, 0 = no mon)
    pub const SCARE_MONSTER: u8 = 99;

    // definitions for creatures, cmove field
    pub mod move_flags {
        pub const CM_ALL_MV_FLAGS: u32 = 0x0000003F;
        pub const CM_ATTACK_ONLY: u32 = 0x00000001;
        pub const CM_MOVE_NORMAL: u32 = 0x00000002;
        pub const CM_ONLY_MAGIC: u32 = 0x00000004; // For Quylthulgs, which have no physical movement.

        pub const CM_RANDOM_MOVE: u32 = 0x00000038;
        pub const CM_20_RANDOM: u32 = 0x00000008;
        pub const CM_40_RANDOM: u32 = 0x00000010;
        pub const CM_75_RANDOM: u32 = 0x00000020;

        pub const CM_SPECIAL: u32 = 0x003F0000;
        pub const CM_INVISIBLE: u32 = 0x00010000;
        pub const CM_OPEN_DOOR: u32 = 0x00020000;
        pub const CM_PHASE: u32 = 0x00040000;
        pub const CM_EATS_OTHER: u32 = 0x00080000;
        pub const CM_PICKS_UP: u32 = 0x00100000;
        pub const CM_MULTIPLY: u32 = 0x00200000;

        pub const CM_SMALL_OBJ: u32 = 0x00800000;
        pub const CM_CARRY_OBJ: u32 = 0x01000000;
        pub const CM_CARRY_GOLD: u32 = 0x02000000;
        pub const CM_TREASURE: u32 = 0x7C000000;
        pub const CM_TR_SHIFT: u32 = 26; // used for recall of treasure
        pub const CM_60_RANDOM: u32 = 0x04000000;
        pub const CM_90_RANDOM: u32 = 0x08000000;
        pub const CM_1D2_OBJ: u32 = 0x10000000;
        pub const CM_2D2_OBJ: u32 = 0x20000000;
        pub const CM_4D2_OBJ: u32 = 0x40000000;
        pub const CM_WIN: u32 = 0x80000000;
    }

    // creature spell definitions
    pub mod spells {
        pub const CS_FREQ: u32 = 0x0000000F;
        pub const CS_SPELLS: u32 = 0x0001FFF0;
        pub const CS_TEL_SHORT: u32 = 0x00000010;
        pub const CS_TEL_LONG: u32 = 0x00000020;
        pub const CS_TEL_TO: u32 = 0x00000040;
        pub const CS_LGHT_WND: u32 = 0x00000080;
        pub const CS_SER_WND: u32 = 0x00000100;
        pub const CS_HOLD_PER: u32 = 0x00000200;
        pub const CS_BLIND: u32 = 0x00000400;
        pub const CS_CONFUSE: u32 = 0x00000800;
        pub const CS_FEAR: u32 = 0x00001000;
        pub const CS_SUMMON_MON: u32 = 0x00002000;
        pub const CS_SUMMON_UND: u32 = 0x00004000;
        pub const CS_SLOW_PER: u32 = 0x00008000;
        pub const CS_DRAIN_MANA: u32 = 0x00010000;

        pub const CS_BREATHE: u32 = 0x00F80000; // may also just indicate resistance
        pub const CS_BR_LIGHT: u32 = 0x00080000; // if no spell frequency set
        pub const CS_BR_GAS: u32 = 0x00100000;
        pub const CS_BR_ACID: u32 = 0x00200000;
        pub const CS_BR_FROST: u32 = 0x00400000;
        pub const CS_BR_FIRE: u32 = 0x00800000;
    }

    // creature defense flags
    pub mod defense {
        pub const CD_DRAGON: u16 = 0x0001;
        pub const CD_ANIMAL: u16 = 0x0002;
        pub const CD_EVIL: u16 = 0x0004;
        pub const CD_UNDEAD: u16 = 0x0008;
        pub const CD_WEAKNESS: u16 = 0x03F0;
        pub const CD_FROST: u16 = 0x0010;
        pub const CD_FIRE: u16 = 0x0020;
        pub const CD_POISON: u16 = 0x0040;
        pub const CD_ACID: u16 = 0x0080;
        pub const CD_LIGHT: u16 = 0x0100;
        pub const CD_STONE: u16 = 0x0200;
        pub const CD_NO_SLEEP: u16 = 0x1000;
        pub const CD_INFRA: u16 = 0x2000;
        pub const CD_MAX_HP: u16 = 0x4000;
    }
}

pub mod player {
    pub const PLAYER_MAX_EXP: i32 = 9999999; // Maximum amount of experience -CJS-
    pub const PLAYER_USE_DEVICE_DIFFICULTY: u8 = 3; // x> Harder devices x< Easier devices
    pub const PLAYER_FOOD_FULL: u16 = 10000; // Getting full
    pub const PLAYER_FOOD_MAX: u16 = 15000; // Maximum food value, beyond is wasted
    pub const PLAYER_FOOD_FAINT: u16 = 300; // Character begins fainting
    pub const PLAYER_FOOD_WEAK: u16 = 1000; // Warn player that they're getting weak
    pub const PLAYER_FOOD_ALERT: u16 = 2000; // Alert player that they're getting low on food
    pub const PLAYER_REGEN_FAINT: u8 = 33; // Regen factor*2^16 when fainting
    pub const PLAYER_REGEN_WEAK: u8 = 98; // Regen factor*2^16 when weak
    pub const PLAYER_REGEN_NORMAL: u8 = 197; // Regen factor*2^16 when full
    pub const PLAYER_REGEN_HPBASE: u16 = 1442; // Min amount hp regen*2^16
    pub const PLAYER_REGEN_MNBASE: u16 = 524; // Min amount mana regen*2^16
    pub const PLAYER_WEIGHT_CAP: u8 = 130; // "#"*(1/10 pounds) per strength point

    // definitions for the player's status field
    pub mod status {
        pub const PY_HUNGRY: u32 = 0x00000001;
        pub const PY_WEAK: u32 = 0x00000002;
        pub const PY_BLIND: u32 = 0x00000004;
        pub const PY_CONFUSED: u32 = 0x00000008;
        pub const PY_FEAR: u32 = 0x00000010;
        pub const PY_POISONED: u32 = 0x00000020;
        pub const PY_FAST: u32 = 0x00000040;
        pub const PY_SLOW: u32 = 0x00000080;
        pub const PY_SEARCH: u32 = 0x00000100;
        pub const PY_REST: u32 = 0x00000200;
        pub const PY_STUDY: u32 = 0x00000400;

        pub const PY_INVULN: u32 = 0x00001000;
        pub const PY_HERO: u32 = 0x00002000;
        pub const PY_SHERO: u32 = 0x00004000;
        pub const PY_BLESSED: u32 = 0x00008000;
        pub const PY_DET_INV: u32 = 0x00010000;
        pub const PY_TIM_INFRA: u32 = 0x00020000;
        pub const PY_SPEED: u32 = 0x00040000;
        pub const PY_STR_WGT: u32 = 0x00080000;
        pub const PY_PARALYSED: u32 = 0x00100000;
        pub const PY_REPEAT: u32 = 0x00200000;
        pub const PY_ARMOR: u32 = 0x00400000;

        pub const PY_STATS: u32 = 0x3F000000;
        pub const PY_STR: u32 = 0x01000000; // these 6 stat flags must be adjacent
        pub const PY_INT: u32 = 0x02000000;
        pub const PY_WIS: u32 = 0x04000000;
        pub const PY_DEX: u32 = 0x08000000;
        pub const PY_CON: u32 = 0x10000000;
        pub const PY_CHR: u32 = 0x20000000;

        pub const PY_HP: u32 = 0x40000000;
        pub const PY_MANA: u32 = 0x80000000;
    }
}

pub mod identification {
    // id's used for object description, stored in objects_identified array
    pub const OD_TRIED: u8 = 0x1;
    pub const OD_KNOWN1: u8 = 0x2;

    // id's used for item description, stored in i_ptr->ident
    pub const ID_MAGIK: u8 = 0x1;
    pub const ID_DAMD: u8 = 0x2;
    pub const ID_EMPTY: u8 = 0x4;
    pub const ID_KNOWN2: u8 = 0x8;
    pub const ID_STORE_BOUGHT: u8 = 0x10;
    pub const ID_SHOW_HIT_DAM: u8 = 0x20;
    pub const ID_NO_SHOW_P1: u8 = 0x40;
    pub const ID_SHOW_P1: u8 = 0x80;
}

pub mod spells {
    // Class spell types
    pub const SPELL_TYPE_NONE: u8 = 0;
    pub const SPELL_TYPE_MAGE: u8 = 1;
    pub const SPELL_TYPE_PRIEST: u8 = 2;

    // offsets to spell names in spell_names[] array
    pub const NAME_OFFSET_SPELLS: u8 = 0;
    pub const NAME_OFFSET_PRAYERS: u8 = 31;
}

pub mod stores {
    pub const STORE_MAX_AUTO_BUY_ITEMS: u8 = 18; // Max diff objects in stock for auto buy
    pub const STORE_MIN_AUTO_SELL_ITEMS: u8 = 10; // Min diff objects in stock for auto sell
    pub const STORE_STOCK_TURN_AROUND: u8 = 9; // Amount of buying and selling normally
}
