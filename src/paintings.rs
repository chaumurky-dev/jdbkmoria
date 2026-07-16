// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Paintings on dungeon walls: an jdbkmoria extension, not present in the
// original umoria sources.
//
// Each dungeon level hangs around fifty paintings on its wall tiles. They
// are kept in a small per-level registry keyed by coordinate rather than in
// the Tile struct, because the save-file format packs `feature_id` into four
// bits and leaves no room for a new wall type. The tile beneath a painting
// stays an ordinary wall: destroying a painting (by bash, missile, or
// wall-destroying magic) simply removes the registry entry and the wall
// remains.
//
// Painting kinds:
// - Harmless: flavor only (roughly half of all paintings; a few secretly
//   hold a single trinket).
// - MeleeMonster: a real, level-appropriate creature dozes in the canvas.
//   It stays dormant until the character looks at it up close (or strikes
//   the painting), then tears free and fights as an ordinary monster.
// - RangedMonster: a spell-casting creature that stays in its canvas and
//   hurls bolts of force once roused by a look or a blow.
// - Loot: may be reached into for one to three items.
// - FakeLoot: looks exactly like Loot but bites (fangs) or stabs (poisoned
//   spikes, chosen by `items`); trap detection reveals it.
// - Teleport: looking at it flings the player across the level.
// - SleepGas: looking at it lulls the player into an enchanted sleep.
// - LevelMap: exactly one per level; looking at it reveals the level map.
//   On dungeon level 1 it is the painting nearest the character's arrival
//   position.
//
// Any painting that is not the level map and holds no monster can be
// reached into, but at least half of all reaches come away empty-handed.
// Monster paintings take their toughness (hit points, armor class,
// experience) from the creature inside; bolts, balls, and thrown missiles
// can strike them, and a canvas whose monster has been defeated (or has
// leapt out) settles into a new scene that may still hold treasure or a
// trap.

use crate::config::monsters::{defense, move_flags, spells as monster_spells, MON_MAX_SIGHT};
use crate::data_creatures::CREATURES_LIST;
use crate::data_paintings::{
    HARMLESS_PAINTINGS, LOOT_PAINTINGS, MAP_PAINTING, MELEE_PAINTING_TEMPLATES,
    RANGED_PAINTING_TEMPLATES, SLEEP_PAINTINGS, TELEPORT_PAINTINGS,
};
use crate::dice::{dice_roll, max_dice_roll, Dice};
use crate::dungeon::{
    cave_tile_visible, coord_distance_between, coord_in_bounds, dg, dungeon_lite_spot,
};
use crate::dungeon_tile::{MAX_CAVE_FLOOR, MIN_CAVE_WALL, TILE_BOUNDARY_WALL};
use crate::game::{game, random_number, sorted_objects};
use crate::game_objects::{item_get_random_object_id, popt, pusht};
use crate::globals::RacyCell;
use crate::identification::item_description;
use crate::inventory::{inventory_can_carry_item_count, inventory_carry_item, inventory_item_copy_to};
use crate::monster::{monster_update_visibility, monsters, Creature};
use crate::monster_manager::place_monster_adjacent_to;
use crate::player::{
    py, player_disturb, player_takes_hit, player_test_being_hit, player_weapon_critical_blow,
    A_DEX, A_STR, CLASS_BTH,
};
use crate::types::Coord;
use crate::ui::{coord_inside_panel, display_character_experience, draw_dungeon_panel, ESCAPE};
use crate::ui_io::{
    get_input_confirmation, get_key_input, panel_move_cursor, print_message,
    put_string_clear_to_eol,
};

// Hard cap on the registry (the target count is 46-56 per level). Must stay
// below 0x80: the save format packs a version flag into the count byte.
pub const MAX_PAINTINGS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintingKind {
    Harmless,
    MeleeMonster,
    RangedMonster,
    Loot,
    FakeLoot,
    Teleport,
    LevelMap,
    SleepGas,
}

#[derive(Debug, Clone, Copy)]
pub struct Painting {
    pub pos: Coord,
    pub kind: PaintingKind,
    pub desc_id: u8,      // index into the matching data_paintings table
    pub creature_id: u16, // monster paintings: index into CREATURES_LIST
    pub hp: i16,          // durability; for monster paintings, the creature's hit points
    pub items: u8,        // Loot/Harmless: remaining grabs; FakeLoot: 0 fangs, 1 spikes
    pub awake: bool,      // monster paintings: roused and fighting
    pub found: bool,      // FakeLoot/Teleport/SleepGas: nature revealed
}

