// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Integration test for painting placement (an jdbkmoria extension, see
// src/paintings.rs), exercised on a real generated dungeon level; the
// generation path performs no curses calls, so this runs without a
// terminal. As with save_roundtrip.rs, the game state is process-wide, so
// everything runs in a single #[test] fn.

use jdbkmoria::data_creatures::CREATURES_LIST;
use jdbkmoria::dungeon::{coord_distance_between, dg};
use jdbkmoria::dungeon_generate::generate_cave;
use jdbkmoria::game_run::{initialize_monster_levels, initialize_treasure_levels};
use jdbkmoria::dungeon_tile::{MAX_CAVE_FLOOR, MIN_CAVE_WALL, TILE_BOUNDARY_WALL};
use jdbkmoria::paintings::{
    painting_from_legacy_save, painting_index_at, painting_kind_from_u8, painting_kind_to_u8,
    paintings, remove_painting_at, PaintingKind,
};
use jdbkmoria::player::py;
use jdbkmoria::rng::set_random_seed;
use jdbkmoria::types::Coord;

#[test]
fn painting_placement() {
    set_random_seed(42);

    // The allocation tables normally built during game startup.
    initialize_monster_levels();
    initialize_treasure_levels();

    // Generate a real level-1 dungeon; dungeon_generate() hangs the
    // paintings after placing the character's starting position.
    dg().current_level = 1;
    generate_cave();

    // Around fifty paintings per level.
    let count = paintings().len();
    assert!(
        (46..=56).contains(&count),
        "expected 46..=56 paintings, got {}",
        count
    );

    let mut map_paintings = 0;
    for painting in paintings().iter() {
        // Every painting hangs on a non-boundary wall tile...
        let feature_id = dg().tile(painting.pos).feature_id;
        assert!(feature_id >= MIN_CAVE_WALL && feature_id != TILE_BOUNDARY_WALL);

        // ...that faces at least one orthogonally adjacent floor tile.
        let mut faces_floor = false;
        for (dy, dx) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let neighbor = Coord::new(painting.pos.y + dy, painting.pos.x + dx);
            if dg().tile(neighbor).feature_id <= MAX_CAVE_FLOOR {
                faces_floor = true;
            }
        }
        assert!(faces_floor, "painting at ({}, {}) faces no floor", painting.pos.y, painting.pos.x);

        // No two paintings share a tile: the index lookup finds each
        // painting at its own coordinate.
        assert_eq!(painting_index_at(painting.pos).map(|i| paintings()[i].pos), Some(painting.pos));

        if painting.kind == PaintingKind::LevelMap {
            map_paintings += 1;
        }

        // Monster paintings carry a real dungeon creature.
        if painting.kind == PaintingKind::MeleeMonster || painting.kind == PaintingKind::RangedMonster {
            let creature = &CREATURES_LIST[painting.creature_id as usize];
            assert!(creature.level >= 1, "town creature {} in a painting", creature.name);
            assert!(painting.hp > 0, "monster painting with no hit points");
        }
    }
    assert_eq!(map_paintings, 1, "expected exactly one level-map painting");

    // On dungeon level 1 the map painting is the one nearest the start.
    let map_painting = *paintings().iter().find(|p| p.kind == PaintingKind::LevelMap).unwrap();
    let map_distance = coord_distance_between(py().pos, map_painting.pos);
    for painting in paintings().iter() {
        assert!(
            coord_distance_between(py().pos, painting.pos) >= map_distance,
            "a painting is closer to the start than the level map"
        );
    }

    // Registry helpers.
    let pos = paintings()[0].pos;
    assert!(remove_painting_at(pos));
    assert!(painting_index_at(pos).is_none());
    assert!(!remove_painting_at(pos));
    assert_eq!(paintings().len(), count - 1);

    // Kind byte serialization round-trips, and rejects garbage.
    for kind in [
        PaintingKind::Harmless,
        PaintingKind::MeleeMonster,
        PaintingKind::RangedMonster,
        PaintingKind::Loot,
        PaintingKind::FakeLoot,
        PaintingKind::Teleport,
        PaintingKind::LevelMap,
        PaintingKind::SleepGas,
    ] {
        assert_eq!(painting_kind_from_u8(painting_kind_to_u8(kind)), Some(kind));
    }
    assert_eq!(painting_kind_from_u8(8), None);

    // Legacy (pre-creature) save records convert: monster paintings get a
    // real creature picked deterministically, FakeLoot gains a trap subtype.
    let legacy = painting_from_legacy_save(Coord::new(5, 5), 1, 3, 30, 0, 1).unwrap();
    assert_eq!(legacy.kind, PaintingKind::MeleeMonster);
    assert!((legacy.creature_id as usize) < CREATURES_LIST.len());
    assert!(CREATURES_LIST[legacy.creature_id as usize].level >= 1);
    assert!(legacy.awake);

    let legacy_trap = painting_from_legacy_save(Coord::new(5, 6), 4, 3, 10, 0, 0).unwrap();
    assert_eq!(legacy_trap.kind, PaintingKind::FakeLoot);
    assert_eq!(legacy_trap.items, 1); // desc_id 3 -> poisoned spikes

    assert!(painting_from_legacy_save(Coord::new(5, 7), 200, 0, 0, 0, 0).is_none());
}
