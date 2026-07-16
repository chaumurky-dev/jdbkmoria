// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Initialization, main() function and main loop

use jdbkmoria::config;
use jdbkmoria::game::{exit_program, game};
use jdbkmoria::game_files::initialize_score_file;
use jdbkmoria::game_run::start_moria;
use jdbkmoria::helpers::string_to_number;
use jdbkmoria::scores::show_scores_screen;
use jdbkmoria::ui_io::{check_file_permissions, terminal_initialize, terminal_restore};
use jdbkmoria::version::{CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, CURRENT_VERSION_PATCH};

const USAGE_INSTRUCTIONS: &str = r#"
Usage:
    jdbkmoria [OPTIONS] SAVEGAME

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
