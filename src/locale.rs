// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// jdbkmoria extension: locale support (translations, number formatting,
// locale digit glyphs).
//
// The active locale is selected once at startup (before any game output)
// from, in order of precedence:
//
//   1. the `-l LOCALE` command line option
//   2. the `JDBKMORIA_LOCALE` environment variable
//   3. the POSIX locale environment (`LC_ALL`, then `LC_MESSAGES`, then `LANG`)
//   4. the default, `en_US`
//
// Message translation is keyed by the original en_US string (which doubles
// as the fallback), looked up in a per-locale sorted catalog. Templates use
// `{}` (sequential) and `{0}`/`{1}` (positional) placeholders so that a
// translation may reorder arguments. Number formatting (digit grouping,
// separators, digit glyphs) is data-driven per locale so future locales
// such as en_IN (3;2 grouping) or ar_AE (Arabic-Indic digits) only need a
// new `LocaleDef`.

use crate::globals::RacyCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleId {
    EnUs,
    EnGb,
    FrCa,
}

pub struct LocaleDef {
    pub id: LocaleId,

    // Canonical tag, e.g. "fr_CA"
    pub tag: &'static str,

    // ISO 639-1 language code, e.g. "fr"
    pub language: &'static str,

    // Sorted-by-key (byte order) list of (en_US key, translation).
    pub catalog: &'static [(&'static str, &'static str)],

    // Digit-grouping separator inserted by `format_number` (e.g. "," or " ").
    pub group_separator: &'static str,

    // Decimal separator (reserved for future use; the game currently only
    // displays integers).
    pub decimal_separator: &'static str,

    // Digit group sizes from the right; the last entry repeats.
    // en_US: [3] -> 1,234,567.  en_IN would be [3, 2] -> 12,34,567.
    // Empty disables grouping.
    pub grouping: &'static [u8],

    // Glyphs for the digits 0-9, used for numerals drawn on the dungeon
    // map (store entrances) and by `format_number`. ASCII for en/fr;
    // a future ar_AE could use Arabic-Indic digits here.
    pub digits: [char; 10],

    // Plural suffix rewrite rules applied (in order) to item names when the
    // count is not 1. Item name templates mark the pluralization point with
    // a trailing `~` (e.g. "& Scroll~"); in the singular the `~` is simply
    // removed. en_US: ch~ -> ches, ~ -> s.
    pub plural_suffixes: &'static [(&'static str, &'static str)],

    // Single-letter answer keys for yes/no, gender, and ring-hand prompts,
    // matched case-insensitively (stored uppercase). These track the letter
    // shown in the translated prompt (e.g. fr_CA shows "[o/n]" for
    // "oui"/"non" and "h)" for "Homme"), not the game's movement/command
    // keybindings, which stay fixed across locales.
    pub yes_key: char,
    pub no_key: char,
    pub male_key: char,
    pub female_key: char,
    // Which hand a ring goes on: en "(l/r/L/R)", fr "(g/d/G/D)".
    pub ring_left_key: char,
    pub ring_right_key: char,
}

pub static EN_US: LocaleDef = LocaleDef {
    id: LocaleId::EnUs,
    tag: "en_US",
    language: "en",
    catalog: &[],
    group_separator: ",",
    decimal_separator: ".",
    grouping: &[3],
    digits: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
    plural_suffixes: &[("ch~", "ches"), ("~", "s")],
    yes_key: 'Y',
    no_key: 'N',
    male_key: 'M',
    female_key: 'F',
    ring_left_key: 'L',
    ring_right_key: 'R',
};

pub static EN_GB: LocaleDef = LocaleDef {
    id: LocaleId::EnGb,
    tag: "en_GB",
    language: "en",
    catalog: crate::locale_data_en_gb::CATALOG,
    group_separator: ",",
    decimal_separator: ".",
    grouping: &[3],
    digits: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
    plural_suffixes: &[("ch~", "ches"), ("~", "s")],
    yes_key: 'Y',
    no_key: 'N',
    male_key: 'M',
    female_key: 'F',
    ring_left_key: 'L',
    ring_right_key: 'R',
};

