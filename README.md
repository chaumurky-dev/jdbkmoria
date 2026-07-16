# jdbkmoria

A fork and expansion of [rmoria](https://github.com/chaumurky-dev/rmoria),
jdbkmoria adds new gameplay features not found in any other version of Moria
(see "Extensions over Umoria" below).

rmoria is a Rust translation of the
[Umoria](https://github.com/dungeons-of-moria/umoria) 5.7.15 C++ sources,
aiming to be gameplay-identical to the original. Umoria itself descends from
_The Dungeons of Moria_, the classic single-player dungeon simulation written
by Robert Alan Koeneke (first public release 1983) and ported to C by
James E. Wilson in 1988.

## License

jdbkmoria is a derivative work of rmoria (itself a derivative work of Umoria)
and is released under the same license: the **GNU General Public License,
version 3.0 or later** (GPL-3.0-or-later). See [LICENSE](LICENSE) for the full
license text.

Original copyrights:

- Copyright (c) 1981-86 Robert A. Koeneke
- Copyright (c) 1987-94 James E. Wilson
- Umoria 5.7.x restoration by Michael R. Cook and contributors

The complete corresponding source code of this program is contained in this
repository. The rmoria port this project forks from is available at
<https://github.com/chaumurky-dev/rmoria>, and the original Umoria source from
which rmoria was ported is available at
<https://github.com/dungeons-of-moria/umoria>.

## Building

```sh
cargo build --release
```

## Running

```sh
jdbkmoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
    -n           Force start of new game
    -r           Enable classic roguelike keys on startup
    -d           Display high scores and exit
    -s NUMBER    Game Seed, as a decimal number (max: 2147483647)
    -v           Print version info and exit
    -h           Display this message
```

## Extensions over Umoria

### Paintings

Each dungeon level hangs around fifty paintings (`0`) on its walls — a
deliberate gameplay addition not present in Umoria. Stand next to one and
**l**ook at it for a description drawn from the classical oil-painting
tradition. Roughly half are just art; the rest hide something:

- **Monsters** — dormant creatures doze inside the canvas until a close look,
  a reach, or a blow rouses them. Walk into a roused canvas to fight the
  creature with ordinary melee combat (normal to-hit, criticals, experience
  on a kill). If slain, its treasure remains IN the painting, waiting for you
  to reach in and pull it out with **g**.
- **Swarms** — canvases swarming with 2 to 20 small creatures (rats, bats,
  ants, spiders...). Once roused, they pour out one at a time. Fight them in
  the canvas, killing members one by one, or bash the painting to destroy all
  at once (forfeiting experience and treasure).
- **Magical larders** — reach in with **g** and pull out an item (sometimes
  two or three). A few are fakes that bite instead; trap detection reveals
  those.
- **Teleporters** — rarely, looking at a painting flings you across the level
  like a teleport trap.
- **The level map** — exactly one painting per level depicts the level
  itself; looking at it reveals the whole map. On the first dungeon level it
  is the painting nearest to where you arrived.

**Bashing a painting** (press **B**) is an all-or-nothing smash: if you
succeed, you rip the entire painting from the wall, destroying everything
inside—creature, swarm, loot, and traps alike—with no experience and no
treasure. If you fail, the painting holds firm but whatever lives in it wakes
up. Tunneling out or dissolving the wall also destroys a painting. Save files
remain compatible: old saves load fine, and the painting state rides along in
new ones.

### Input handling

Gameplay is otherwise unchanged, but the terminal input handling accepts a few extra keys:

- **Arrow / navigation keys** (arrows, Home, End, PgUp, PgDn — what a numpad
  sends in most terminals) move the player, using the usual 8-way keypad
  layout. They also answer any "Which direction?" prompt.
- **Shift + arrow/keypad key** runs in that direction, exactly like `.`
  followed by the direction (or the shifted letter in roguelike-keys mode).
  This requires a terminal that sends xterm-style modified sequences for
  shifted keys (most modern terminal emulators do).

## Testing

Run the unit tests with:

```sh
cargo test
```

Run the end-to-end PTY smoke test (requires a debug build):

```sh
python3 tools/smoke_test.py
```

The smoke test verifies version reporting, character creation, movement, and graceful exit. It runs the game in a temporary directory to avoid dirtying the repository with save files.