// What became of a painting struck by a bash, bolt, or missile. Monster
// outcomes print their own messages; the caller narrates the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintingHitResult {
    MonsterKilled,
    MonsterAlive,
    Destroyed,
    Damaged,
}

static PAINTINGS: RacyCell<Vec<Painting>> = RacyCell::new(Vec::new());

pub fn paintings() -> &'static mut Vec<Painting> {
    PAINTINGS.get()
}

pub fn painting_index_at(coord: Coord) -> Option<usize> {
    paintings().iter().position(|p| p.pos.y == coord.y && p.pos.x == coord.x)
}

// Serialization helpers for game_save.rs.
pub fn painting_kind_to_u8(kind: PaintingKind) -> u8 {
    match kind {
        PaintingKind::Harmless => 0,
        PaintingKind::MeleeMonster => 1,
        PaintingKind::RangedMonster => 2,
        PaintingKind::Loot => 3,
        PaintingKind::FakeLoot => 4,
        PaintingKind::Teleport => 5,
        PaintingKind::LevelMap => 6,
        PaintingKind::SleepGas => 7,
    }
}

pub fn painting_kind_from_u8(value: u8) -> Option<PaintingKind> {
    match value {
        0 => Some(PaintingKind::Harmless),
        1 => Some(PaintingKind::MeleeMonster),
        2 => Some(PaintingKind::RangedMonster),
        3 => Some(PaintingKind::Loot),
        4 => Some(PaintingKind::FakeLoot),
        5 => Some(PaintingKind::Teleport),
        6 => Some(PaintingKind::LevelMap),
        7 => Some(PaintingKind::SleepGas),
        _ => None,
    }
}

fn painting_kind_table_len(kind: PaintingKind) -> usize {
    match kind {
        PaintingKind::Harmless => HARMLESS_PAINTINGS.len(),
        PaintingKind::MeleeMonster => MELEE_PAINTING_TEMPLATES.len(),
        PaintingKind::RangedMonster => RANGED_PAINTING_TEMPLATES.len(),
        PaintingKind::Loot | PaintingKind::FakeLoot => LOOT_PAINTINGS.len(),
        PaintingKind::Teleport => TELEPORT_PAINTINGS.len(),
        PaintingKind::SleepGas => SLEEP_PAINTINGS.len(),
        PaintingKind::LevelMap => 1,
    }
}

// Rebuild one painting from its save-file fields, validating everything
// that would otherwise be used as an array index. Returns None on any
// out-of-range value so a corrupt save fails the restore cleanly.
pub fn painting_from_save(pos: Coord, kind_byte: u8, desc_id: u8, creature_id: u16, hp: i16, items: u8, flags: u8) -> Option<Painting> {
    let kind = painting_kind_from_u8(kind_byte)?;

    if desc_id as usize >= painting_kind_table_len(kind) {
        return None;
    }

    let is_monster = kind == PaintingKind::MeleeMonster || kind == PaintingKind::RangedMonster;
    if is_monster && creature_id as usize >= CREATURES_LIST.len() {
        return None;
    }

    Some(Painting {
        pos,
        kind,
        desc_id,
        creature_id,
        hp,
        items,
        awake: (flags & 0x1) != 0,
        found: (flags & 0x2) != 0,
    })
}

// Rebuild one painting from the original (pre-creature) save-file record.
// Monster paintings had no creature then, so pick one deterministically
// from the fields at hand; the RNG must not be touched during a restore.
pub fn painting_from_legacy_save(pos: Coord, kind_byte: u8, desc_id: u8, hp: i16, items: u8, flags: u8) -> Option<Painting> {
    let kind = painting_kind_from_u8(kind_byte)?;

    let mut painting = Painting {
        pos,
        kind,
        desc_id,
        creature_id: 0,
        hp,
        items,
        awake: (flags & 0x1) != 0,
        found: (flags & 0x2) != 0,
    };

    match kind {
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster => {
            let ranged = kind == PaintingKind::RangedMonster;
            let candidates = painting_creature_candidates(level(), ranged);
            match candidates.get(desc_id as usize % candidates.len().max(1)) {
                Some(&creature_id) => {
                    painting.creature_id = creature_id;
                    painting.desc_id = desc_id % painting_kind_table_len(kind) as u8;
                }
                None => {
                    // No candidate creature at all: hang a harmless scene.
                    painting.kind = PaintingKind::Harmless;
                    painting.desc_id = desc_id % HARMLESS_PAINTINGS.len() as u8;
                    painting.awake = false;
                }
            }
        }
        PaintingKind::FakeLoot => {
            // `items` was unused for FakeLoot; it now selects the trap.
            painting.items = desc_id % 2;
            if desc_id as usize >= painting_kind_table_len(kind) {
                return None;
            }
        }
        _ => {
            if desc_id as usize >= painting_kind_table_len(kind) {
                return None;
            }
        }
    }

    Some(painting)
}

