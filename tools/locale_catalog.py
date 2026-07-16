#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Locale catalog tooling for jdbkmoria.

Subcommands:

  extract   Collect every translatable key into JSONL on stdout:
              - literal first arguments of tr!("...") / tr_fmt!("...") in src/*.rs
              - display strings from the static data tables (creature names,
                item names, store owners/speech, recall fragments, player
                races/classes/titles/backgrounds, spell names, adjective
                lists, painting templates)
            Each line: {"key": "..."}. Sorted, de-duplicated.

  gen       Read one or more translation JSONL files (lines of
            {"key": "...", "tr": "..."}) given as arguments, merge them,
            and emit a sorted Rust CATALOG array body on stdout, ready to
            paste into src/locale_data_*.rs. Duplicate keys must agree.
            Keys whose translation equals the key are dropped (fallback
            handles them).

Used when adding a locale: run `extract`, hand chunks to translators, then
`gen` the catalog module. See PORTING.md ("Locale support").
"""

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "src"

# Data-table files whose string literals are (almost) all display text.
DATA_FILES = [
    "data_creatures.rs",
    "data_treasure.rs",
    "data_player.rs",
    "data_tables.rs",
    "data_recall.rs",
    "data_store_owners.rs",
    "data_paintings.rs",
]

# Arrays inside those files that must NOT be translated.
EXCLUDED_ARRAYS = ["SYLLABLES"]

# Translatable consts living outside the data files, as (file, const name).
# Their string literals reach tr!/tr_fmt! through a variable at the use
# site, so TR_RE cannot see them. An entry may be an array of literals or a
# single (raw) string literal.
EXTRA_ITEMS = [
    ("game.rs", "GAME_OPTIONS"),  # options-menu labels
    ("main.rs", "USAGE_INSTRUCTIONS"),  # -h usage text (raw string)
    ("paintings.rs", "MUNDANE_REACH_MESSAGES"),
]

TR_RE = re.compile(r'tr(?:_fmt)?!\(\s*"((?:[^"\\]|\\.)*)"')


def rust_strings(text: str):
    """Yield the raw contents of normal "..." string literals, correctly
    skipping // comments and b'..'/'..' char/byte literals (whose quotes
    would otherwise mis-pair the strings, e.g. sprite bytes like b'"')."""
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "/" and text[i : i + 2] == "//":
            i = text.find("\n", i)
            if i == -1:
                return
        elif c == "'" or (c == "b" and text[i : i + 2] == "b'"):
            i += 1 if c == "'" else 2
            if i < n and text[i] == "\\":
                i += 1
            i += 2  # the char itself and the closing quote
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\":
                    j += 1
                j += 1
            yield text[i + 1 : j]
            i = j + 1
        else:
            i += 1

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\", "0": "\0", "'": "'"}


def unescape(s: str) -> str:
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            n = s[i + 1]
            if n in ESCAPES:
                out.append(ESCAPES[n])
                i += 2
                continue
            if n == "u" and s[i + 2 : i + 3] == "{":
                j = s.index("}", i)
                out.append(chr(int(s[i + 3 : j], 16)))
                i = j + 1
                continue
        out.append(c)
        i += 1
    return "".join(out)


def escape_rust(s: str) -> str:
    return (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
    )


def strip_comments(text: str) -> str:
    # Remove // comments (good enough for this codebase; no /* */ in data files)
    return re.sub(r"//[^\n]*", "", text)


def strip_excluded_arrays(text: str) -> str:
    for name in EXCLUDED_ARRAYS:
        m = re.search(rf"\b{name}\b[^=]*=\s*\[", text)
        if not m:
            continue
        depth = 1
        i = m.end()
        while i < len(text) and depth:
            if text[i] == "[":
                depth += 1
            elif text[i] == "]":
                depth -= 1
            i += 1
        text = text[: m.start()] + text[i:]
    return text


def item_body(text: str, name: str):
    """The initializer of `NAME ... = <body>;` as ("raw", contents) for a
    raw string literal, or ("array", source text) for a bracketed array.
    Bracket matching is as naive as strip_excluded_arrays (fine as long as
    the array's strings contain no square brackets)."""
    m = re.search(rf"\b{name}\b[^=]*=\s*", text)
    if not m:
        sys.exit(f"item_body: {name} not found")
    i = m.end()
    raw = re.match(r'r(#*)"', text[i:])
    if raw:
        start = i + raw.end()
        end = text.index('"' + raw.group(1), start)
        return "raw", text[start:end]
    if text[i] != "[":
        sys.exit(f"item_body: {name} is neither an array nor a raw string")
    depth, j = 1, i + 1
    while j < len(text) and depth:
        if text[j] == "[":
            depth += 1
        elif text[j] == "]":
            depth -= 1
        j += 1
    return "array", text[i:j]


def extract() -> None:
    keys = set()

    for path in sorted(SRC.glob("*.rs")):
        text = strip_comments(path.read_text())
        if path.name in DATA_FILES:
            text = strip_excluded_arrays(text)
            for raw in rust_strings(text):
                s = unescape(raw)
                if s.strip():
                    keys.add(s)
        else:
            for m in TR_RE.finditer(text):
                s = unescape(m.group(1))
                if s.strip():
                    keys.add(s)

    for fname, item in EXTRA_ITEMS:
        text = strip_comments((SRC / fname).read_text())
        kind, body = item_body(text, item)
        if kind == "raw":
            keys.add(body)
        else:
            for raw in rust_strings(body):
                s = unescape(raw)
                if s.strip():
                    keys.add(s)

    for key in sorted(keys):
        print(json.dumps({"key": key}, ensure_ascii=False))


def gen(paths) -> None:
    valid_keys = None
    if paths and paths[0] == "--filter":
        valid_keys = {
            json.loads(line)["key"]
            for line in Path(paths[1]).read_text().splitlines()
            if line.strip()
        }
        paths = paths[2:]

    entries = {}
    for p in paths:
        for line_no, line in enumerate(Path(p).read_text().splitlines(), 1):
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            key, tr = obj["key"], obj["tr"]
            if valid_keys is not None and key not in valid_keys:
                print(f"note: dropping unknown key from {p}: {key!r}", file=sys.stderr)
                continue
            if key in entries and entries[key] != tr:
                sys.exit(f"{p}:{line_no}: conflicting translation for key {key!r}")
            entries[key] = tr

    for key in sorted(entries, key=lambda k: k.encode()):
        tr = entries[key]
        if tr == key:
            continue
        print(f'    ("{escape_rust(key)}", "{escape_rust(tr)}"),')


def main() -> None:
    if len(sys.argv) < 2 or sys.argv[1] not in ("extract", "gen"):
        sys.exit(__doc__)
    if sys.argv[1] == "extract":
        extract()
    else:
        gen(sys.argv[2:])


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        pass
