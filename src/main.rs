// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Initialization, main() function and main loop

mod character;
mod config;
mod data_creatures;
mod data_player;
mod data_recall;
mod data_store_owners;
mod data_stores;
mod data_tables;
mod data_treasure;
mod dice;
mod dungeon;
mod dungeon_generate;
mod dungeon_los;
mod dungeon_tile;
mod game;
mod game_death;
mod game_files;
mod game_objects;
mod game_run;
mod game_save;
mod globals;
mod helpers;
mod identification;
mod inventory;
mod mage_spells;
mod monster;
mod monster_manager;
mod player;
mod player_bash;
mod player_eat;
mod player_magic;
mod player_move;
mod player_pray;
mod player_quaff;
mod player_run;
mod player_stats;
mod player_throw;
mod player_traps;
mod player_tunnel;
mod recall;
mod recall_data;
mod rng;
mod scores;
mod scrolls;
mod spells;
mod spells_data;
mod staves;
mod store;
mod store_data;
mod store_inventory;
mod treasure;
mod treasure_magic;
mod types;
mod ui;
mod ui_inventory;
mod ui_io;
mod version;
mod wizard;

use game::{exit_program, game};
use game_files::initialize_score_file;
use game_run::start_moria;
use helpers::string_to_number;
use scores::show_scores_screen;
use ui_io::{check_file_permissions, terminal_initialize, terminal_restore};
use version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

const USAGE_INSTRUCTIONS: &str = r#"
Usage:
    rmoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
    -n           Force start of new game
    -r           Enable classic roguelike keys on startup (default: disabled, or save game settings)
    -d           Display high scores and exit
    -s NUMBER    Game Seed, as a decimal number (max: 2147483647)

    -v           Print version info and exit
    -h           Display this message
"#;

// Initialize, restore, and get the ball rolling. -RAK-
fn main() {
    let mut seed: u32 = 0;
    let mut new_game = false;
    let mut roguelike_keys = false;

    // call this routine to grab a file pointer to the high score file
    // and prepare things to relinquish setuid privileges
    if !initialize_score_file() {
        eprintln!("Can't open score file '{}'", config::files::SCORES);
        std::process::exit(1);
    }

    // Make sure we have access to all files -MRC-
    if !check_file_permissions() {
        std::process::exit(1);
    }

    if !terminal_initialize() {
        std::process::exit(1);
    }

    // check for user interface option
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() && args[i].starts_with('-') {
        match args[i].chars().nth(1) {
            Some('v') => {
                terminal_restore();
                println!("{}.{}.{}", CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH);
                std::process::exit(0);
            }
            Some('n') => {
                new_game = true;
            }
            Some('r') => {
                roguelike_keys = true;
            }
            Some('d') => {
                show_scores_screen();
                exit_program();
            }
            Some('s') => {
                // No NUMBER provided?
                if i + 1 >= args.len() {
                    break;
                }

                // Move onto the NUMBER value
                i += 1;

                if !parse_game_seed(&args[i], &mut seed) {
                    terminal_restore();
                    println!("Game seed must be a decimal number between 1 and 2147483647");
                    std::process::exit(-1);
                }
            }
            Some('w') => {
                game().to_be_wizard = true;
            }
            _ => {
                terminal_restore();

                println!("Robert A. Koeneke's classic dungeon crawler.");
                println!(
                    "Umoria {}.{}.{} is released under a GPL-3.0-or-later license.",
                    CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH
                );
                print!("{}", USAGE_INSTRUCTIONS);
                std::process::exit(0);
            }
        }
        i += 1;
    }

    // Auto-restart of saved file
    if i < args.len() {
        config::files::set_save_game(&args[i]);
    }

    start_moria(seed, new_game, roguelike_keys);
}

fn parse_game_seed(argv: &str, seed: &mut u32) -> bool {
    let mut value: i32 = 0;

    if !string_to_number(argv, &mut value) {
        return false;
    }
    if value <= 0 {
        return false;
    }

    *seed = value as u32;

    true
}