fn level() -> i32 {
    dg().current_level as i32
}

// "a Giant Frog" / "an Ogre" -- the creature's name with its article, for
// splicing into the description templates.
fn creature_indefinite_name(creature_id: u16) -> String {
    let name = CREATURES_LIST[creature_id as usize].name;
    let article = match name.chars().next() {
        Some('A' | 'E' | 'I' | 'O' | 'U') => "an",
        _ => "a",
    };
    format!("{} {}", article, name)
}

// The name used in combat and death messages for a monster painting.
fn painted_name(painting: &Painting) -> String {
    format!("painted {}", CREATURES_LIST[painting.creature_id as usize].name)
}

fn painting_description(painting: &Painting) -> String {
    match painting.kind {
        PaintingKind::Harmless => HARMLESS_PAINTINGS[painting.desc_id as usize].to_string(),
        PaintingKind::MeleeMonster => {
            MELEE_PAINTING_TEMPLATES[painting.desc_id as usize].replace("{}", &creature_indefinite_name(painting.creature_id))
        }
        PaintingKind::RangedMonster => {
            RANGED_PAINTING_TEMPLATES[painting.desc_id as usize].replace("{}", &creature_indefinite_name(painting.creature_id))
        }
        PaintingKind::Loot | PaintingKind::FakeLoot => LOOT_PAINTINGS[painting.desc_id as usize].to_string(),
        PaintingKind::Teleport => TELEPORT_PAINTINGS[painting.desc_id as usize].to_string(),
        PaintingKind::SleepGas => SLEEP_PAINTINGS[painting.desc_id as usize].to_string(),
        PaintingKind::LevelMap => MAP_PAINTING.to_string(),
    }
}

// A creature that can step out of its frame and fight hand to hand.
fn creature_can_leave_canvas(creature: &Creature) -> bool {
    (creature.movement & move_flags::CM_MOVE_NORMAL) != 0 && (creature.movement & move_flags::CM_ONLY_MAGIC) == 0
}

// A creature with a damaging ranged attack; it fights from the canvas.
// The breath bits only mean resistance when no cast frequency is set.
fn creature_fires_from_canvas(creature: &Creature) -> bool {
    (creature.spells & monster_spells::CS_FREQ) != 0
        && (creature.spells
            & (monster_spells::CS_LGHT_WND | monster_spells::CS_SER_WND | monster_spells::CS_DRAIN_MANA | monster_spells::CS_BREATHE))
            != 0
}

// Creatures suitable for a monster painting near the given dungeon level,
// widening the level band until something qualifies. Never the Balrog, and
// never the level-0 town folk.
fn painting_creature_candidates(level: i32, ranged: bool) -> Vec<u16> {
    let mut low = level - 3;
    let mut high = level + 2;

    loop {
        let mut candidates = Vec::new();

        for (id, creature) in CREATURES_LIST.iter().enumerate() {
            if (creature.movement & move_flags::CM_WIN) != 0 {
                continue;
            }

            let suits = if ranged {
                creature_fires_from_canvas(creature)
            } else {
                creature_can_leave_canvas(creature) && !creature_fires_from_canvas(creature)
            };

            let creature_level = creature.level as i32;
            if suits && creature_level >= low.max(1) && creature_level <= high {
                candidates.push(id as u16);
            }
        }

        if !candidates.is_empty() || (low <= 1 && high >= 127) {
            return candidates;
        }

        low -= 3;
        high += 2;
    }
}

fn painting_pick_creature(level: i32, ranged: bool) -> Option<u16> {
    let candidates = painting_creature_candidates(level, ranged);
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[(random_number(candidates.len() as i32) - 1) as usize])
}

// (d) A monster painting is exactly as tough as the creature inside it.
fn painting_monster_hit_points(creature_id: u16) -> i16 {
    let creature = &CREATURES_LIST[creature_id as usize];
    if (creature.defenses & defense::CD_MAX_HP) != 0 {
        max_dice_roll(creature.hit_die) as i16
    } else {
        dice_roll(creature.hit_die) as i16
    }
}

