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
//   It stays dormant until the character looks at it up close, walks into
//   it, or strikes the painting; once awake it either fights back from the
//   canvas or tears free and fights as an ordinary monster.
// - RangedMonster: a spell-casting creature that stays in its canvas and
//   hurls bolts of force once roused.
// - Swarm: a canvas crowded with two to twenty small vermin (rats, bats,
//   ants, and their kin) that pour out of the frame one at a time once
//   roused, and that can also be fought member by member without ever
//   leaving the wall.
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
// experience) from the creature inside; bolts, balls, thrown missiles, and
// (once the creature is awake) melee blows can all strike them.
//
// Bashing and fighting a monster painting are deliberately different
// trades. A bash (player_bash_painting) is a stuck-door-style smash: one
// all-or-nothing roll either tears the whole painting off the wall,
// destroying everything inside -- monster, swarm, loot, trap -- with no
// experience and no consolation prize, or it holds firm and merely rouses
// whatever lives there. Fighting the creature where it hangs
// (player_attack_painting), by walking into an awake canvas or striking it
// with a weapon, uses the same math as an ordinary monster fight and pays
// the same experience -- and whatever the creature carried is folded into
// the canvas's own loot, waiting to be reached for with `g` just as it
// would be from a Loot painting. A canvas whose monster has been defeated
// (or has leapt out) settles into a new scene that may still hold that
// loot, fresh treasure of its own, or a trap.

use crate::config::monsters::{defense, move_flags, spells as monster_spells, MON_MAX_SIGHT};
use crate::data_creatures::CREATURES_LIST;
use crate::data_paintings::{
    HARMLESS_PAINTINGS, LOOT_PAINTINGS, MAP_PAINTING, MELEE_PAINTING_TEMPLATES,
    RANGED_PAINTING_TEMPLATES, SLEEP_PAINTINGS, SWARM_PAINTING_TEMPLATES, TELEPORT_PAINTINGS,
};
use crate::dice::{dice_roll, max_dice_roll, Dice};
use crate::dungeon::{
    cave_tile_visible, coord_distance_between, coord_in_bounds, dg, dungeon_lite_spot,
};
use crate::dungeon_tile::{MAX_CAVE_FLOOR, MIN_CAVE_WALL, TILE_BOUNDARY_WALL};
use crate::game::{game, get_direction_with_memory, random_number, sorted_objects};
use crate::game_objects::{item_get_random_object_id, popt, pusht};
use crate::globals::RacyCell;
use crate::identification::item_description;
use crate::inventory::{
    inventory_can_carry_item_count, inventory_carry_item, inventory_item_copy_to, PlayerEquipment,
};
use crate::monster::{monster_death_item_drop_count, monster_update_visibility, monsters, Creature};
use crate::monster_manager::place_monster_adjacent_to;
use crate::player::{
    py, player_calculate_base_to_hit, player_calculate_to_hit_blows, player_disturb, player_move_position,
    player_takes_hit, player_test_being_hit, player_weapon_critical_blow, A_DEX, A_STR, CLASS_BTH,
};
use crate::player_magic::item_magic_ability_damage;
use crate::treasure::TV_NOTHING;
use crate::types::Coord;
use crate::ui::{coord_inside_panel, display_character_experience, draw_dungeon_panel, ESCAPE};
use crate::ui_io::{
    get_input_confirmation, get_key_input, panel_move_cursor, print_message, print_message_no_command_interrupt,
    put_string_clear_to_eol,
};

// Hard cap on the registry (the target count is 46-56 per level). Must stay
// below 0x40: the save format now packs two version flag bits into the top
// of the count byte (see game_save.rs's painting write/read).
pub const MAX_PAINTINGS: usize = 63;

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
    Swarm,
}

