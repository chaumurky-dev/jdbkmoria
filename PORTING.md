# Porting status: umoria (C++) → jdbkmoria (Rust)

Source: `../umoria/src` at version 5.7.15. Goal: gameplay-identical port, GPL-3.0-or-later.

Note: the port described in this document is rmoria (<https://github.com/chaumurky-dev/rmoria>);
jdbkmoria is a downstream fork that extends rmoria with new gameplay features
(see "Extensions beyond upstream" below). The porting status and file-by-file
mapping that follow describe the rmoria layer that jdbkmoria is built on.

## Conventions

- One Rust module per C++ source file (roughly). C++ `camelCase` functions become `snake_case`.
- Global game state (`game`, `dg`, `py`, `monsters`, ...) lives in `RacyCell` statics
  (see `globals.rs`) with accessor functions: `game()`, `dg()`, `py()`, `monsters()`, etc.
  The game is single threaded; do not add threads.
- Static data tables were machine-converted from the C sources (scripts preserved in git
  history / scratchpad) and verified value-for-value against the originals.
- `Coord_t` → `types::Coord`, `Dice_t` → `dice::Dice`, `vtype_t`/`obj_desc_t` → `String`.
- Original copyright headers and SPDX `GPL-3.0-or-later` identifiers preserved per file.
- The crate is split into a library (`src/lib.rs`, `pub mod` for every module) and a thin
  binary (`src/main.rs`, just `main()`/`parse_game_seed()`), so integration tests and any
  future consumers can exercise game logic directly. `tests/save_roundtrip.rs` seeds the
  RNG, pokes known values into the global state (player misc/stats, a couple of inventory
  items, monster recall memory, `game().character_died_from`), then round-trips them
  through `game_save::save_game_state_to_file()` / `load_game_state_from_file()` - small
  `pub` non-interactive cores factored out of `save_char()`/`restore_from_file()` that
  contain no curses calls, so the test runs without a terminal. Everything lives in one
  `#[test] fn` since the game state is process-wide (see `globals.rs`).
- `cargo clippy` is kept at zero warnings; a handful of pervasive, intentionally
  C-mirroring lints (`needless_range_loop`, `manual_range_contains`, `collapsible_if`,
  `collapsible_match`) are allowed crate-wide in `lib.rs` rather than hand-fixed, to avoid
  any risk of reordering RNG/game-logic calls.

## Status

**PORT COMPLETE (2026-07-12).** All 40 C++ source files ported; builds clean;
RNG reference test passes; verified end-to-end in a PTY (character creation,
town generation, movement, quit/tombstone, save + restore roundtrip).
Save files use the umoria 5.2.2+ compatible xor-encrypted format.

| C++ file | Rust module | Status |
|---|---|---|
| rng.cpp | rng.rs | done (reference test passes) |
| dice.cpp | dice.rs | done |
| helpers.cpp | helpers.rs | done |
| config.cpp/h | config.rs | done |
| types.h | types.rs | done |
| version.h | version.rs | done |
| game.h/cpp | game.rs | done |
| dungeon_tile.h | dungeon_tile.rs | done |
| dungeon.h/cpp | dungeon.rs | done |
| player.h/cpp | player.rs | done |
| monster.h/cpp | monster.rs | done |
| inventory.h/cpp | inventory.rs | done |
| treasure.h | treasure.rs | done (treasure.cpp -> treasure_magic.rs) |
| character.h/cpp | character.rs | done |
| recall.h/cpp | recall_data.rs + recall.rs | done |
| store.h | store_data.rs | done + verified |
| scores.h | scores.rs | done (high score read/write via game_save.rs xor I/O, display screen, scoring) |
| spells.h | spells_data.rs | done + verified |
| identification.h/cpp | identification.rs | done |
| data_creatures.cpp | data_creatures.rs | done + verified |
| data_treasure.cpp | data_treasure.rs | done + verified |
| data_player.cpp | data_player.rs | done + verified |
| data_recall.cpp | data_recall.rs | done + verified |
| data_stores.cpp | data_stores.rs | done + verified |
| data_store_owners.cpp | data_store_owners.rs | done + verified |
| data_tables.cpp | data_tables.rs | done + verified |
| ui_io.cpp | ui_io.rs | done (pancurses) |
| ui.h + ui.cpp | ui.rs | done |
| main.cpp | main.rs | done |
| game_files.cpp | game_files.rs | done |
| game_save.cpp | game_save.rs | done (save/load game state, xor byte I/O, high score I/O; builds clean) |
| game_death.cpp | game_death.rs | done |
| game_run.cpp | game_run.rs | done |
| game_objects.cpp | game_objects.rs | done |
| ui_inventory.cpp | ui_inventory.rs | done |
| dungeon_los.cpp | dungeon_los.rs | done |
| treasure.cpp | treasure_magic.rs | done |
| dungeon_generate.cpp | dungeon_generate.rs | done |
| monster_manager.cpp | monster_manager.rs | done |
| player_*.cpp | player_*.rs | done |
| spells.cpp | spells.rs | done |
| mage_spells.cpp | mage_spells.rs | done |
| scrolls.cpp | scrolls.rs | done |
| staves.cpp | staves.rs | done |
| store.cpp | store.rs | done |
| store_inventory.cpp | store_inventory.rs | done |
| wizard.cpp | wizard.rs | done |

## Extensions beyond upstream

Deliberate gameplay additions, kept out of the ported modules where possible
and marked `jdbkmoria extension` at every hook site in shared code:

- **Paintings** (`paintings.rs` + `data_paintings.rs`, 2026-07): ~50 paintings
  per dungeon level on wall tiles. Registry is a per-level `Vec<Painting>`
  keyed by coordinate (the save format packs `feature_id` into 4 bits, so no
  new wall type). Monster paintings hold a real creature from
  `CREATURES_LIST` (level-appropriate, `Painting::creature_id`) that sets
  their hit points, armor class, and kill experience; they are dormant until
  looked at up close or struck. **Bashing a painting** (`player_bash_painting`)
  is a stuck-door-style all-or-nothing smash (strength + body weight vs. the
  painting's toughness): success rips the painting from the wall, destroying
  everything inside—monster, swarm, loot, and traps alike—with no experience
  and no treasure; failure rouses whatever lives there. **Fighting a creature
  inside its canvas** (`player_attack_painting`), by walking into an awake
  monster/swarm painting or striking it with a weapon, uses ordinary melee
  combat math and awards experience on a kill. Bolts, balls, and thrown
  missiles also strike the creature. A creature slain inside its canvas may
  leave its treasure behind IN the painting, which becomes a loot scene you
  reach into with `g`. **Swarms** are paintings crowded with 2–20 small vermin
  (rats, bats, ants, spiders) appropriate to the depth; once roused they pour
  out one at a time, turn by turn, until exhausted. You can fight them in the
  canvas (killing members one by one) or bash the painting to destroy all
  remaining members at once (forfeiting their experience and loot). Non-map,
  monster-free paintings can be reached into (at least half of reaches yield
  nothing). Traps: teleport and sleep-gas fire on a look; fangs and poisoned
  spikes only on a reach. Hooks: `dungeon_generate()` (placement),
  `look_see()` (describe/interact), `player_bash()` (smash),
  `player_move()` (walking into a roused canvas fights it instead of bumping
  the wall), `update_paintings()` in the main loop (roused monster/swarm
  paintings act), `spell_fire_bolt()` / `spell_fire_ball()` /
  `player_throw_item()` (missiles strike paintings),
  `spell_detect_traps_within_vicinity()` (reveal trapped ones),
  `player_tunnel_wall()` (wall gone → painting gone), `cave_get_tile_symbol()`
  (`'0'` glyph). Save format: a painting block is appended after the monster
  data; restore probes for it with the same raw EOF peek the dead/alive fork
  uses, so pre-painting save files still load. The block's count byte carries
  a version flag in its top two bits (legacy: both clear; v1: creature id
  added; v2: adds the loot byte); legacy (pre-creature) records are converted
  on load (`painting_from_legacy_save()`). Tests: `tests/paintings.rs`
  (placement on a real generated level, legacy conversion, Swarm placement
  invariants, `painting_from_save()` validation) and
  `tests/save_roundtrip.rs` (serialization).

- **Locale support** (`locale.rs` + `locale_data_en_gb.rs` /
  `locale_data_fr_ca.rs`, 2026-07): the game supports en_US (default),
  en_GB, and fr_CA. Selection at startup: `-l LOCALE` > `JDBKMORIA_LOCALE`
  > `LC_ALL`/`LC_MESSAGES`/`LANG` > en_US (`locale::initialize_locale()`).
  Translation is keyed by the original en_US string: user-facing literals
  are wrapped in `tr!(...)` (lookup) or `tr_fmt!(...)` (lookup + `{}`/`{0}`
  positional template formatting, so translations can reorder arguments);
  static data-table names (creatures, treasure, stores, spells, races,
  backgrounds, recall fragments, paintings) are wrapped at their DISPLAY
  sites — the machine-verified `data_*.rs` tables themselves are never
  edited. Catalogs are sorted `&[(&str, &str)]` arrays; missing keys fall
  back to en_US; `tests/locale.rs` enforces sortedness and placeholder
  parity. `LocaleDef` also carries digit-grouping rules (`format_number`;
  fr_CA groups with no-break spaces; an en_IN-style `[3, 2]` grouping is
  supported), digit glyphs for the store-entrance map numerals
  (`map_digit_glyph`, ready for e.g. Arabic-Indic digits), and per-locale
  plural-suffix rewrite rules used by `item_description()`'s `~` marker.
  Item-name templates keep upstream's `&`/`~`/`%s` codes; store haggle
  speech keeps `%A1`/`%A2`. Localized data files resolve via
  `config::files::localized()` (`data/fr_CA/help.txt` etc., falling back
  to English). UTF-8: the binary links `ncursesw` (pancurses `wide`
  feature) and calls `setlocale()` before `initscr()`; text
  truncation/wrapping paths were hardened to char boundaries
  (`tests/utf8_safety.rs`). Adding a locale: run
  `python3 tools/locale_catalog.py extract` for the key list, translate to
  JSONL, `gen` the catalog module, add a `LocaleDef` to `locale.rs`, and
  (optionally) drop localized text files under `data/<tag>/`. Known v1
  limitations: English-specific article machinery (a/an via `is_vowel`,
  no French elision), masculine-default gender, en-US-shaped plural rules
  in a few composed fragments, and fixed-width status fields that truncate
  long translations (`{:<N.N}`).