pub fn painting_armor_class(index: usize) -> i32 {
    let painting = &paintings()[index];
    match painting.kind {
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster => CREATURES_LIST[painting.creature_id as usize].ac as i32,
        _ => 4,
    }
}

// Roll the kind, description, and stats for one freshly hung painting.
fn roll_new_painting(pos: Coord) -> Painting {
    let mut painting = Painting {
        pos,
        kind: PaintingKind::Harmless,
        desc_id: 0,
        creature_id: 0,
        hp: (4 + random_number(6)) as i16,
        items: 0,
        awake: false,
        found: false,
    };

    // Roughly half harmless; the rest split between monsters, loot,
    // trapped loot, and the rare sleeper and teleporter.
    let roll = random_number(100);
    if roll <= 55 {
        painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8;
        // (g) a few harmless scenes secretly hold a single trinket.
        if random_number(7) == 1 {
            painting.items = 1;
        }
    } else if roll <= 68 {
        match painting_pick_creature(level(), false) {
            Some(creature_id) => {
                painting.kind = PaintingKind::MeleeMonster;
                painting.creature_id = creature_id;
                painting.desc_id = (random_number(MELEE_PAINTING_TEMPLATES.len() as i32) - 1) as u8;
                painting.hp = painting_monster_hit_points(creature_id);
            }
            None => painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8,
        }
    } else if roll <= 76 {
        match painting_pick_creature(level(), true) {
            Some(creature_id) => {
                painting.kind = PaintingKind::RangedMonster;
                painting.creature_id = creature_id;
                painting.desc_id = (random_number(RANGED_PAINTING_TEMPLATES.len() as i32) - 1) as u8;
                painting.hp = painting_monster_hit_points(creature_id);
            }
            None => painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8,
        }
    } else if roll <= 88 {
        painting.kind = PaintingKind::Loot;
        painting.desc_id = (random_number(LOOT_PAINTINGS.len() as i32) - 1) as u8;
        painting.items = match random_number(6) {
            1..=3 => 1,
            4..=5 => 2,
            _ => 3,
        };
    } else if roll <= 93 {
        painting.kind = PaintingKind::FakeLoot;
        painting.desc_id = (random_number(LOOT_PAINTINGS.len() as i32) - 1) as u8;
        painting.items = (random_number(2) - 1) as u8; // 0 fangs, 1 spikes
    } else if roll <= 97 {
        painting.kind = PaintingKind::SleepGas;
        painting.desc_id = (random_number(SLEEP_PAINTINGS.len() as i32) - 1) as u8;
    } else {
        painting.kind = PaintingKind::Teleport;
        painting.desc_id = (random_number(TELEPORT_PAINTINGS.len() as i32) - 1) as u8;
    }

    painting
}

// A painting must hang where it can be seen: on a non-boundary wall tile
// with at least one orthogonally adjacent floor tile.
fn is_hangable_wall(coord: Coord) -> bool {
    let feature_id = dg().tile(coord).feature_id;
    if feature_id < MIN_CAVE_WALL || feature_id == TILE_BOUNDARY_WALL {
        return false;
    }

    for (dy, dx) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let neighbor = Coord::new(coord.y + dy, coord.x + dx);
        if coord_in_bounds(neighbor) && dg().tile(neighbor).feature_id <= MAX_CAVE_FLOOR {
            return true;
        }
    }

    false
}

// Hang paintings on the freshly generated level. Called at the end of
// dungeon generation, after the character's starting position is set (the
// level-map painting on dungeon level 1 must be the one nearest to it).
pub fn place_paintings() {
    paintings().clear();

    let target = (45 + random_number(11)) as usize; // 46..=56

    // The dungeon always has plenty of hangable wall, but cap the attempts
    // so a degenerate cave cannot loop forever.
    let mut attempts = 0;
    while paintings().len() < target && attempts < 50_000 {
        attempts += 1;

        let coord = Coord::new(random_number(dg().height as i32 - 2), random_number(dg().width as i32 - 2));

        if !is_hangable_wall(coord) || painting_index_at(coord).is_some() {
            continue;
        }

        let painting = roll_new_painting(coord);
        paintings().push(painting);
    }

    if paintings().is_empty() {
        return;
    }

    // Exactly one painting per level is a map of the level. On the first
    // dungeon level it is the painting nearest the character's start.
    let map_id = if dg().current_level == 1 {
        let mut nearest = 0;
        let mut nearest_distance = i32::MAX;
        for (i, painting) in paintings().iter().enumerate() {
            let distance = coord_distance_between(py().pos, painting.pos);
            if distance < nearest_distance {
                nearest_distance = distance;
                nearest = i;
            }
        }
        nearest
    } else {
        (random_number(paintings().len() as i32) - 1) as usize
    };

    let map_painting = &mut paintings()[map_id];
    map_painting.kind = PaintingKind::LevelMap;
    map_painting.desc_id = 0;
    map_painting.creature_id = 0;
    map_painting.items = 0;
    map_painting.awake = false;
    map_painting.found = false;
}