#[derive(Debug, Clone, Copy)]
pub struct Painting {
    pub pos: Coord,
    pub kind: PaintingKind,
    pub desc_id: u8,      // index into the matching data_paintings table
    pub creature_id: u16, // monster paintings: index into CREATURES_LIST
    pub hp: i16,          // durability; for monster paintings, the front creature's hit points
    pub items: u8,        // Loot/Harmless: remaining grabs; FakeLoot: 0 fangs, 1 spikes; Swarm: members left
    pub awake: bool,      // monster paintings: roused and fighting
    pub found: bool,      // FakeLoot/Teleport/SleepGas: nature revealed
    pub loot: u8,         // treasure grabs owed by monsters slain inside the canvas (see painting_take_hit)
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
        PaintingKind::Swarm => 8,
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
        8 => Some(PaintingKind::Swarm),
        _ => None,
    }
}

fn painting_kind_table_len(kind: PaintingKind) -> usize {
    match kind {
        PaintingKind::Harmless => HARMLESS_PAINTINGS.len(),
        PaintingKind::MeleeMonster => MELEE_PAINTING_TEMPLATES.len(),
        PaintingKind::RangedMonster => RANGED_PAINTING_TEMPLATES.len(),
        PaintingKind::Swarm => SWARM_PAINTING_TEMPLATES.len(),
        PaintingKind::Loot | PaintingKind::FakeLoot => LOOT_PAINTINGS.len(),
        PaintingKind::Teleport => TELEPORT_PAINTINGS.len(),
        PaintingKind::SleepGas => SLEEP_PAINTINGS.len(),
        PaintingKind::LevelMap => 1,
    }
}

// A painting kind whose `creature_id`/`hp` describe a real creature living
// (or hiding) in the canvas, as opposed to plain flavor or loot.
fn painting_kind_is_monster(kind: PaintingKind) -> bool {
    matches!(kind, PaintingKind::MeleeMonster | PaintingKind::RangedMonster | PaintingKind::Swarm)
}

// Rebuild one painting from its save-file fields, validating everything
// that would otherwise be used as an array index. Returns None on any
// out-of-range value so a corrupt save fails the restore cleanly.
#[allow(clippy::too_many_arguments)]
pub fn painting_from_save(
    pos: Coord,
    kind_byte: u8,
    desc_id: u8,
    creature_id: u16,
    hp: i16,
    items: u8,
    flags: u8,
    loot: u8,
) -> Option<Painting> {
    let kind = painting_kind_from_u8(kind_byte)?;

    if desc_id as usize >= painting_kind_table_len(kind) {
        return None;
    }

    if painting_kind_is_monster(kind) && creature_id as usize >= CREATURES_LIST.len() {
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
        loot,
    })
}

