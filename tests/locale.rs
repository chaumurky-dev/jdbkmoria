// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// jdbkmoria extension: locale system tests.
//
// Everything lives in a single #[test] fn: locale state (like all game
// state) is process-wide, and the environment-variable tests must not
// interleave with each other.

use jdbkmoria::locale::{
    detect_system_locale, format_number, group_digits, initialize_locale, locale, parse_locale_tag, set_locale, template_arity, tr, tr_format, LocaleId, EN_GB, EN_US, FR_CA, LOCALES,
};

fn assert_catalog_valid(def: &jdbkmoria::locale::LocaleDef) {
    let catalog = def.catalog;

    for window in catalog.windows(2) {
        assert!(
            window[0].0 < window[1].0,
            "{} catalog not strictly sorted at key {:?} >= {:?}",
            def.tag,
            window[0].0,
            window[1].0
        );
    }

    for (key, translation) in catalog {
        let (key_arity, key_valid) = template_arity(key);
        let (tr_arity, tr_valid) = template_arity(translation);
        // A handful of keys legitimately contain a literal, un-doubled '{'
        // (tile-legend lines like "{ - Arrow..."); they only ever pass
        // through tr!, never tr_format. Key and translation must simply
        // agree on validity so a translator can't break a real template.
        assert_eq!(
            key_valid, tr_valid,
            "{}: placeholder validity mismatch for key {:?} -> {:?}",
            def.tag, key, translation
        );
        assert_eq!(
            key_arity, tr_arity,
            "{}: placeholder arity mismatch for key {:?} -> {:?}",
            def.tag, key, translation
        );
        assert!(!translation.is_empty(), "{}: empty translation for key {:?}", def.tag, key);
    }
}