// Reveal the entire level, in the manner of spell_map_current_area() but
// over the whole map rather than the area around the panel.
fn reveal_level_map() {
    for y in 0..dg().height as i32 {
        for x in 0..dg().width as i32 {
            let coord = Coord::new(y, x);
            if coord_in_bounds(coord) && dg().tile(coord).feature_id <= MAX_CAVE_FLOOR {
                crate::spells::dungeon_light_area_around_floor_tile(coord);
            }
        }
    }

    draw_dungeon_panel();
}

// Award experience for defeating a painting's monster, mirroring the
// exp-fraction arithmetic of player_gain_kill_experience().
fn painting_gain_experience(exp: i32) {
    let mut quotient = exp / py().misc.level as i32;
    let mut remainder = exp % py().misc.level as i32;

    remainder *= 0x10000;
    remainder /= py().misc.level as i32;
    remainder += py().misc.exp_fraction as i32;

    if remainder >= 0x10000 {
        quotient += 1;
        py().misc.exp_fraction = (remainder - 0x10000) as u16;
    } else {
        py().misc.exp_fraction = remainder as u16;
    }

    py().misc.exp += quotient;
}

// Pull one random level-appropriate item out of a painting.
// Returns true if a grab was consumed.
fn painting_grab_item() -> bool {
    // Materialize the item through the treasure list so the standard
    // magic-ability enchantment path applies, then free the slot again.
    let free_treasure_id = popt() as usize;
    let object_id = item_get_random_object_id(level(), false);
    inventory_item_copy_to(sorted_objects()[object_id as usize] as usize, &mut game().treasure.list[free_treasure_id]);
    crate::treasure_magic::magic_treasure_magical_ability(free_treasure_id as i32, level());
    let mut item = game().treasure.list[free_treasure_id];
    pusht(free_treasure_id as u8);

    if !inventory_can_carry_item_count(&item) {
        // No room in the pack: set it down at the character's feet if the
        // floor there is clear, otherwise the painting keeps its prize.
        if dg().tile(py().pos).treasure_id == 0 {
            let floor_id = popt() as usize;
            game().treasure.list[floor_id] = item;
            dg().tile_mut(py().pos).treasure_id = floor_id as u8;
            print_message(Some("Your pack is full; it tumbles to the floor at your feet."));
            return true;
        }

        print_message(Some("Your pack is full, and it slips back into the canvas."));
        return false;
    }

    let slot_id = inventory_carry_item(&mut item) as usize;
    let description = item_description(&py().inventory[slot_id], true);
    let msg = format!("You have {} ({})", description, (slot_id as u8 + b'a') as char);
    print_message(Some(&msg));

    true
}

// (e) With its monster gone -- slain in the canvas or escaped from it --
// the painting settles into a new scene that may still hold treasure
// and/or a trap.
fn painting_after_monster_leaves(index: usize) {
    let roll = random_number(100);
    let (kind, desc_id, items) = if roll <= 40 {
        let desc = (random_number(LOOT_PAINTINGS.len() as i32) - 1) as u8;
        let items = if random_number(3) == 1 { 2 } else { 1 };
        (PaintingKind::Loot, desc, items)
    } else if roll <= 60 {
        let desc = (random_number(LOOT_PAINTINGS.len() as i32) - 1) as u8;
        (PaintingKind::FakeLoot, desc, (random_number(2) - 1) as u8)
    } else {
        let desc = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8;
        let items = if random_number(7) == 1 { 1 } else { 0 };
        (PaintingKind::Harmless, desc, items)
    };
    let hp = (4 + random_number(6)) as i16;

    let painting = &mut paintings()[index];
    painting.kind = kind;
    painting.desc_id = desc_id;
    painting.creature_id = 0;
    painting.hp = hp;
    painting.items = items;
    painting.awake = false;
    painting.found = false;

    if cave_tile_visible(paintings()[index].pos) {
        print_message(Some("The paint swirls and settles into a new scene."));
    }
}