// Rebuild one painting from the original (pre-creature) save-file record.
// Monster paintings had no creature then, so pick one deterministically
// from the fields at hand; the RNG must not be touched during a restore.
// Swarm did not exist yet either, so a legacy record can never decode to
// one; `loot` is likewise a v2 invention and is always zero here.
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
        loot: 0,
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
        PaintingKind::Swarm => {
            SWARM_PAINTING_TEMPLATES[painting.desc_id as usize].replace("{}", &creature_indefinite_name(painting.creature_id))
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

// Creatures matching `suits` near the given dungeon level, widening the
// level band until something qualifies. Never the Balrog, and never the
// level-0 town folk. Shared by the melee/ranged monster picker and the
// swarm picker below; each supplies its own notion of "suits".
fn painting_creature_candidates_matching(level: i32, suits: impl Fn(&Creature) -> bool) -> Vec<u16> {
    let mut low = level - 3;
    let mut high = level + 2;

    loop {
        let mut candidates = Vec::new();

        for (id, creature) in CREATURES_LIST.iter().enumerate() {
            if (creature.movement & move_flags::CM_WIN) != 0 {
                continue;
            }

            let creature_level = creature.level as i32;
            if suits(creature) && creature_level >= low.max(1) && creature_level <= high {
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

// Creatures suitable for a MeleeMonster/RangedMonster painting.
fn painting_creature_candidates(level: i32, ranged: bool) -> Vec<u16> {
    painting_creature_candidates_matching(level, |creature| {
        if ranged {
            creature_fires_from_canvas(creature)
        } else {
            creature_can_leave_canvas(creature) && !creature_fires_from_canvas(creature)
        }
    })
}

fn painting_pick_creature(level: i32, ranged: bool) -> Option<u16> {
    let candidates = painting_creature_candidates(level, ranged);
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[(random_number(candidates.len() as i32) - 1) as usize])
}

// The small vermin a Swarm painting is willing to hold: ants, bats,
// centipedes, lice, rats, spiders, worms -- anything that scuttles or
// scurries rather than something dignified enough to warrant its own solo
// canvas.
const SWARM_SPRITES: &[u8] = b"abclrSw";

fn creature_is_swarm_animal(creature: &Creature) -> bool {
    SWARM_SPRITES.contains(&creature.sprite)
}

// Creatures suitable for a Swarm painting: the same "can leave the canvas
// and fights hand to hand" test as a MeleeMonster, further narrowed to the
// vermin sprite set above.
fn painting_swarm_candidates(level: i32) -> Vec<u16> {
    painting_creature_candidates_matching(level, |creature| {
        creature_is_swarm_animal(creature) && creature_can_leave_canvas(creature) && !creature_fires_from_canvas(creature)
    })
}

fn painting_pick_swarm_creature(level: i32) -> Option<u16> {
    let candidates = painting_swarm_candidates(level);
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
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster | PaintingKind::Swarm => {
            CREATURES_LIST[painting.creature_id as usize].ac as i32
        }
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
        loot: 0,
    };

    // Just under half harmless; the rest split between monsters (solo or
    // swarm), loot, trapped loot, and the rare sleeper and teleporter.
    let roll = random_number(100);
    if roll <= 50 {
        painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8;
        // (g) a few harmless scenes secretly hold a single trinket.
        if random_number(7) == 1 {
            painting.items = 1;
        }
    } else if roll <= 63 {
        match painting_pick_creature(level(), false) {
            Some(creature_id) => {
                painting.kind = PaintingKind::MeleeMonster;
                painting.creature_id = creature_id;
                painting.desc_id = (random_number(MELEE_PAINTING_TEMPLATES.len() as i32) - 1) as u8;
                painting.hp = painting_monster_hit_points(creature_id);
            }
            None => painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8,
        }
    } else if roll <= 71 {
        match painting_pick_creature(level(), true) {
            Some(creature_id) => {
                painting.kind = PaintingKind::RangedMonster;
                painting.creature_id = creature_id;
                painting.desc_id = (random_number(RANGED_PAINTING_TEMPLATES.len() as i32) - 1) as u8;
                painting.hp = painting_monster_hit_points(creature_id);
            }
            None => painting.desc_id = (random_number(HARMLESS_PAINTINGS.len() as i32) - 1) as u8,
        }
    } else if roll <= 76 {
        match painting_pick_swarm_creature(level()) {
            Some(creature_id) => {
                painting.kind = PaintingKind::Swarm;
                painting.creature_id = creature_id;
                painting.desc_id = (random_number(SWARM_PAINTING_TEMPLATES.len() as i32) - 1) as u8;
                painting.items = (1 + random_number(19)) as u8; // 2..=20 members
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

    let pending_loot = paintings()[index].loot;

    let painting = &mut paintings()[index];
    painting.kind = kind;
    painting.desc_id = desc_id;
    painting.creature_id = 0;
    painting.hp = hp;
    painting.items = items;
    painting.awake = false;
    painting.found = false;
    painting.loot = 0;

    // (i) Whatever the slain creature(s) carried outlives the scene
    // re-roll: it forces a Loot painting regardless of what the dice
    // above produced, so the drops are always reachable even if the re-roll
    // had settled on FakeLoot or Harmless.
    if pending_loot > 0 {
        let painting = &mut paintings()[index];
        painting.kind = PaintingKind::Loot;
        painting.desc_id = (random_number(LOOT_PAINTINGS.len() as i32) - 1) as u8;
        painting.items = painting.items.saturating_add(pending_loot).min(8);
    }

    if cave_tile_visible(paintings()[index].pos) {
        print_message(Some("The paint swirls and settles into a new scene."));
    }
}

// (h) Roll a slain in-canvas creature's drop and add it to the painting's
// pending loot (folded in by painting_after_monster_leaves). The reach
// command materializes generic level-appropriate items standing in for
// whatever the creature actually carried; any gold it would have dropped
// is folded into that same pool of item grabs, since a canvas has no purse
// to jingle.
fn painting_add_loot_drop(index: usize, creature: &Creature) {
    if (creature.movement & (move_flags::CM_CARRY_OBJ | move_flags::CM_CARRY_GOLD)) != 0 {
        let drop = monster_death_item_drop_count(creature.movement) as u8;
        paintings()[index].loot = paintings()[index].loot.saturating_add(drop);
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
// a missile, or the character walking into it. Melee creatures leap out;
// casters wake in the canvas; a swarm stirs and starts pouring out members.
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
        PaintingKind::Swarm => {
            if !paintings()[index].awake {
                paintings()[index].awake = true;
                print_message(Some("The canvas seethes -- a hundred painted eyes turn toward you!"));
                player_disturb(1, 0);
            }
        }
        _ => {}
    }
}

// (j) One member of a Swarm painting dies to a blow. Unlike a solo monster
// painting, the canvas itself survives as long as members remain: a fresh
// front creature (and its hit points) steps up to take the last one's
// place, and the painting only truly dies -- settling into a new scene --
// once the whole swarm is spent.
fn painting_swarm_member_dies(index: usize) -> PaintingHitResult {
    let painting = paintings()[index];
    let creature = &CREATURES_LIST[painting.creature_id as usize];

    let msg = format!("A {} dissolves into dead pigment!", painted_name(&painting));
    print_message(Some(&msg));

    painting_gain_experience(creature.kill_exp_value as i32 * creature.level as i32);
    display_character_experience();

    painting_add_loot_drop(index, creature);

    paintings()[index].items -= 1;

    if paintings()[index].items > 0 {
        paintings()[index].hp = painting_monster_hit_points(painting.creature_id);
        PaintingHitResult::MonsterAlive
    } else {
        painting_after_monster_leaves(index);
        PaintingHitResult::MonsterKilled
    }
}

// Apply damage from any source (bash, bolt, ball, thrown missile, or melee
// blow) to the painting at `index`. Monster outcomes are narrated here (and
// rouse the creature); Destroyed/Damaged messages are left to the caller,
// whose flavor differs by weapon.
pub fn painting_take_hit(index: usize, damage: i32) -> PaintingHitResult {
    let kind = paintings()[index].kind;
    let is_monster = painting_kind_is_monster(kind);

    paintings()[index].hp -= damage as i16;

    if paintings()[index].hp < 0 {
        if kind == PaintingKind::Swarm {
            return painting_swarm_member_dies(index);
        }

        if is_monster {
            let painting = paintings()[index];
            let creature = &CREATURES_LIST[painting.creature_id as usize];

            let msg = format!("The {} shrieks and dissolves into dead pigment!", painted_name(&painting));
            print_message(Some(&msg));

            painting_gain_experience(creature.kill_exp_value as i32 * creature.level as i32);
            display_character_experience();

            painting_add_loot_drop(index, creature);
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
            // jdbkmoria extension: reaching in is now the dedicated 'g' command;
            // looking only hints at what a reach might find.
            if painting.kind == PaintingKind::Loot && painting.items > 0 {
                print_message(Some("Your fingertips tingle: something waits behind the paint."));
            }
            if painting.kind == PaintingKind::FakeLoot && painting.found {
                print_message(Some("You sense a malevolent presence behind the paint!"));
            }
        }
        PaintingKind::LevelMap => {
            reveal_level_map();
            print_message(Some("You suddenly know every hall and chamber of this level!"));
        }
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster | PaintingKind::Swarm => {
            painting_monster_roused(index, true);
        }
        PaintingKind::Teleport => {
            if painting.found && !get_input_confirmation("It pulls at your gaze. Stare into the painting?") {
                return (msg, key);
            }
            painting_teleport_trap(index);
        }
        PaintingKind::SleepGas => {
            if painting.found && !get_input_confirmation("Its warmth invites sleep. Keep gazing?") {
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

// (f) A trap sprung by merely looking: the dungeon whirls the character away.
fn painting_teleport_trap(index: usize) {
    paintings()[index].found = true;

    print_message(Some("The dungeon spins around you!"));
    game().teleport_player = true;
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

// (g) A grope at a painting that is not (or is no longer) magical to the
// touch: just paint on a wall. One line, chosen at random, so the flavor
// doesn't repeat too predictably.
const MUNDANE_REACH_MESSAGES: [&str; 8] = [
    "You grope at the canvas. The canvas remains a canvas.",
    "It's paint on a wall. You do know that, right?",
    "You press your palm against the painting. It declines to become a portal.",
    "The painting's subject watches you paw at it with something like pity.",
    "Your fingers find only canvas. Somewhere, a museum guard shudders.",
    "You reach dramatically into... plain, ordinary paint.",
    "Nothing happens, as any child could have told you.",
    "The wall behind the canvas is unmoved by your ambition.",
];

// (g) Reach into (grope at) a painting -- the dedicated 'g' command. Unlike
// looking, this always attempts the interaction: at least half of all
// Loot/Harmless reaches come away with nothing, so an empty canvas cannot be
// told from a spent one.
pub fn painting_reach_command() {
    let mut dir = 0;
    if !get_direction_with_memory(None, &mut dir) {
        return;
    }

    let mut coord = py().pos;
    player_move_position(dir, &mut coord);

    let tile = *dg().tile(coord);
    let index = if tile.feature_id >= MIN_CAVE_WALL { painting_index_at(coord) } else { None };

    let index = match index {
        Some(index) => index,
        None => {
            print_message(Some("You see no painting there."));
            game().player_free_turn = true;
            return;
        }
    };

    match paintings()[index].kind {
        PaintingKind::Loot | PaintingKind::Harmless if paintings()[index].items > 0 => {
            if random_number(2) == 1 {
                print_message(Some("You feel nothing but the rough back of the canvas."));
                return;
            }

            if painting_grab_item() {
                paintings()[index].items -= 1;

                if paintings()[index].items == 0 && paintings()[index].kind == PaintingKind::Loot {
                    print_message(Some("The colors fade to a dull grey."));
                }
            }
        }
        PaintingKind::FakeLoot => {
            fake_loot_trap(index);
        }
        PaintingKind::MeleeMonster | PaintingKind::RangedMonster | PaintingKind::Swarm => {
            print_message(Some("Something inside the canvas snaps at your fingers!"));
            painting_monster_roused(index, true);
        }
        PaintingKind::Teleport if !paintings()[index].found => {
            painting_teleport_trap(index);
        }
        PaintingKind::SleepGas if !paintings()[index].found => {
            painting_sleep_trap(index);
        }
        _ => {
            let msg = MUNDANE_REACH_MESSAGES[(random_number(MUNDANE_REACH_MESSAGES.len() as i32) - 1) as usize];
            print_message(Some(msg));
            game().player_free_turn = true;
        }
    }
}

// Bash the painting at `coord`, stuck-door style (see player_bash_closed_door
// in player_bash.rs): one all-or-nothing roll, not a combat exchange. Success
// tears the whole painting from the wall and destroys everything inside --
// no experience, no loot, unlike besting the creature in a fight. Failure
// just rouses whatever lives there. The wall tile itself is never touched.
// Called from player_bash() when the target tile is a wall with a painting.
pub fn player_bash_painting(coord: Coord) {
    let index = match painting_index_at(coord) {
        Some(index) => index,
        None => return,
    };

    print_message_no_command_interrupt("You slam your shoulder into the painting!");

    let kind = paintings()[index].kind;
    let is_monster = painting_kind_is_monster(kind);

    let chance = py().stats.used[A_STR] as i32 + py().misc.weight as i32 / 2;
    let toughness = 5 + level() / 4;

    // Same method as bashing a stuck/locked door, with the painting's
    // toughness (scaled off dungeon depth) playing misc_use's role.
    if random_number(chance * (20 + toughness)) < 10 * (chance - toughness) {
        let painting = paintings()[index];
        paintings().swap_remove(index);
        dungeon_lite_spot(coord);

        print_message(Some("The painting rips from the wall and tears apart!"));
        if is_monster {
            let msg = format!("The {} is torn apart along with the canvas.", painted_name(&painting));
            print_message(Some(&msg));
        }

        return;
    }

    // Even a failed smash rattles whatever lives in the canvas awake.
    if is_monster {
        painting_monster_roused(index, true);
    }

    if random_number(150) > py().stats.used[A_DEX] as i32 {
        print_message(Some("You are off-balance."));
        py().flags.paralysis = (1 + random_number(2)) as i16;
        return;
    }

    if game().command_count == 0 {
        print_message(Some("The painting holds firm on its hook."));
    }
}

// (j) The player attacks the creature living in an awake monster or swarm
// painting -- the same to-hit/damage math as an ordinary monster fight
// (player_attack_monster in player.rs), but the target is a painting index
// rather than a Creature record, and each landed blow goes through
// painting_take_hit rather than monster_take_hit. Called instead of a wall
// bump when the character walks into a roused canvas (see player_move.rs),
// and could equally be wired up for a direct "fight the wall" command later.
pub fn player_attack_painting(coord: Coord) {
    if py().flags.afraid > 0 {
        print_message(Some("You are too afraid to attack it!"));
        return;
    }

    let index = match painting_index_at(coord) {
        Some(index) => index,
        None => return,
    };

    let item = py().inventory[PlayerEquipment::Wield as usize];

    let mut blows = 0;
    let mut total_to_hit = 0;
    player_calculate_to_hit_blows(item.category_id, item.weight, &mut blows, &mut total_to_hit);

    // A painting's creature is always in plain view: there is no "unlit
    // monster in the dark" case for something hanging on a wall you're
    // standing next to.
    let base_to_hit = player_calculate_base_to_hit(true, total_to_hit);

    let mut i = blows;
    while i > 0 {
        if painting_index_at(coord) != Some(index) {
            // The painting died (or was otherwise removed) mid-flurry.
            return;
        }

        if !painting_kind_is_monster(paintings()[index].kind) {
            // A blow roused a stuck melee creature and it broke out between
            // swings, re-rolling the canvas: nothing is left here to fight.
            return;
        }

        let name = painted_name(&paintings()[index]);
        let armor_class = painting_armor_class(index);

        if !player_test_being_hit(base_to_hit, py().misc.level as i32, total_to_hit, armor_class, CLASS_BTH) {
            let msg = format!("You miss the {}.", name);
            print_message(Some(&msg));
            i -= 1;
            continue;
        }

        let msg = format!("You hit the {}.", name);
        print_message(Some(&msg));

        let item = py().inventory[PlayerEquipment::Wield as usize];
        let creature_id = paintings()[index].creature_id as usize;
        let mut damage;
        if item.category_id != TV_NOTHING {
            damage = dice_roll(item.damage);
            damage = item_magic_ability_damage(&item, damage, creature_id);
            damage = player_weapon_critical_blow(item.weight as i32, total_to_hit, damage, CLASS_BTH);
        } else {
            // Bare hands!?
            damage = dice_roll(Dice::new(1, 1));
            damage = player_weapon_critical_blow(1, 0, damage, CLASS_BTH);
        }

        damage += py().misc.plusses_to_damage as i32;
        if damage < 0 {
            damage = 0;
        }

        if painting_take_hit(index, damage) == PaintingHitResult::MonsterKilled {
            return;
        }

        i -= 1;
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
            PaintingKind::Swarm => {
                // A 1-in-3 chance per turn to push one more member out;
                // failure (no room adjacent to the canvas) just means it
                // tries again next turn. The front member's `hp` is untouched
                // -- monster_place_new rolls the emerging creature its own
                // fresh hit points.
                if awake && paintings()[i].items > 0 && random_number(3) == 1 {
                    let creature_id = paintings()[i].creature_id as i32;
                    let mut coord = pos;
                    if place_monster_adjacent_to(creature_id, &mut coord, false) {
                        paintings()[i].items -= 1;

                        let msg = format!("A {} scrambles out of the canvas!", painted_name(&paintings()[i]));
                        print_message(Some(&msg));
                        player_disturb(1, 0);

                        if paintings()[i].items == 0 {
                            painting_after_monster_leaves(i);
                        }
                    }
                }
            }
            _ => {}
        }

        i += 1;
    }
}
