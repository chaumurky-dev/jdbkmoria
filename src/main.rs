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
use jdbkmoria::version::{JDBK_VERSION_MAJOR, JDBK_VERSION_MINOR, JDBK_VERSION_PATCH};

const USAGE_INSTRUCTIONS: &str = r#"
Usage:
    jdbkmoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
    -n           Force start of new game
    -r           Enable classic roguelike keys on startup (default: disabled, or save game settings)
    -d           Display high scores and exit
    -s NUMBER    Game Seed, as a decimal number (max: 2147483647)
    -l LOCALE    Locale (en_US, en_GB, fr_CA); overrides JDBKMORIA_LOCALE
                 and the system locale (LC_ALL/LC_MESSAGES/LANG)
    -W COLSxLINES Force a specific terminal window size (default: auto-fit to terminal, min 80x24)

    -v           Print version info and exit
    -h           Display this message
"#;

// Initialize, restore, and get the ball rolling. -RAK-
fn main() {
    let mut seed: u32 = 0;
    let mut new_game = false;
    let mut roguelike_keys = false;
    let mut window_size: Option<(i32, i32)> = None;
    let mut show_scores = false;

    // jdbkmoria extension: pick up the locale from JDBKMORIA_LOCALE or the
    // system environment before anything prints; `-l` below overrides it.
    if let Err(bad_tag) = jdbkmoria::locale::initialize_locale(None) {
        eprintln!(
            "{}",
            jdbkmoria::tr_fmt!(
                "Unsupported JDBKMORIA_LOCALE '{}' (supported: en_US, en_GB, fr_CA); using en_US",
                bad_tag
            )
        );
    }

    // call this routine to grab a file pointer to the high score file
    // and prepare things to relinquish setuid privileges
    if !initialize_score_file() {
        eprintln!(
            "{}",
            jdbkmoria::tr_fmt!("Can't open score file '{}'", config::files::SCORES)
        );
        std::process::exit(1);
    }

    // Make sure we have access to all files -MRC-
    if !check_file_permissions() {
        std::process::exit(1);
    }

    // check for user interface option
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() && args[i].starts_with('-') {
        match args[i].chars().nth(1) {
            Some('v') => {
                terminal_restore();
                println!("{}.{}.{}", JDBK_VERSION_MAJOR, JDBK_VERSION_MINOR, JDBK_VERSION_PATCH);
                std::process::exit(0);
            }
            Some('n') => {
                new_game = true;
            }
            Some('r') => {
                roguelike_keys = true;
            }
            Some('d') => {
                show_scores = true;
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
                    println!(
                        "{}",
                        jdbkmoria::tr!("Game seed must be a decimal number between 1 and 2147483647")
                    );
                    std::process::exit(-1);
                }
            }
            Some('W') => {
                // No COLSxLINES provided?
                if i + 1 >= args.len() {
                    break;
                }
                i += 1;
                if let Some((cols, lines)) = parse_window_size(&args[i]) {
                    window_size = Some((cols, lines));
                } else {
                    terminal_restore();
                    println!("{}", jdbkmoria::tr!("Window size must be COLSxLINES, e.g. -W 120x50"));
                    std::process::exit(-1);
                }
            }
            Some('w') => {
                game().to_be_wizard = true;
            }
            Some('l') => {
                // No LOCALE provided?
                if i + 1 >= args.len() {
                    break;
                }

                // Move onto the LOCALE value
                i += 1;

                if jdbkmoria::locale::initialize_locale(Some(&args[i])).is_err() {
                    terminal_restore();
                    println!(
                        "{}",
                        jdbkmoria::tr_fmt!(
                            "Unsupported locale '{}' (supported: en_US, en_GB, fr_CA)",
                            args[i]
                        )
                    );
                    std::process::exit(-1);
                }
            }
            _ => {
                terminal_restore();

                println!(
                    "{}",
                    jdbkmoria::tr_fmt!(
                        "jdbkmoria {}.{}.{}: An expanded edition of Robert A. Koeneke's classic dungeon crawler.",
                        JDBK_VERSION_MAJOR,
                        JDBK_VERSION_MINOR,
                        JDBK_VERSION_PATCH
                    )
                );
                println!(
                    "{}",
                    jdbkmoria::tr!("Based on Umoria 5.7.15, released under a GPL-3.0-or-later license.")
                );
                print!("{}", jdbkmoria::tr!(USAGE_INSTRUCTIONS));
                std::process::exit(0);
            }
        }
        i += 1;
    }

    // Auto-restart of saved file
    if i < args.len() {
        config::files::set_save_game(&args[i]);
    }

    if !terminal_initialize(window_size) {
        std::process::exit(1);
    }

    if show_scores {
        show_scores_screen();
        exit_program();
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

// Parses a "COLSxLINES" window-size string, e.g. "120x50".
fn parse_window_size(argv: &str) -> Option<(i32, i32)> {
    let (cols_str, lines_str) = argv.split_once('x')?;

    let mut cols = 0;
    let mut lines = 0;
    if !string_to_number(cols_str, &mut cols) || !string_to_number(lines_str, &mut lines) {
        return None;
    }
    if cols <= 0 || lines <= 0 {
        return None;
    }

    Some((cols, lines))
}