pub static FR_CA: LocaleDef = LocaleDef {
    id: LocaleId::FrCa,
    tag: "fr_CA",
    language: "fr",
    catalog: crate::locale_data_fr_ca::CATALOG,
    // French Canadian style: space-grouped thousands, comma decimals.
    group_separator: "\u{00A0}",
    decimal_separator: ",",
    grouping: &[3],
    digits: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
    plural_suffixes: &[("eau~", "eaux"), ("al~", "aux"), ("~", "s")],
    // "oui"/"non", "h)"/"f)" ("Homme"/"Femme"), "g"/"d" ("gauche"/"droite").
    yes_key: 'O',
    no_key: 'N',
    male_key: 'H',
    female_key: 'F',
    ring_left_key: 'G',
    ring_right_key: 'D',
};

// All supported locales, in the order tried for language-only matches
// (the first entry for a language is that language's default region).
pub static LOCALES: &[&LocaleDef] = &[&EN_US, &EN_GB, &FR_CA];

static CURRENT: RacyCell<&'static LocaleDef> = RacyCell::new(&EN_US);

pub fn locale() -> &'static LocaleDef {
    CURRENT.get()
}

pub fn set_locale(id: LocaleId) {
    for def in LOCALES {
        if def.id == id {
            *CURRENT.get() = def;
            return;
        }
    }
}

// Parse a locale tag such as "fr_CA", "fr-ca", "fr_CA.UTF-8@euro", or just
// "fr". Case-insensitive; `-` and `_` are interchangeable; any `.codeset`
// or `@modifier` suffix is ignored. A language-only tag selects that
// language's default region. Returns None for unknown/unsupported tags.
pub fn parse_locale_tag(tag: &str) -> Option<LocaleId> {
    let base = tag.split(['.', '@']).next().unwrap_or("");
    let normalized = base.trim().replace('-', "_").to_lowercase();

    if normalized.is_empty() {
        return None;
    }

    for def in LOCALES {
        if def.tag.to_lowercase() == normalized {
            return Some(def.id);
        }
    }

    // Language-only match, e.g. "en" -> en_US, "fr" -> fr_CA
    for def in LOCALES {
        if def.language == normalized {
            return Some(def.id);
        }
    }

    None
}

// Determine the locale from the POSIX environment: LC_ALL beats
// LC_MESSAGES beats LANG. Unset, empty, or unsupported values fall through.
pub fn detect_system_locale() -> Option<LocaleId> {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(var) {
            if value.is_empty() {
                continue;
            }
            if let Some(id) = parse_locale_tag(&value) {
                return Some(id);
            }
            // A set-but-unsupported categorical value (e.g. "C", "POSIX",
            // "de_DE") still ends the search for that precedence level
            // only when it names a real locale we don't ship; "C"/"POSIX"
            // just fall through to the next variable.
        }
    }
    None
}

// Select the active locale at startup. `cli_override` is the value of the
// `-l` option, if given. Returns Err with the offending tag when an
// explicit override (CLI or JDBKMORIA_LOCALE) names an unsupported locale.
pub fn initialize_locale(cli_override: Option<&str>) -> Result<(), String> {
    if let Some(tag) = cli_override {
        match parse_locale_tag(tag) {
            Some(id) => {
                set_locale(id);
                return Ok(());
            }
            None => return Err(tag.to_string()),
        }
    }

    if let Ok(tag) = std::env::var("JDBKMORIA_LOCALE") {
        if !tag.is_empty() {
            match parse_locale_tag(&tag) {
                Some(id) => {
                    set_locale(id);
                    return Ok(());
                }
                None => return Err(tag),
            }
        }
    }

    set_locale(detect_system_locale().unwrap_or(LocaleId::EnUs));

    Ok(())
}

// Look up the translation for `key` in the active locale's catalog,
// falling back to the key itself (the en_US text).
pub fn tr(key: &str) -> &str {
    let catalog = locale().catalog;
    match catalog.binary_search_by(|(k, _)| (*k).cmp(key)) {
        Ok(index) => catalog[index].1,
        Err(_) => key,
    }
}

