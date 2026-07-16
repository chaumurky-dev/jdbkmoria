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
use jdbkmoria::data_paintings::SWARM_PAINTING_TEMPLATES;
use jdbkmoria::dungeon::{coord_distance_between, coord_in_bounds, dg};
use jdbkmoria::dungeon_generate::generate_cave;
use jdbkmoria::game_run::{initialize_monster_levels, initialize_treasure_levels};
use jdbkmoria::dungeon_tile::{MAX_CAVE_FLOOR, MIN_CAVE_WALL, TILE_BOUNDARY_WALL, TILE_LIGHT_FLOOR};
use jdbkmoria::paintings::{
    painting_from_legacy_save, painting_from_save, painting_index_at, painting_kind_from_u8, painting_kind_to_u8,
    paintings, remove_painting_at, PaintingKind,
};
use jdbkmoria::player::py;
use jdbkmoria::rng::set_random_seed;
use jdbkmoria::types::Coord;

// The Swarm animal sprite set documented in paintings.rs's module header
// ("abclrSw"); it is not exported, so tests mirror the literal here.
const SWARM_SPRITES: &[u8] = b"abclrSw";

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

        // Swarm paintings hold 2..=20 vermin, no loot yet, and a creature
        // whose sprite is in the documented Swarm animal set.
        if painting.kind == PaintingKind::Swarm {
            assert!(
                (2..=20).contains(&painting.items),
                "swarm painting with {} members, expected 2..=20",
                painting.items
            );
            assert_eq!(painting.loot, 0, "a freshly placed swarm painting should owe no loot");
            let creature = &CREATURES_LIST[painting.creature_id as usize];
            assert!(
                SWARM_SPRITES.contains(&creature.sprite),
                "swarm creature {} has sprite {:?}, not in the swarm animal set",
                creature.name,
                creature.sprite as char
            );
            assert!(painting.hp > 0, "swarm painting with no hit points");
        }
    }
    assert_eq!(map_paintings, 1, "expected exactly one level-map painting");

    // On dungeon levels 1-2 the map painting must always hang in a lit
    // room, and it must be VERY close to the character's start: the
    // nearest lit-room hangable wall tile on the whole level, not merely
    // the nearest among the ~50 paintings that happened to get placed
    // (which can be many tiles away in another room on a map this size).
    let is_lit_room_wall = |coord: Coord| -> bool {
        [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dy, dx)| {
            let neighbor = Coord::new(coord.y + dy, coord.x + dx);
            coord_in_bounds(neighbor) && dg().tile(neighbor).feature_id == TILE_LIGHT_FLOOR
        })
    };

    // Mirrors src/paintings.rs's is_hangable_wall: a non-boundary wall
    // tile with at least one orthogonally adjacent floor tile.
    let is_hangable_wall = |coord: Coord| -> bool {
        let feature_id = dg().tile(coord).feature_id;
        if feature_id < MIN_CAVE_WALL || feature_id == TILE_BOUNDARY_WALL {
            return false;
        }
        [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dy, dx)| {
            let neighbor = Coord::new(coord.y + dy, coord.x + dx);
            coord_in_bounds(neighbor) && dg().tile(neighbor).feature_id <= MAX_CAVE_FLOOR
        })
    };

    // Scans the whole level for the lit-room hangable wall tile nearest
    // `pos`, mirroring src/paintings.rs's nearest_lit_hangable_wall.
    let nearest_lit_hangable_wall_distance = |pos: Coord| -> i32 {
        let mut nearest_distance = i32::MAX;
        for y in 1..=(dg().height as i32 - 2) {
            for x in 1..=(dg().width as i32 - 2) {
                let coord = Coord::new(y, x);
                if !is_hangable_wall(coord) || !is_lit_room_wall(coord) {
                    continue;
                }
                nearest_distance = nearest_distance.min(coord_distance_between(pos, coord));
            }
        }
        nearest_distance
    };

    // A generous sanity bound on how far the level 1-2 map painting can
    // land from the start. Observed distances across seeds 1..=30 on both
    // levels: mostly single digits (median 2), with rare outliers up to 24
    // when the character's randomly chosen start tile happens to land deep
    // in a corridor far from any lit room (dungeon_new_spot picks any open
    // floor tile on the level, not necessarily inside a room). 60 is
    // comfortably above the observed max while still catching a regression
    // to "nearest among the ~50 placed paintings", which routinely lands
    // far further away on a map this size.
    const MAP_PAINTING_MAX_DISTANCE: i32 = 60;

    let map_painting = *paintings().iter().find(|p| p.kind == PaintingKind::LevelMap).unwrap();
    assert!(
        is_lit_room_wall(map_painting.pos),
        "the level-map painting at ({}, {}) does not hang in a lit room",
        map_painting.pos.y,
        map_painting.pos.x
    );

    let map_distance = coord_distance_between(py().pos, map_painting.pos);
    assert_eq!(
        map_distance,
        nearest_lit_hangable_wall_distance(py().pos),
        "the level-map painting at ({}, {}) is not on the lit-room hangable wall nearest the start",
        map_painting.pos.y,
        map_painting.pos.x
    );
    assert!(
        map_distance <= MAP_PAINTING_MAX_DISTANCE,
        "level-1 map painting distance {} exceeds the sanity bound of {}",
        map_distance,
        MAP_PAINTING_MAX_DISTANCE
    );

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
        PaintingKind::Swarm,
    ] {
        assert_eq!(painting_kind_from_u8(painting_kind_to_u8(kind)), Some(kind));
    }
    assert_eq!(painting_kind_from_u8(9), None);

    // Legacy (pre-creature) save records convert: monster paintings get a
    // real creature picked deterministically, FakeLoot gains a trap subtype.
    let legacy = painting_from_legacy_save(Coord::new(5, 5), 1, 3, 30, 0, 1).unwrap();
    assert_eq!(legacy.kind, PaintingKind::MeleeMonster);
    assert!((legacy.creature_id as usize) < CREATURES_LIST.len());
    assert!(CREATURES_LIST[legacy.creature_id as usize].level >= 1);
    assert!(legacy.awake);
    // `loot` is a v2 invention: legacy records never carry it.
    assert_eq!(legacy.loot, 0);

    let legacy_trap = painting_from_legacy_save(Coord::new(5, 6), 4, 3, 10, 0, 0).unwrap();
    assert_eq!(legacy_trap.kind, PaintingKind::FakeLoot);
    assert_eq!(legacy_trap.items, 1); // desc_id 3 -> poisoned spikes
    assert_eq!(legacy_trap.loot, 0);

    assert!(painting_from_legacy_save(Coord::new(5, 7), 200, 0, 0, 0, 0).is_none());

    // painting_from_save: v2 Swarm records. Pick a real vermin creature --
    // one whose sprite is in the documented Swarm animal set -- so the
    // "valid" record below is not just in-range but thematically real.
    let swarm_creature_id = CREATURES_LIST
        .iter()
        .position(|creature| SWARM_SPRITES.contains(&creature.sprite))
        .expect("expected at least one swarm-eligible creature in CREATURES_LIST") as u16;

    let swarm_kind_byte = painting_kind_to_u8(PaintingKind::Swarm);
    let valid_swarm = painting_from_save(Coord::new(6, 6), swarm_kind_byte, 0, swarm_creature_id, 30, 5, 0, 7)
        .expect("a valid v2 Swarm record should be accepted");
    assert_eq!(valid_swarm.kind, PaintingKind::Swarm);
    assert_eq!(valid_swarm.creature_id, swarm_creature_id);
    assert_eq!(valid_swarm.items, 5);
    assert_eq!(valid_swarm.hp, 30);
    assert_eq!(valid_swarm.loot, 7, "the loot byte should be preserved for a v2 record");

    // Out-of-range creature_id is rejected for a Swarm record.
    assert!(painting_from_save(
        Coord::new(6, 6),
        swarm_kind_byte,
        0,
        CREATURES_LIST.len() as u16,
        30,
        5,
        0,
        7,
    )
    .is_none());

    // Out-of-range desc_id is rejected for a Swarm record.
    assert!(painting_from_save(
        Coord::new(6, 6),
        swarm_kind_byte,
        SWARM_PAINTING_TEMPLATES.len() as u8,
        swarm_creature_id,
        30,
        5,
        0,
        7,
    )
    .is_none());

    // Backward compat: painting_from_save also serves v1 records (creature
    // id present, but no loot byte on disk -- game_save.rs's restore path
    // always passes loot = 0 for `is_v1`). The rebuilt painting has loot 0.
    let v1_style = painting_from_save(Coord::new(7, 7), painting_kind_to_u8(PaintingKind::MeleeMonster), 0, 10, 20, 0, 1, 0)
        .expect("a v1-style MeleeMonster record should be accepted");
    assert_eq!(v1_style.kind, PaintingKind::MeleeMonster);
    assert_eq!(v1_style.loot, 0);

    // On dungeon level 2 (each room lit with 24/25 probability, so this is
    // not a certainty per room but should hold across a handful of seeds),
    // the map painting must also hang in a lit room, on the lit-room
    // hangable wall nearest the start, and within the same sanity bound.
    for seed in [1u32, 2, 3, 4, 5] {
        set_random_seed(seed);
        dg().current_level = 2;
        generate_cave();

        let map_painting = *paintings().iter().find(|p| p.kind == PaintingKind::LevelMap).unwrap();
        assert!(
            is_lit_room_wall(map_painting.pos),
            "seed {}: the level-2 map painting at ({}, {}) does not hang in a lit room",
            seed,
            map_painting.pos.y,
            map_painting.pos.x
        );

        let map_distance = coord_distance_between(py().pos, map_painting.pos);
        assert_eq!(
            map_distance,
            nearest_lit_hangable_wall_distance(py().pos),
            "seed {}: the level-2 map painting at ({}, {}) is not on the lit-room hangable wall nearest the start",
            seed,
            map_painting.pos.y,
            map_painting.pos.x
        );
        assert!(
            map_distance <= MAP_PAINTING_MAX_DISTANCE,
            "seed {}: level-2 map painting distance {} exceeds the sanity bound of {}",
            seed,
            map_distance,
            MAP_PAINTING_MAX_DISTANCE
        );
    }

    // Swarm placement across several generated levels/seeds: a Swarm roll
    // is only ~5% per painting slot, so pool several regenerated levels
    // rather than relying on a single seed to produce one.
    let mut swarm_paintings_seen = 0;
    for (seed, level) in [(1u32, 3i16), (7, 5), (99, 8), (555, 12), (2024, 20), (77_777, 15)] {
        set_random_seed(seed);
        dg().current_level = level;
        generate_cave();

        for painting in paintings().iter() {
            if painting.kind != PaintingKind::Swarm {
                continue;
            }
            swarm_paintings_seen += 1;

            assert!(
                (2..=20).contains(&painting.items),
                "swarm painting with {} members, expected 2..=20",
                painting.items
            );
            assert_eq!(painting.loot, 0, "a freshly placed swarm painting should owe no loot");
            assert!(painting.hp > 0, "swarm painting with no hit points");

            let creature = &CREATURES_LIST[painting.creature_id as usize];
            assert!(
                SWARM_SPRITES.contains(&creature.sprite),
                "swarm creature {} has sprite {:?}, not in the swarm animal set",
                creature.name,
                creature.sprite as char
            );
        }
    }
    assert!(swarm_paintings_seen > 0, "expected at least one Swarm painting across several levels/seeds");
}