#[test]
fn locale_system() {
    // --- tag parsing ---
    assert_eq!(parse_locale_tag("en_US"), Some(LocaleId::EnUs));
    assert_eq!(parse_locale_tag("en-us"), Some(LocaleId::EnUs));
    assert_eq!(parse_locale_tag("en_GB.UTF-8"), Some(LocaleId::EnGb));
    assert_eq!(parse_locale_tag("fr_CA.ISO8859-1@euro"), Some(LocaleId::FrCa));
    assert_eq!(parse_locale_tag("FR-CA"), Some(LocaleId::FrCa));
    // language-only tags select the language's default region
    assert_eq!(parse_locale_tag("en"), Some(LocaleId::EnUs));
    assert_eq!(parse_locale_tag("fr"), Some(LocaleId::FrCa));
    assert_eq!(parse_locale_tag("fr.UTF-8"), Some(LocaleId::FrCa));
    // unsupported
    assert_eq!(parse_locale_tag("de_DE"), None);
    assert_eq!(parse_locale_tag("C"), None);
    assert_eq!(parse_locale_tag("POSIX"), None);
    assert_eq!(parse_locale_tag(""), None);

    // --- system locale detection: LC_ALL > LC_MESSAGES > LANG ---
    std::env::remove_var("LC_ALL");
    std::env::remove_var("LC_MESSAGES");
    std::env::remove_var("LANG");
    assert_eq!(detect_system_locale(), None);

    std::env::set_var("LANG", "fr_CA.UTF-8");
    assert_eq!(detect_system_locale(), Some(LocaleId::FrCa));

    std::env::set_var("LC_MESSAGES", "en_GB.UTF-8");
    assert_eq!(detect_system_locale(), Some(LocaleId::EnGb));

    std::env::set_var("LC_ALL", "en_US.UTF-8");
    assert_eq!(detect_system_locale(), Some(LocaleId::EnUs));

    // "C"/"POSIX" fall through to the next variable
    std::env::set_var("LC_ALL", "C");
    assert_eq!(detect_system_locale(), Some(LocaleId::EnGb));

    // --- initialize_locale precedence ---
    std::env::set_var("JDBKMORIA_LOCALE", "fr_CA");
    assert!(initialize_locale(None).is_ok());
    assert_eq!(locale().id, LocaleId::FrCa);

    // CLI override beats JDBKMORIA_LOCALE
    assert!(initialize_locale(Some("en_GB")).is_ok());
    assert_eq!(locale().id, LocaleId::EnGb);

    // bad explicit tags are errors
    assert!(initialize_locale(Some("xx_XX")).is_err());
    std::env::set_var("JDBKMORIA_LOCALE", "xx_XX");
    assert!(initialize_locale(None).is_err());
    std::env::remove_var("JDBKMORIA_LOCALE");

    // no override, no usable env (LC_ALL=C wins over nothing; LC_MESSAGES set)
    std::env::remove_var("LC_ALL");
    std::env::remove_var("LC_MESSAGES");
    std::env::remove_var("LANG");
    assert!(initialize_locale(None).is_ok());
    assert_eq!(locale().id, LocaleId::EnUs);

    // --- template formatting ---
    assert_eq!(tr_format("You feel better.", &[]), "You feel better.");
    assert_eq!(tr_format("{} hits you.", &["The Kobold".to_string()]), "The Kobold hits you.");
    assert_eq!(
        tr_format("{1} takes {0} damage.", &["3".to_string(), "The rat".to_string()]),
        "The rat takes 3 damage."
    );
    // explicit indices do not advance the sequential counter
    assert_eq!(
        tr_format("{0} and {} and {1}", &["a".to_string(), "b".to_string()]),
        "a and a and b"
    );
    assert_eq!(tr_format("{{literal}}", &[]), "{literal}");
    // out-of-range placeholders are left visible
    assert_eq!(tr_format("{7} oops", &["a".to_string()]), "{7} oops");

    assert_eq!(template_arity("no args"), (0, true));
    assert_eq!(template_arity("{} {}"), (2, true));
    assert_eq!(template_arity("{1} {}"), (2, true));
    assert_eq!(template_arity("{{}}"), (0, true));
    assert_eq!(template_arity("{bad"), (0, false));

    // --- number formatting ---
    set_locale(LocaleId::EnUs);
    assert_eq!(format_number(0), "0");
    assert_eq!(format_number(999), "999");
    assert_eq!(format_number(1000), "1,000");
    assert_eq!(format_number(1234567), "1,234,567");
    assert_eq!(format_number(-45210), "-45,210");

    set_locale(LocaleId::FrCa);
    assert_eq!(format_number(1234567), "1\u{00A0}234\u{00A0}567");
    assert_eq!(format_number(-1000), "-1\u{00A0}000");

    // future-locale grouping shapes (e.g. en_IN 3;2 lakh/crore grouping)
    assert_eq!(group_digits("1234567", &[3, 2], ","), "12,34,567");
    assert_eq!(group_digits("1234567", &[], ","), "1234567");
    assert_eq!(group_digits("123", &[3], ","), "123");

    // --- store entrance digit glyphs ---
    set_locale(LocaleId::FrCa);
    assert_eq!(jdbkmoria::locale::map_digit_glyph(b'1'), '1');
    assert_eq!(jdbkmoria::locale::map_digit_glyph(b'#'), '#');

    // --- tr lookup and fallback ---
    set_locale(LocaleId::EnUs);
    assert_eq!(tr("You feel better."), "You feel better.");
    set_locale(LocaleId::FrCa);
    // any key missing from the catalog falls back to the en_US text
    assert_eq!(tr("__definitely_not_a_key__"), "__definitely_not_a_key__");
    if let Some((key, translation)) = FR_CA.catalog.first() {
        assert_eq!(tr(key), *translation);
    }

    // --- catalog validity (sortedness, placeholder parity) ---
    for def in LOCALES {
        assert_catalog_valid(def);
    }
    assert!(EN_US.catalog.is_empty(), "en_US is the key language; its catalog must stay empty");
    assert_catalog_valid(&EN_GB);
    assert_catalog_valid(&FR_CA);

    // restore default for any later state
    set_locale(LocaleId::EnUs);
}
