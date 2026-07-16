// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Description tables for dungeon paintings (an jdbkmoria extension, see paintings.rs).
//
// Descriptions are flavor text shown by the look command. Harmless paintings
// draw on the classical oil-painting tradition; monster and loot paintings
// must telegraph their nature, since gameplay follows the description.
//
// Keep each description under about 62 characters: it is shown on the
// single message line as "It depicts {description}." and must fit 80 columns.

// Paintings with no effect beyond their description.
pub const HARMLESS_PAINTINGS: [&str; 32] = [
    "a still life of ripe fruit beside a guttering candle",
    "a seascape, a ship heeling before a towering green wave",
    "a stern noblewoman in black, a lace ruff at her throat",
    "a night sky boiling with stars above a sleeping village",
    "a haywain fording a placid river beneath elm trees",
    "a peasant wedding feast at a long crowded table",
    "a wanderer above a sea of fog, his back turned to you",
    "a girl glancing back, a pearl earring catching the light",
    "an old windmill, its sails gilded by the dying light",
    "a wooden bridge over a pond thick with water lilies",
    "skaters wheeling on a frozen river under a leaden sky",
    "a bowl of sunflowers, the paint laid on thick as butter",
    "a barmaid at a mirrored counter heaped with oranges",
    "two card players eyeing each other over a bare table",
    "an old warship towed to her last berth by a black tug",
    "an anatomy lesson proceeding by lamplight",
    "a royal household gathered before a tall dim mirror",
    "a raft of shipwrecked men hailing a distant sail",
    "a coronation crowded with lords in ermine and velvet",
    "a grim farmer and his wife, pitchfork held upright",
    "a lady on a garden swing, one slipper flying free",
    "a moonlit harbor where fishing boats ride at anchor",
    "a mountain of ice grinding a ship to splinters",
    "a wheat field under a whirling storm of crows",
    "hunters returning through snow, their hounds at heel",
    "a bowl of irises against a buttermilk wall",
    "a cathedral facade dissolving in morning light",
    "a bison in ochre and charcoal, crude yet vigorous",
    "a scholar in his study, a skull upon the sill",
    "dancers in gauze skirts waiting in a theatre wing",
    "a laughing cavalier in slashed sleeves",
    "an empress in a gown sewn with mosaic gold",
];

// Monster paintings hold a real creature (see Painting::creature_id); the
// look command splices the creature's name into one of these templates via
// str::replace, so every template must contain exactly one "{}". The name
// arrives with its article ("a Giant Red Ant"). Longest creature names run
// to about 28 characters with the article, so keep each template at 42
// characters or fewer (counting its "{}") to stay inside 80 columns.

// Templates for creatures that leap from the canvas and fight hand to hand.
pub const MELEE_PAINTING_TEMPLATES: [&str; 6] = [
    "{} poised as if to spring from the frame",
    "a battle scene where {} turns to face you",
    "{} rendered so vividly it seems to breathe",
    "a moonlit hunt led by {} with wet eyes",
    "{} glowering from a field of black crags",
    "{} whose painted eyes seem to follow you",
];

// Templates for spell-casting creatures that bolt you from the canvas.
pub const RANGED_PAINTING_TEMPLATES: [&str; 5] = [
    "{} amid candles that burn with black flame",
    "{} tracing a glyph that still seems to glow",
    "{} with a mote of pale light at its command",
    "{} within a chalk ring scrawled with runes",
    "{} regarding you over a table of old bones",
];

// Magical paintings the player may reach into. Fake (trapped) paintings
// use this same table so the two cannot be told apart by description.
pub const LOOT_PAINTINGS: [&str; 8] = [
    "a banquet of roast fowl and wine that glistens wetly",
    "an armory wall hung with bright-edged swords and axes",
    "a wizard's study crowded with scrolls and phials",
    "a torchlit vault heaped with gear and provisions",
    "a merchant's stall that shimmers like heat over a road",
    "a pantry of cheeses, salted meat, and coiled rope",
    "a scriptorium, the ink still wet on a curling scroll",
    "a campfire with packs and weapons stacked by a stone",
];

// Paintings whose soporific scenes lull the viewer into an enchanted sleep.
pub const SLEEP_PAINTINGS: [&str; 4] = [
    "a field of scarlet poppies nodding in warm sun",
    "a curtained bed turned down beside a dying fire",
    "a shepherd asleep in tall grass, his flock adrift",
    "a moonlit pool where white lotus blossoms drowse",
];

// Paintings that fling the viewer across the level, as a teleport trap does.
pub const TELEPORT_PAINTINGS: [&str; 4] = [
    "a spiral stair descending into mist; the steps turn",
    "a doorway opening onto a corridor just like this one",
    "a whirlpool of grey water under no sky at all",
    "an endless hall of arches, each smaller than the last",
];

// The one painting per level that is a map of the level itself.
pub const MAP_PAINTING: &str = "a precise chart of halls; you recognize this very level";
