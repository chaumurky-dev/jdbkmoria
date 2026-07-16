// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Integration test for the save/load machinery, exercised without a
// terminal via the non-interactive `save_game_state_to_file()` /
// `load_game_state_from_file()` helpers exposed in `game_save.rs`.
//
// The game's global state is process-wide (see `globals.rs`), so everything
// here runs in a single #[test] fn to avoid interference from parallel test
// execution. The `rng.rs` unit test runs in its own `cargo test` process and
// so cannot interfere with this one either.

use jdbkmoria::dungeon::dg;
use jdbkmoria::game::game;
use jdbkmoria::game_save::{load_game_state_from_file, save_game_state_to_file};
use jdbkmoria::inventory::inventory_item_copy_to;
use jdbkmoria::paintings::{paintings, Painting, PaintingKind};
use jdbkmoria::player::py;
use jdbkmoria::recall_data::creature_recall;
use jdbkmoria::rng::set_random_seed;
use jdbkmoria::types::Coord;

#[test]
fn save_and_load_roundtrip() {
    // (a) seed the RNG - save_game_state_to_file() draws one random byte
    // (the xor seed) from it, exactly like the real save_char() does.
    set_random_seed(123_456_789);

    // (b) poke known values into the global state.
    py().misc.name = "TestHero".to_string();
    py().misc.gender = true;
    py().misc.level = 7;
    py().misc.au = 4242;
    py().misc.max_hp = 55;
    // Keep current_hp negative so the restore path's "(alive and well)"
    // overwrite of character_died_from does not fire (see game_save.rs,
    // restore_from_file's final `if py().misc.current_hp >= 0` check),
    // letting us verify our own character_died_from string round-trips.
    py().misc.current_hp = -3;
    py().misc.history[0] = "Once, an ordinary test fixture...".to_string();

    // A couple of pack inventory items. Nothing is put into the equipment
    // slots (WIELD..) - player_strength(), which restore_from_file() calls
    // unconditionally on a full restoration, would otherwise print a
    // curses message about wielding a weapon too heavy for a 0-strength
    // test character.
    py().pack.unique_items = 2;
    inventory_item_copy_to(1, &mut py().inventory[0]);
    py().inventory[0].items_count = 3;
    inventory_item_copy_to(2, &mut py().inventory[1]);
    py().inventory[1].items_count = 1;

    game().character_died_from = "a test scenario".to_string();
    game().character_is_dead = false;
    game().total_winner = false;
    game().magic_seed = 111;
    game().town_seed = 222;

    dg().game_turn = 500;

    creature_recall()[5].kills = 9;
    creature_recall()[5].movement = 0x1234;
    creature_recall()[5].wake = 7;

    // Two paintings (jdbkmoria extension block appended to the save format).
    paintings().clear();
    paintings().push(Painting {
        pos: Coord::new(10, 20),
        kind: PaintingKind::RangedMonster,
        desc_id: 3,
        creature_id: 42,
        hp: 42,
        items: 0,
        awake: true,
        found: false,
        loot: 0,
    });
    paintings().push(Painting {
        // x = 300 exceeds u8::MAX (255): this exercises the save format's
        // widened short coordinates, needed now that MAX_WIDTH (396) no
        // longer fits a byte (see game_save.rs's wr_short/rd_short).
        pos: Coord::new(33, 300),
        kind: PaintingKind::Loot,
        desc_id: 5,
        creature_id: 0,
        hp: 7,
        items: 2,
        awake: false,
        found: true,
        loot: 0,
    });
    // A Swarm painting (v2 kind byte 8) with nonzero pending loot -- treasure
    // grabs owed by members already slain inside the canvas (see
    // painting_add_loot_drop / painting_after_monster_leaves in
    // src/paintings.rs). The v2 record format is the only one with a loot
    // byte at all, so this value surviving the round trip is itself proof
    // that the header byte written was 0xC0 | count: game_save.rs's restore
    // path only reads a loot byte (and only accepts kind byte 8) when
    // `header & 0xC0 == 0xC0`; for a v1 or legacy header, loot is forced to
    // 0 regardless of what is on disk (see the `is_v2` gate around
    // `painting_from_save`'s `loot` parameter).
    paintings().push(Painting {
        pos: Coord::new(44, 60),
        kind: PaintingKind::Swarm,
        desc_id: 2,
        creature_id: 17,
        hp: 12,
        items: 9,
        awake: true,
        found: false,
        loot: 5,
    });

    // Write the save file into a tempdir with a unique name.
    let mut path = std::env::temp_dir();
    path.push(format!("jdbkmoria_save_roundtrip_{}.sav", std::process::id()));
    let path_str = path.to_str().expect("temp path should be valid UTF-8");

    // (c) save.
    assert!(save_game_state_to_file(path_str), "save_game_state_to_file() failed");

    // (d) clobber the state so the load below can't trivially "pass" by
    // reading back values that were never overwritten.
    py().misc.name.clear();
    py().misc.gender = false;
    py().misc.level = 0;
    py().misc.au = 0;
    py().misc.max_hp = 0;
    py().misc.current_hp = 0;
    py().misc.history[0].clear();
    py().pack.unique_items = 0;
    py().inventory[0] = Default::default();
    py().inventory[1] = Default::default();
    game().character_died_from.clear();
    game().magic_seed = 0;
    game().town_seed = 0;
    dg().game_turn = -1;
    creature_recall()[5].kills = 0;
    creature_recall()[5].movement = 0;
    creature_recall()[5].wake = 0;
    paintings().clear();

    // (e) load.
    let mut generate = true;
    let result = load_game_state_from_file(path_str, &mut generate);

    // Clean up the temp file regardless of assertion outcome below.
    let _ = std::fs::remove_file(&path);

    // (f) assert the values round-tripped.
    assert_eq!(result, Some(true), "expected a full restoration");
    assert!(!generate, "a full cave was restored, so generate should be false");

    assert_eq!(py().misc.name, "TestHero");
    assert!(py().misc.gender);
    assert_eq!(py().misc.level, 7);
    assert_eq!(py().misc.au, 4242);
    assert_eq!(py().misc.max_hp, 55);
    assert_eq!(py().misc.current_hp, -3);
    assert_eq!(py().misc.history[0], "Once, an ordinary test fixture...");

    assert_eq!(py().pack.unique_items, 2);
    assert_eq!(py().inventory[0].id, 1);
    assert_eq!(py().inventory[0].items_count, 3);
    assert_eq!(py().inventory[1].id, 2);
    assert_eq!(py().inventory[1].items_count, 1);

    assert_eq!(game().character_died_from, "a test scenario");
    assert_eq!(game().magic_seed, 111);
    assert_eq!(game().town_seed, 222);

    assert_eq!(dg().game_turn, 500);

    assert_eq!(creature_recall()[5].kills, 9);
    assert_eq!(creature_recall()[5].movement, 0x1234);
    assert_eq!(creature_recall()[5].wake, 7);

    assert_eq!(paintings().len(), 3);
    assert_eq!(paintings()[0].pos.y, 10);
    assert_eq!(paintings()[0].pos.x, 20);
    assert_eq!(paintings()[0].kind, PaintingKind::RangedMonster);
    assert_eq!(paintings()[0].desc_id, 3);
    assert_eq!(paintings()[0].creature_id, 42);
    assert_eq!(paintings()[0].hp, 42);
    assert_eq!(paintings()[0].items, 0);
    assert!(paintings()[0].awake);
    assert!(!paintings()[0].found);
    assert_eq!(paintings()[1].pos.y, 33);
    assert_eq!(paintings()[1].pos.x, 300);
    assert_eq!(paintings()[1].kind, PaintingKind::Loot);
    assert_eq!(paintings()[1].desc_id, 5);
    assert_eq!(paintings()[1].creature_id, 0);
    assert_eq!(paintings()[1].hp, 7);
    assert_eq!(paintings()[1].items, 2);
    assert!(!paintings()[1].awake);
    assert!(paintings()[1].found);
    assert_eq!(paintings()[1].loot, 0);

    assert_eq!(paintings()[2].pos.y, 44);
    assert_eq!(paintings()[2].pos.x, 60);
    assert_eq!(paintings()[2].kind, PaintingKind::Swarm);
    assert_eq!(paintings()[2].desc_id, 2);
    assert_eq!(paintings()[2].creature_id, 17);
    assert_eq!(paintings()[2].hp, 12);
    assert_eq!(paintings()[2].items, 9);
    assert!(paintings()[2].awake);
    assert!(!paintings()[2].found);
    assert_eq!(paintings()[2].loot, 5, "nonzero loot must survive the v2 round trip");
}