// (b) A melee creature roused from its canvas tears free and becomes an
// ordinary monster beside the painting. Returns false if there was no room
// (or no free monster slot); the creature keeps straining until there is.
fn melee_monster_breaks_out(index: usize, announce_failure: bool) -> bool {
    let painting = paintings()[index];

    let mut coord = painting.pos;
    if !place_monster_adjacent_to(painting.creature_id as i32, &mut coord, false) {
        paintings()[index].awake = true;
        if announce_failure {
            print_message(Some("The canvas bulges, but nothing emerges."));
        }
        return false;
    }

    // Wounds dealt while it hung in the canvas stay with the creature.
    let monster_id = dg().tile(coord).creature_id as i32;
    monsters()[monster_id as usize].hp = painting.hp;
    monster_update_visibility(monster_id);

    let msg = format!("The {} tears itself free of the canvas!", painted_name(&painting));
    print_message(Some(&msg));
    player_disturb(1, 0);

    painting_after_monster_leaves(index);

    true
}

// (a) A monster painting acts only once roused -- by a close look, a bash,
// or a missile. Melee creatures leap out; casters wake in the canvas.
fn painting_monster_roused(index: usize, announce_failure: bool) {
    match paintings()[index].kind {
        PaintingKind::MeleeMonster => {
            melee_monster_breaks_out(index, announce_failure);
        }
        PaintingKind::RangedMonster => {
            if !paintings()[index].awake {
                paintings()[index].awake = true;
                let msg = format!("The {} turns its gaze upon you!", painted_name(&paintings()[index]));
                print_message(Some(&msg));
                player_disturb(1, 0);
            }
        }
        _ => {}
    }
}

// Apply damage from any source (bash, bolt, ball, thrown missile) to the
// painting at `index`. Monster outcomes are narrated here (and rouse the
// creature); Destroyed/Damaged messages are left to the caller, whose
// flavor differs by weapon.
pub fn painting_take_hit(index: usize, damage: i32) -> PaintingHitResult {
    let kind = paintings()[index].kind;
    let is_monster = kind == PaintingKind::MeleeMonster || kind == PaintingKind::RangedMonster;

    paintings()[index].hp -= damage as i16;

    if paintings()[index].hp < 0 {
        if is_monster {
            let painting = paintings()[index];
            let creature = &CREATURES_LIST[painting.creature_id as usize];

            let msg = format!("The {} shrieks and dissolves into dead pigment!", painted_name(&painting));
            print_message(Some(&msg));

            painting_gain_experience(creature.kill_exp_value as i32 * creature.level as i32);
            display_character_experience();

            painting_after_monster_leaves(index);
            PaintingHitResult::MonsterKilled
        } else {
            let pos = paintings()[index].pos;
            paintings().swap_remove(index);
            dungeon_lite_spot(pos);
            PaintingHitResult::Destroyed
        }
    } else if is_monster {
        painting_monster_roused(index, false);
        PaintingHitResult::MonsterAlive
    } else {
        PaintingHitResult::Damaged
    }
}

// (c) A bolt or ball spell striking a painted wall tile hits the painting.
// Called from spell_fire_bolt()/spell_fire_ball() with the (distance
// attenuated, for balls) spell damage.
pub fn painting_struck_by_magic(coord: Coord, damage: i32, spell_name: &str) {
    let index = match painting_index_at(coord) {
        Some(index) => index,
        None => return,
    };

    let msg = format!("The {} strikes a painting.", spell_name);
    print_message(Some(&msg));

    match painting_take_hit(index, damage) {
        PaintingHitResult::Destroyed => print_message(Some("The painting is blasted from the wall!")),
        PaintingHitResult::Damaged => print_message(Some("The canvas shudders.")),
        PaintingHitResult::MonsterKilled | PaintingHitResult::MonsterAlive => {}
    }
}

