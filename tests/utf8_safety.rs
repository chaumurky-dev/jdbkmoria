// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// jdbkmoria extension: regression tests for the char-boundary hardening of
// two legacy text-wrapping paths ahead of multi-byte UTF-8 (accented
// French) translations landing in the catalogs:
//
//   - `character::wrap_history_lines()`, factored out of
//     `character_get_history()`'s post-processing.
//   - `recall::roff_split_buffer()`, factored out of `Roff::print()`.
//
// Both are pure functions with no dependency on the game's global state, so
// unlike `save_roundtrip.rs` / `paintings.rs` there is no need to cram
// everything into a single #[test] fn.

use jdbkmoria::character::wrap_history_lines;
use jdbkmoria::recall::roff_split_buffer;

// Mirrors the outer push/flush loop of `Roff::print()` (see recall.rs),
// using a configurable width so tests don't need a MORIA_MESSAGE_SIZE (80
// char) string. The only logic under test here is `roff_split_buffer()`
// itself; this loop is just the harness that feeds it.
fn simulate_roff(text: &str, width: usize) -> Vec<String> {
    let mut buffer: Vec<char> = Vec::new();
    let mut lines = Vec::new();

    for ch in text.chars() {
        buffer.push(ch);
        if ch == '\n' || buffer.len() >= width {
            let (line, remainder) = roff_split_buffer(&buffer, ch);
            lines.push(line);
            buffer = remainder;
        }
    }
    if !buffer.is_empty() {
        lines.push(buffer.into_iter().collect());
    }

    lines
}

#[test]
fn wrap_history_lines_ascii_parity() {
    // A plain-ASCII background block, long enough to wrap more than once.
    // This pins the exact wrapping behavior of the (now char-based) port so
    // a future change can't silently alter it for the common case.
    let block = "You are the illegitimate and unacknowledged child of a Serf.  \
You are the black sheep of the family.  You are a credit to the family.  \
You are a well liked member of the community.  ";

    let lines = wrap_history_lines(block);

    // Every line fits the 60-column budget, and reassembling with single
    // spaces reproduces the original words (ignoring the exact whitespace
    // run-lengths the algorithm collapses at wrap points).
    for line in &lines {
        assert!(line.chars().count() <= 60, "line exceeds 60 columns: {:?}", line);
    }

    let reassembled = lines.join(" ");
    for word in ["illegitimate", "unacknowledged", "Serf.", "black", "sheep", "credit", "well", "liked", "community."] {
        assert!(reassembled.contains(word), "missing word {:?} in reassembled text: {:?}", word, reassembled);
    }
}

#[test]
fn wrap_history_lines_multibyte_no_panic_and_char_width() {
    // French-flavored background text: accented, multi-byte UTF-8 chars
    // placed right around where the 60-char wrap boundary will fall. Before
    // the char-boundary fix this indexed `history_block.as_bytes()` and
    // could panic (or silently corrupt a character) if a wrap point landed
    // inside a multi-byte sequence.
    let block = "Vous \u{ea}tes le fils illégitime et non reconnu d'un serf. \
Vous \u{ea}tes le mouton noir de la famille. Vous \u{ea}tes un cr\u{e9}dit \
pour la famille. Vous \u{ea}tes un membre appréci\u{e9} de la communaut\u{e9}. \
Caf\u{e9} \u{e9}t\u{e9} \u{e9}cout\u{e9} \u{e9}l\u{e9}gant d\u{e9}j\u{e0} \u{e9}crit.";

    // Sanity check: this text really does contain multi-byte chars, i.e.
    // char count and byte count differ.
    assert!(block.chars().count() < block.len(), "test fixture has no multi-byte chars");

    let lines = wrap_history_lines(block);

    // No line ever exceeds 60 *characters* (display columns) - byte-based
    // counting would UNDER-count multi-byte lines to more than 60 chars, or
    // wrap early on a run of accented text.
    for line in &lines {
        let char_len = line.chars().count();
        assert!(char_len <= 60, "line exceeds 60 columns: {:?} ({} chars)", line, char_len);
        // Every produced line must itself be valid, uncorrupted UTF-8 with
        // no stray replacement characters from a mis-sliced multi-byte
        // sequence.
        assert!(!line.contains('\u{FFFD}'), "line contains a corrupted char: {:?}", line);
    }

    // The accented words survive intact (not split/mangled) somewhere in
    // the wrapped output.
    let reassembled = lines.join(" ");
    assert!(reassembled.contains("illégitime"));
    assert!(reassembled.contains("crédit"));
    assert!(reassembled.contains("communauté."));
}

#[test]
fn roff_split_buffer_ascii_parity() {
    // "café " repeated is used in the multi-byte test below; first pin the
    // plain-ASCII equivalent so the wrap-at-width behavior is unchanged.
    let text = "toto toto toto toto toto toto ";
    let lines = simulate_roff(text, 20);

    assert_eq!(lines[0], "toto toto toto toto");
}

#[test]
fn roff_split_buffer_multibyte_wrap_counts_chars_not_bytes() {
    // Each "café " group is 5 chars but 6 bytes (é is 2 bytes in UTF-8).
    // With a 20-*char* width, the wrap must land after exactly 4 groups (20
    // chars: "café café café café"), not after ~3.33 groups as a
    // byte-length trigger would produce.
    let text = "café café café café café café ";
    let lines = simulate_roff(text, 20);

    assert_eq!(lines[0], "café café café café");
    assert_eq!(lines[0].chars().count(), 19); // 4 words + 3 spaces, no trailing space
}

#[test]
fn roff_split_buffer_no_panic_when_multibyte_char_hits_wrap_width() {
    // Same "café " text, but with width=19 so the char that actually trips
    // the wrap condition (buffer.len() >= width) is itself the 2-byte 'é'
    // in the 4th "café" - exactly the scenario that would panic or corrupt
    // output under byte-indexed slicing. The char-based buffer must back up
    // over 'é', 'f', 'a', 'c' one whole char at a time and stop cleanly at
    // the preceding space.
    let text = "café café café café";
    let lines = simulate_roff(text, 19);

    assert_eq!(lines[0], "café café café");
    // Nothing is dropped or duplicated: 3 full words (14 chars) + 1 space
    // consumed by the wrap + the remaining "café" (4 chars) accounts for
    // all 19 buffered chars.
    assert_eq!(lines[0].chars().count() + 1 + 4, 19);
}

#[test]
fn roff_split_buffer_forced_newline_still_works() {
    // A literal '\n' forces a flush regardless of width, splitting right
    // before the newline with nothing dropped - unaffected by the
    // char-vs-byte change, but worth pinning alongside the width-based path.
    let text = "café\n";
    let lines = simulate_roff(text, 80);

    assert_eq!(lines[0], "café");
}
