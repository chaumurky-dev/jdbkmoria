// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Game initialization and maintenance related functions

use crate::config;
use crate::globals::RacyCell;
use crate::tr;
use crate::helpers::get_current_unix_time;
use crate::inventory::Inventory;
use crate::player::py;
use crate::rng::{get_random_seed, rnd, set_random_seed};
use crate::types::Coord;
use crate::ui::{ESCAPE};
use crate::ui_io::{
    erase_line, flush_input_buffer, get_command, get_key_input, keypad_direction, move_cursor,
    put_string, put_string_clear_to_eol, terminal_bell_sound, terminal_restore,
};
use crate::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

pub const TREASURE_MAX_LEVELS: usize = 50; // Maximum level of magic in dungeon

// Note that the following constants are all related, if you change one, you
// must also change all succeeding ones.
// Also, player_base_provisions[] and store_choices[] may also have to be changed.
pub const MAX_OBJECTS_IN_GAME: usize = 420; // Number of objects for universe
pub const MAX_DUNGEON_OBJECTS: usize = 344; // Number of dungeon objects
pub const OBJECT_IDENT_SIZE: usize = 448; // 7*64, see object_offset() in desc.cpp, could be MAX_OBJECTS o_o() rewritten

// With LEVEL_MAX_OBJECTS set to 150, it's possible to get compacting
// objects during level generation, although it is extremely rare.
// jdbkmoria extension: scaled x4 (175 -> 700) to match the level content
// density increase that came with quadrupling the dungeon's area (see
// MAX_HEIGHT/MAX_WIDTH in dungeon.rs). Save-format safe: the save file
// stores an explicit object count, validated against this constant on load
// (see game_save.rs).
pub const LEVEL_MAX_OBJECTS: usize = 700; // Max objects per level

// definitions for the pseudo-normal distribution generation
pub const NORMAL_TABLE_SIZE: usize = 256;
pub const NORMAL_TABLE_SD: i32 = 64; // the standard deviation for the table

// Inventory command screen states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Blank = 0,
    Equipment,
    Inventory,
    Wear,
    Help,
    Wrong,
}

pub struct GameTreasure {
    pub current_id: i16, // Current treasure heap ptr
    pub list: [Inventory; LEVEL_MAX_OBJECTS],
}

// Keep track of the state of the current screen (inventory, equipment, help, etc.).
pub struct GameScreen {
    pub current_screen_id: Screen,
    pub screen_left_pos: i32,
    pub screen_bottom_pos: i32,
    pub wear_low_id: i32,
    pub wear_high_id: i32,
}

pub struct Game {
    pub magic_seed: u32, // Seed for initializing magic items (Potions, Wands, Staves, Scrolls, etc.)
    pub town_seed: u32,  // Seed for town generation

    pub character_generated: bool, // Don't save score until character generation is finished
    pub character_saved: bool,     // Prevents save on kill after saving a character
    pub character_is_dead: bool,   // `true` if character has died

    pub total_winner: bool, // Character beat the Balrog

    pub teleport_player: bool,  // Handle teleport traps
    pub player_free_turn: bool, // Player has a free turn, so do not move creatures

    pub to_be_wizard: bool, // Player requests to be Wizard - used during startup, when -w option used
    pub wizard_mode: bool,  // Character is a Wizard when true
    pub noscore: i16,       // Don't save a score for this game. -CJS-

    pub use_last_direction: bool,     // `true` when repeat commands should use last known direction
    pub doing_inventory_command: char, // Track inventory commands -CJS-
    pub last_command: char,           // Save of the previous player command
    pub command_count: u32,           // How many times to repeat a specific command -CJS-

    pub character_died_from: String, // What the character died from: starvation, Bat, etc.

    pub treasure: GameTreasure,

    pub screen: GameScreen,
}

impl Game {
    pub const fn new() -> Self {
        Game {
            magic_seed: 0,
            town_seed: 0,
            character_generated: false,
            character_saved: false,
            character_is_dead: false,
            total_winner: false,
            teleport_player: false,
            player_free_turn: false,
            to_be_wizard: false,
            wizard_mode: false,
            noscore: 0,
            use_last_direction: false,
            doing_inventory_command: '\0',
            last_command: ' ',
            command_count: 0,
            character_died_from: String::new(),
            treasure: GameTreasure {
                current_id: 0,
                list: [Inventory::empty(); LEVEL_MAX_OBJECTS],
            },
            screen: GameScreen {
                current_screen_id: Screen::Blank,
                screen_left_pos: 0,
                screen_bottom_pos: 0,
                wear_low_id: 0,
                wear_high_id: 0,
            },
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

static GAME: RacyCell<Game> = RacyCell::new(Game::new());

// Access the global game object (the C code's `game` global).
pub fn game() -> &'static mut Game {
    GAME.get()
}

static SORTED_OBJECTS: RacyCell<[i16; MAX_DUNGEON_OBJECTS]> = RacyCell::new([0; MAX_DUNGEON_OBJECTS]);
static TREASURE_LEVELS: RacyCell<[i16; TREASURE_MAX_LEVELS + 1]> = RacyCell::new([0; TREASURE_MAX_LEVELS + 1]);

pub fn sorted_objects() -> &'static mut [i16; MAX_DUNGEON_OBJECTS] {
    SORTED_OBJECTS.get()
}

pub fn treasure_levels() -> &'static mut [i16; TREASURE_MAX_LEVELS + 1] {
    TREASURE_LEVELS.get()
}