// Render a translated template with positional arguments. Supports `{}`
// (next sequential argument), `{0}`/`{1}`/... (explicit index, which does
// not advance the sequential counter), and `{{`/`}}` escapes. An
// out-of-range reference is left in place verbatim so mistakes are visible
// rather than silently dropped.
pub fn tr_format(template: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut chars = template.chars().peekable();
    let mut next_arg = 0usize;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    out.push('{');
                    continue;
                }
                let mut spec = String::new();
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                    spec.push(inner);
                }
                if !closed {
                    // Unterminated brace: emit as-is.
                    out.push('{');
                    out.push_str(&spec);
                    continue;
                }
                let index = if spec.is_empty() {
                    let index = next_arg;
                    next_arg += 1;
                    Some(index)
                } else {
                    spec.parse::<usize>().ok()
                };
                match index {
                    Some(index) if index < args.len() => out.push_str(&args[index]),
                    _ => {
                        out.push('{');
                        out.push_str(&spec);
                        out.push('}');
                    }
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                }
                out.push('}');
            }
            _ => out.push(ch),
        }
    }

    out
}

// Count the placeholders a template consumes, and validate explicit
// indices. Returns (max arity, valid). Used by catalog validation tests.
pub fn template_arity(template: &str) -> (usize, bool) {
    let mut chars = template.chars().peekable();
    let mut sequential = 0usize;
    let mut max_arity = 0usize;
    let mut valid = true;

    while let Some(ch) = chars.next() {
        if ch != '{' {
            continue;
        }
        if chars.peek() == Some(&'{') {
            chars.next();
            continue;
        }
        let mut spec = String::new();
        let mut closed = false;
        for inner in chars.by_ref() {
            if inner == '}' {
                closed = true;
                break;
            }
            spec.push(inner);
        }
        if !closed {
            valid = false;
            break;
        }
        if spec.is_empty() {
            sequential += 1;
            max_arity = max_arity.max(sequential);
        } else {
            match spec.parse::<usize>() {
                Ok(index) => max_arity = max_arity.max(index + 1),
                Err(_) => valid = false,
            }
        }
    }

    (max_arity, valid)
}

// Format an integer per the active locale: digit grouping plus the
// locale's digit glyphs.
pub fn format_number(value: i64) -> String {
    let def = locale();
    let digits = value.unsigned_abs().to_string();

    let grouped = group_digits(&digits, def.grouping, def.group_separator);
    let localized = map_digits(&grouped, &def.digits);

    if value < 0 {
        format!("-{}", localized)
    } else {
        localized
    }
}

// Insert group separators into a plain ASCII digit string. Group sizes are
// taken from `grouping` right-to-left, with the final size repeating.
pub fn group_digits(digits: &str, grouping: &[u8], separator: &str) -> String {
    if grouping.is_empty() || separator.is_empty() {
        return digits.to_string();
    }

    let bytes = digits.as_bytes();
    let mut boundaries: Vec<usize> = Vec::new();
    let mut position = bytes.len();
    let mut group_index = 0usize;

    loop {
        let size = grouping[group_index.min(grouping.len() - 1)] as usize;
        if size == 0 || position <= size {
            break;
        }
        position -= size;
        boundaries.push(position);
        group_index += 1;
    }

    let mut out = String::with_capacity(digits.len() + boundaries.len() * separator.len());
    for (i, b) in bytes.iter().enumerate() {
        if boundaries.contains(&i) && i != 0 {
            out.push_str(separator);
        }
        out.push(*b as char);
    }
    // boundaries are positions where a separator goes BEFORE the byte
    // (never position 0), collected right-to-left; the contains() above
    // applies them left-to-right.
    out
}

// Translate ASCII digits in a string to the locale's digit glyphs.
pub fn map_digits(text: &str, digits: &[char; 10]) -> String {
    text.chars()
        .map(|ch| match ch.to_digit(10) {
            Some(d) => digits[d as usize],
            None => ch,
        })
        .collect()
}

// The map glyph for a numeral drawn on the dungeon map (store entrances).
// `ascii` is an ASCII digit byte, b'0'..=b'9'.
pub fn map_digit_glyph(ascii: u8) -> char {
    if ascii.is_ascii_digit() {
        locale().digits[(ascii - b'0') as usize]
    } else {
        ascii as char
    }
}

// Translate a message key: `tr!("You feel better.")`
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::locale::tr($key)
    };
}

// Translate and fill a message template:
// `tr_fmt!("{} hits you.", name)`. Arguments are rendered with `Display`
// before substitution, so translations may reorder them with `{0}`, `{1}`.
#[macro_export]
macro_rules! tr_fmt {
    ($key:expr $(, $arg:expr)* $(,)?) => {
        $crate::locale::tr_format($crate::locale::tr($key), &[$(format!("{}", $arg)),*])
    };
}
