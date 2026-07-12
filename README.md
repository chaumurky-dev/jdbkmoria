# rmoria

A Rust port of [Umoria](https://github.com/dungeons-of-moria/umoria) — _The
Dungeons of Moria_, the classic single-player dungeon simulation originally
written by Robert Alan Koeneke (first public release 1983), ported to C by
James E. Wilson in 1988 and released as _Umoria_.

This port aims to be gameplay-identical to Umoria 5.7.15, translated from the
C++ sources into Rust.

## License

rmoria is a derivative work of Umoria and is released under the same license:
the **GNU General Public License, version 3.0 or later** (GPL-3.0-or-later).
See [LICENSE](LICENSE) for the full license text.

Original copyrights:

- Copyright (c) 1981-86 Robert A. Koeneke
- Copyright (c) 1987-94 James E. Wilson
- Umoria 5.7.x restoration by Michael R. Cook and contributors

The complete corresponding source code of this program is contained in this
repository. The original Umoria source from which it was ported is available
at <https://github.com/dungeons-of-moria/umoria>.

## Building

```sh
cargo build --release
```

## Running

```sh
rmoria [OPTIONS] SAVEGAME

SAVEGAME is an optional save game filename (default: game.sav)

Options:
    -n           Force start of new game
    -r           Enable classic roguelike keys on startup
    -d           Display high scores and exit
    -s NUMBER    Game Seed, as a decimal number (max: 2147483647)
    -v           Print version info and exit
    -h           Display this message
```

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