// holds the previous rnd state
static OLD_SEED: RacyCell<u32> = RacyCell::new(0);

// gets a new random seed for the random number generator
pub fn seeds_initialize(seed: u32) {
    let mut clock_var: u32;

    if seed == 0 {
        clock_var = get_current_unix_time();
    } else {
        clock_var = seed;
    }

    game().magic_seed = clock_var;

    clock_var = clock_var.wrapping_add(8762);
    game().town_seed = clock_var;

    clock_var = clock_var.wrapping_add(113452);
    set_random_seed(clock_var);

    // make it a little more random
    for _ in 0..random_number(100) {
        rnd();
    }
}

// change to different random number generator state
pub fn seed_set(seed: u32) {
    *OLD_SEED.get() = get_random_seed();

    // want reproducible state here
    set_random_seed(seed);
}

// restore the normal random generator state
pub fn seed_reset_to_old_seed() {
    set_random_seed(*OLD_SEED.get());
}

// Generates a random integer x where 1<=X<=MAXVAL -RAK-
pub fn random_number(max: i32) -> i32 {
    (rnd() % max) + 1
}

// Generates a random integer number of NORMAL distribution -RAK-
pub fn random_number_normal_distribution(mean: i32, standard: i32) -> i32 {
    let tmp = random_number(i16::MAX as i32);

    // off scale, assign random value between 4 and 5 times SD
    if tmp == i16::MAX as i32 {
        let mut offset = 4 * standard + random_number(standard);

        // one half are negative
        if random_number(2) == 1 {
            offset = -offset;
        }

        return mean + offset;
    }

    // binary search normal normal_table to get index that
    // matches tmp this takes up to 8 iterations.
    let normal_table = crate::data_tables::normal_table();

    let mut low = 0usize;
    let mut iindex = NORMAL_TABLE_SIZE >> 1;
    let mut high = NORMAL_TABLE_SIZE;

    loop {
        if normal_table[iindex] as i32 == tmp || high == low + 1 {
            break;
        }

        if normal_table[iindex] as i32 > tmp {
            high = iindex;
            iindex = low + ((iindex - low) >> 1);
        } else {
            low = iindex;
            iindex += (high - iindex) >> 1;
        }
    }

    // might end up one below target, check that here
    if (normal_table[iindex] as i32) < tmp {
        iindex += 1;
    }

    // normal_table is based on SD of 64, so adjust the
    // index value here, round the half way case up.
    let mut offset = ((standard * iindex as i32) + (NORMAL_TABLE_SD >> 1)) / NORMAL_TABLE_SD;

    // one half should be negative
    if random_number(2) == 1 {
        offset = -offset;
    }

    mean + offset
}

// Option accessor used by the options menu: (prompt, get, set) triples
// mirroring the C code's table of bool pointers.
struct GameOption {
    prompt: &'static str,
    get: fn() -> bool,
    set: fn(bool),
}

const GAME_OPTIONS: [GameOption; 11] = [
    GameOption {
        prompt: "Running: cut known corners",
        get: || config::options::options().run_cut_corners,
        set: |v| config::options::options().run_cut_corners = v,
    },
    GameOption {
        prompt: "Running: examine potential corners",
        get: || config::options::options().run_examine_corners,
        set: |v| config::options::options().run_examine_corners = v,
    },
    GameOption {
        prompt: "Running: print self during run",
        get: || config::options::options().run_print_self,
        set: |v| config::options::options().run_print_self = v,
    },
    GameOption {
        prompt: "Running: stop when map sector changes",
        get: || config::options::options().find_bound,
        set: |v| config::options::options().find_bound = v,
    },
    GameOption {
        prompt: "Running: run through open doors",
        get: || config::options::options().run_ignore_doors,
        set: |v| config::options::options().run_ignore_doors = v,
    },
    GameOption {
        prompt: "Prompt to pick up objects",
        get: || config::options::options().prompt_to_pickup,
        set: |v| config::options::options().prompt_to_pickup = v,
    },
    GameOption {
        prompt: "Rogue like commands",
        get: || config::options::options().use_roguelike_keys,
        set: |v| config::options::options().use_roguelike_keys = v,
    },
    GameOption {
        prompt: "Show weights in inventory",
        get: || config::options::options().show_inventory_weights,
        set: |v| config::options::options().show_inventory_weights = v,
    },
    GameOption {
        prompt: "Highlight and notice mineral seams",
        get: || config::options::options().highlight_seams,
        set: |v| config::options::options().highlight_seams = v,
    },
    GameOption {
        prompt: "Beep for invalid character",
        get: || config::options::options().error_beep_sound,
        set: |v| config::options::options().error_beep_sound = v,
    },
    GameOption {
        prompt: "Display rest/repeat counts",
        get: || config::options::options().display_counts,
        set: |v| config::options::options().display_counts = v,
    },
];