// Describe (and possibly trigger) the painting at `coord` for the look
// command. `prefix` is look's sentence opener ("You see", ...). Returns the
// message shown (empty if nothing was described; the caller counts non-empty
// messages) and the last key read, so ESCAPE still aborts the look.
pub fn look_at_painting(coord: Coord, prefix: &str) -> (String, char) {
    let index = match painting_index_at(coord) {
        Some(index) => index,
        None => return (String::new(), ' '),
    };

    // The wall may have been destroyed since the registry was built.
    if dg().tile(coord).feature_id < MIN_CAVE_WALL {
        paintings().swap_remove(index);
        return (String::new(), ' ');
    }

    // Full descriptions (and effects) require standing beside the painting.
    if coord_distance_between(py().pos, coord) > 1 {
        let msg = format!("{} a painting hanging on the wall ---pause---", prefix);
        put_string_clear_to_eol(&msg, Coord::new(0, 0));
        panel_move_cursor(coord);
        return (msg, get_key_input());
    }

    let painting = paintings()[index];
    let msg = format!("{} a painting ---pause---", prefix);
    put_string_clear_to_eol(&msg, Coord::new(0, 0));
    panel_move_cursor(coord);
    let key = get_key_input();
    if key == ESCAPE {
        return (msg, key);
    }

    let depicts = format!("It depicts {}.", painting_description(&painting));
    print_message(Some(&depicts));

    match painting.kind {
        PaintingKind::Harmless | PaintingKind::Loot | PaintingKind::FakeLoot => {
            reach_into_painting(index);
        }
        PaintingKind::LevelMap => {
            reveal_level_map();
            print_message(Some("You suddenly know every hall and chamber of this level!"));
        }
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster => {
            painting_monster_roused(index, true);
        }
        PaintingKind::Teleport => {
            if painting.found && !get_input_confirmation("It pulls at your gaze. Stare into the painting?") {
                reach_into_painting(index);
                return (msg, key);
            }
            paintings()[index].found = true;
            print_message(Some("The dungeon spins around you!"));
            game().teleport_player = true;
        }
        PaintingKind::SleepGas => {
            if painting.found && !get_input_confirmation("Its warmth invites sleep. Keep gazing?") {
                reach_into_painting(index);
                return (msg, key);
            }
            painting_sleep_trap(index);
        }
    }

    (msg, key)
}

// (f) A trap sprung by merely looking: the scene lulls you to sleep.
fn painting_sleep_trap(index: usize) {
    paintings()[index].found = true;

    print_message(Some("A drowsy sweetness seems to pour from the canvas."));

    if py().flags.free_action {
        print_message(Some("You feel momentarily heavy-lidded, but it passes."));
        return;
    }

    print_message(Some("Your eyes close; you fall into an enchanted sleep!"));
    py().flags.paralysis += (10 + random_number(20)) as i16;
    player_disturb(1, 0);
}

// (f) A trap sprung only by reaching in: fangs or poisoned spikes,
// selected by the painting's `items` field.
fn fake_loot_trap(index: usize) {
    paintings()[index].found = true;

    if paintings()[index].items == 0 {
        print_message(Some("Fangs sink into your hand!"));
        let damage = dice_roll(Dice::new(2, 6)) + level() / 2;
        player_takes_hit(damage, "a fanged painting");
    } else {
        print_message(Some("Poisoned spikes spring through the canvas!"));
        let damage = dice_roll(Dice::new(1, 8)) + level() / 3;
        player_takes_hit(damage, "a spiked painting");
        py().flags.poisoned += (10 + random_number(20)) as i16;
    }
}

// (g) The reach-in interaction, offered by any painting that is not the
// level map and holds no monster. At least half of all reaches come away
// with nothing, so an empty canvas cannot be told from a spent one.
fn reach_into_painting(index: usize) {
    if paintings()[index].kind == PaintingKind::Loot && paintings()[index].items > 0 {
        print_message(Some("Your fingertips tingle: something waits behind the paint."));
    }

    if paintings()[index].kind == PaintingKind::FakeLoot && paintings()[index].found {
        print_message(Some("You sense a malevolent presence behind the paint!"));
    }

    while get_input_confirmation("Reach into the painting?") {
        if paintings()[index].kind == PaintingKind::FakeLoot {
            fake_loot_trap(index);
            return;
        }

        if paintings()[index].items == 0 || random_number(2) == 1 {
            print_message(Some("You feel nothing but the rough back of the canvas."));
            continue;
        }

        if painting_grab_item() {
            paintings()[index].items -= 1;

            if paintings()[index].items == 0 {
                if paintings()[index].kind == PaintingKind::Loot {
                    print_message(Some("The colors fade to a dull grey."));
                }
                return;
            }

            print_message(Some("The painting still shimmers invitingly."));
        }
    }
}

