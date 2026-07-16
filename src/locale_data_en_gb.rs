// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// jdbkmoria extension: en_GB message catalog.
//
// Sparse: only strings whose British spelling differs from the en_US
// original appear here; everything else falls back to the en_US key.
// Entries MUST be sorted by key (byte order) — `tests/locale.rs` enforces
// this along with placeholder parity.

pub static CATALOG: &[(&str, &str)] = &[
    (" It has an armor rating of {}", " It has an armour rating of {}"),
    ("& Hairy %s Mold~", "& Hairy %s Mould~"),
    ("& Hairy Mold~", "& Hairy Mould~"),
    ("& Slime Mold~", "& Slime Mould~"),
    ("( - Soft armor.", "( - Soft armour."),
    ("*Enchant Armor*", "*Enchant Armour*"),
    ("2 - Entrance to Armory.", "2 - Entrance to Armoury."),
    ("A gas of scintillating colors surrounds you!", "A gas of scintillating colours surrounds you!"),
    ("Armory", "Armoury"),
    ("Based on Umoria 5.7.15, released under a GPL-3.0-or-later license.", "Based on Umoria 5.7.15, released under a GPL-3.0-or-later licence."),
    ("Black Mold", "Black Mould"),
    ("Brown Mold", "Brown Mould"),
    ("Crimson Mold", "Crimson Mould"),
    ("Curse Armor", "Curse Armour"),
    ("Darg-Low the Grim      (Human)      Armory", "Darg-Low the Grim      (Human)      Armoury"),
    ("Disenchanter Mold", "Disenchanter Mould"),
    ("Enchant Armor", "Enchant Armour"),
    ("Full Plate Armor", "Full Plate Armour"),
    ("Green Mold", "Green Mould"),
    ("Hairy Mold", "Hairy Mould"),
    ("Hard Leather Armor", "Hard Leather Armour"),
    ("Laminated Armor", "Laminated Armour"),
    ("Mauglim the Horrible   (Half-Orc)   Armory", "Mauglim the Horrible   (Half-Orc)   Armoury"),
    ("Mauglin the Grumpy     (Dwarf)      Armory", "Mauglin the Grumpy     (Dwarf)      Armoury"),
    ("Metal Brigandine Armor", "Metal Brigandine Armour"),
    ("Metal Lamellar Armor", "Metal Lamellar Armour"),
    ("Partial Plate Armor", "Partial Plate Armour"),
    ("Red Mold", "Red Mould"),
    ("Ribbed Plate Armor", "Ribbed Plate Armour"),
    ("Shimmering Mold", "Shimmering Mould"),
    ("Slime Mold Juice", "Slime Mould Juice"),
    ("Soft Leather Armor", "Soft Leather Armour"),
    ("The colors fade to a dull grey.", "The colours fade to a dull grey."),
    ("Violet Mold", "Violet Mould"),
    ("Wooden Mold", "Wooden Mould"),
    ("Woven Cord Armor", "Woven Cord Armour"),
    ("Yellow Mold", "Yellow Mould"),
    ("You are paralyzed.", "You are paralysed."),
    ("You have blue-gray eyes, ", "You have blue-grey eyes, "),
    ("[ - Hard armor.", "[ - Hard armour."),
    ("] - Misc. armor.", "] - Misc. armour."),
    ("a moonlit harbor where fishing boats ride at anchor", "a moonlit harbour where fishing boats ride at anchor"),
    ("a precise chart of halls; you recognize this very level", "a precise chart of halls; you recognise this very level"),
    ("an armory wall hung with bright-edged swords and axes", "an armoury wall hung with bright-edged swords and axes"),
    ("m - Mold.", "m - Mould."),
];