// Set or unset various boolean config::options -CJS-
pub fn set_game_options() {
    put_string_clear_to_eol(
        tr!("  ESC when finished, y/n to set options, <return> or - to move cursor"),
        Coord::new(0, 0),
    );

    let max = GAME_OPTIONS.len();
    for (i, option) in GAME_OPTIONS.iter().enumerate() {
        let msg = format!("{:<38}: {}", tr!(option.prompt), if (option.get)() { tr!("yes") } else { tr!("no ") });
        put_string_clear_to_eol(&msg, Coord::new(i as i32 + 1, 0));
    }
    erase_line(Coord::new(max as i32 + 1, 0));

    let mut option_id = 0usize;
    loop {
        move_cursor(Coord::new(option_id as i32 + 1, 40));

        match get_key_input() {
            ESCAPE => return,
            '-' => {
                if option_id > 0 {
                    option_id -= 1;
                } else {
                    option_id = max - 1;
                }
            }
            ' ' | '\n' | '\r' => {
                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            'y' | 'Y' => {
                put_string(tr!("yes"), Coord::new(option_id as i32 + 1, 40));

                (GAME_OPTIONS[option_id].set)(true);

                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            'n' | 'N' => {
                put_string(tr!("no "), Coord::new(option_id as i32 + 1, 40));

                (GAME_OPTIONS[option_id].set)(false);

                if option_id + 1 < max {
                    option_id += 1;
                } else {
                    option_id = 0;
                }
            }
            _ => {
                terminal_bell_sound();
            }
        }
    }
}

// Support for Umoria 5.2.2 up to 5.7.x.
// The save file format was frozen as of version 5.2.2.
pub fn valid_game_version(major: u8, minor: u8, patch: u8) -> bool {
    if major != 5 {
        return false;
    }

    if minor < 2 {
        return false;
    }

    if minor == 2 && patch < 2 {
        return false;
    }

    minor <= 7
}

pub fn is_current_game_version(major: u8, minor: u8, patch: u8) -> bool {
    major == CURRENT_VERSION_MAJOR && minor == CURRENT_VERSION_MINOR && patch == CURRENT_VERSION_PATCH
}

pub fn get_random_direction() -> i32 {
    loop {
        let dir = random_number(9);
        if dir != 5 {
            return dir;
        }
    }
}

// map roguelike direction commands into numbers
fn map_roguelike_keys_to_keypad(command: char) -> char {
    match command {
        'h' => '4',
        'y' => '7',
        'k' => '8',
        'u' => '9',
        'l' => '6',
        'n' => '3',
        'j' => '2',
        'b' => '1',
        '.' => '5',
        _ => command,
    }
}

// Prompts for a direction -RAK-
// Direction memory added, for repeated commands.  -CJS
pub fn get_direction_with_memory(prompt: Option<&str>, direction: &mut i32) -> bool {
    // used in counted commands. -CJS-
    if game().use_last_direction {
        *direction = py().prev_dir;
        return true;
    }

    let prompt = prompt.unwrap_or(tr!("Which direction?"));

    let mut command = '\0';

    loop {
        // Don't end a counted command. -CJS-
        let old_count = game().command_count;
        if !get_command(prompt, &mut command) {
            game().player_free_turn = true;
            return false;
        }
        game().command_count = old_count;

        if config::options::options().use_roguelike_keys {
            command = map_roguelike_keys_to_keypad(command);
        }

        // Arrow/keypad keys answer the prompt too (shifted or not).
        if let Some((direction, _)) = keypad_direction(command) {
            command = (b'0' + direction as u8) as char;
        }

        if ('1'..='9').contains(&command) && command != '5' {
            py().prev_dir = command as i32 - '0' as i32;
            *direction = py().prev_dir;
            return true;
        }

        terminal_bell_sound();
    }
}

// Similar to get_direction_with_memory(), except that no memory exists,
// and it is allowed to enter the null direction. -CJS-
pub fn get_all_directions(prompt: &str, direction: &mut i32) -> bool {
    let mut command = '\0';

    loop {
        if !get_command(prompt, &mut command) {
            game().player_free_turn = true;
            return false;
        }

        if config::options::options().use_roguelike_keys {
            command = map_roguelike_keys_to_keypad(command);
        }

        // Arrow/keypad keys answer the prompt too (shifted or not).
        if let Some((direction, _)) = keypad_direction(command) {
            command = (b'0' + direction as u8) as char;
        }

        if ('1'..='9').contains(&command) {
            *direction = command as i32 - '0' as i32;
            return true;
        }

        terminal_bell_sound();
    }
}

// Restore the terminal and exit
pub fn exit_program() -> ! {
    flush_input_buffer();
    terminal_restore();
    std::process::exit(0);
}

// Abort the program with a message displayed on the terminal.
// (never called in the original C++ either; kept for fidelity)
#[allow(dead_code)]
pub fn abort_program(msg: &str) -> ! {
    flush_input_buffer();
    terminal_restore();

    println!("Program was manually aborted with the message:");
    println!("{}", msg);

    std::process::exit(0);
}