// Bash the painting at `coord`. Any painting can be battered from the wall;
// the wall tile itself is unharmed. Called from player_bash() when the
// target tile is a wall with a painting.
pub fn player_bash_painting(coord: Coord) {
    let index = match painting_index_at(coord) {
        Some(index) => index,
        None => return,
    };

    let kind = paintings()[index].kind;
    let is_monster = kind == PaintingKind::MeleeMonster || kind == PaintingKind::RangedMonster;

    // Same bash mechanics as bashing a creature (player_bash_attack).
    let mut base_to_hit = py().stats.used[A_STR] as i32;
    base_to_hit += py().inventory[crate::inventory::PlayerEquipment::Arm as usize].weight as i32 / 2;
    base_to_hit += py().misc.weight as i32 / 10;

    let painting_ac = painting_armor_class(index);

    if player_test_being_hit(base_to_hit, py().misc.level as i32, py().stats.used[A_DEX] as i32, painting_ac, CLASS_BTH) {
        let arm_item = py().inventory[crate::inventory::PlayerEquipment::Arm as usize];
        let mut damage = dice_roll(arm_item.damage);
        damage = player_weapon_critical_blow(arm_item.weight as i32 / 4 + py().stats.used[A_STR] as i32, 0, damage, CLASS_BTH);
        damage += py().misc.weight as i32 / 60;
        damage += 3;

        if damage < 0 {
            damage = 0;
        }

        if is_monster {
            let msg = format!("You strike the {}!", painted_name(&paintings()[index]));
            print_message(Some(&msg));
        }

        match painting_take_hit(index, damage) {
            PaintingHitResult::Destroyed => print_message(Some("You smash the painting to tatters!")),
            PaintingHitResult::Damaged => print_message(Some("The painting shudders on its hook.")),
            PaintingHitResult::MonsterKilled | PaintingHitResult::MonsterAlive => {}
        }
    } else {
        print_message(Some("You bash at the painting and miss."));
        // Even a glancing blow rouses whatever lives in the canvas.
        if is_monster {
            painting_monster_roused(index, true);
        }
    }

    if random_number(150) > py().stats.used[A_DEX] as i32 {
        print_message(Some("You are off balance."));
        py().flags.paralysis = (1 + random_number(2)) as i16;
    }
}

// Remove any painting at `coord`; used when a wall tile is dug out or
// dissolved. Returns true if a painting was destroyed.
pub fn remove_painting_at(coord: Coord) -> bool {
    match painting_index_at(coord) {
        Some(index) => {
            paintings().swap_remove(index);
            true
        }
        None => false,
    }
}

// Reveal trapped paintings on the current panel; called from the
// detect-traps spell/scroll. Returns true if anything was found.
pub fn detect_painting_traps() -> bool {
    let mut detected = false;

    for painting in paintings().iter_mut() {
        if !coord_inside_panel(painting.pos) || painting.found {
            continue;
        }

        if painting.kind == PaintingKind::FakeLoot || painting.kind == PaintingKind::Teleport || painting.kind == PaintingKind::SleepGas {
            painting.found = true;
            detected = true;
        }
    }

    if detected {
        print_message(Some("You sense hostile magic within a painting here!"));
    }

    detected
}

// One game turn for every painting; called from the main loop alongside
// update_monsters(). Roused casters bolt anyone they can see; a melee
// creature still stuck in its canvas (no room to break out) keeps trying.
// Dormant monsters do nothing until the character rouses them.
pub fn update_paintings() {
    if dg().current_level == 0 {
        return;
    }

    let mut i = 0;
    while i < paintings().len() {
        if game().character_is_dead || dg().generate_new_level {
            return;
        }

        let pos = paintings()[i].pos;

        // The wall beneath may have been tunneled out or turned to mud.
        if dg().tile(pos).feature_id < MIN_CAVE_WALL {
            if cave_tile_visible(pos) {
                print_message(Some("A painting crumbles away with the wall."));
            }
            paintings().swap_remove(i);
            continue;
        }

        let kind = paintings()[i].kind;
        let awake = paintings()[i].awake;

        match kind {
            PaintingKind::MeleeMonster => {
                // Awake here means an earlier break-out failed for lack of
                // room (or the save predates break-outs); keep straining.
                if awake {
                    melee_monster_breaks_out(i, false);
                }
            }
            PaintingKind::RangedMonster => {
                let distance = coord_distance_between(py().pos, pos);
                let in_sight = distance <= MON_MAX_SIGHT as i32 && crate::dungeon_los::los(pos, py().pos);

                if awake && in_sight && random_number(3) == 1 {
                    let painting = paintings()[i];
                    let creature = &CREATURES_LIST[painting.creature_id as usize];
                    let name = painted_name(&painting);

                    let msg = format!("A bolt of force leaps from the {}'s canvas!", name);
                    print_message(Some(&msg));
                    player_disturb(1, 0);

                    let damage = dice_roll(Dice::new(2 + creature.level / 10, 6));
                    player_takes_hit(damage, &format!("a {}", name));
                }
            }
            _ => {}
        }

        i += 1;
    }
}
