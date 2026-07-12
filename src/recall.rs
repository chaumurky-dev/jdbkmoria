// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Monster memory info -CJS-

use crate::config::monsters::defense::{CD_ANIMAL, CD_EVIL, CD_FROST, CD_INFRA, CD_MAX_HP, CD_NO_SLEEP, CD_UNDEAD, CD_WEAKNESS};
use crate::config::monsters::move_flags::{
    CM_1D2_OBJ, CM_2D2_OBJ, CM_4D2_OBJ, CM_60_RANDOM, CM_90_RANDOM, CM_ALL_MV_FLAGS, CM_ATTACK_ONLY, CM_CARRY_GOLD, CM_CARRY_OBJ,
    CM_INVISIBLE, CM_ONLY_MAGIC, CM_RANDOM_MOVE, CM_SMALL_OBJ, CM_SPECIAL, CM_TREASURE, CM_TR_SHIFT, CM_WIN,
};
use crate::config::monsters::spells::{CS_BREATHE, CS_BR_LIGHT, CS_FREQ, CS_SPELLS, CS_TEL_SHORT};
use crate::config::monsters::MON_ENDGAME_LEVEL;
use crate::data_creatures::{CREATURES_LIST, MONSTER_ATTACKS};
use crate::data_recall::{
    RECALL_DESCRIPTION_ATTACK_METHOD, RECALL_DESCRIPTION_ATTACK_TYPE, RECALL_DESCRIPTION_BREATH, RECALL_DESCRIPTION_HOW_MUCH,
    RECALL_DESCRIPTION_MOVE, RECALL_DESCRIPTION_SPELL, RECALL_DESCRIPTION_WEAKNESS,
};
use crate::game::game;
use crate::monster::{Creature, MON_MAX_ATTACKS, MON_MAX_CREATURES};
use crate::player::py;
use crate::recall_data::{creature_recall, Recall};
use crate::types::{Coord, MORIA_MESSAGE_SIZE};
use crate::ui::ESCAPE;
use crate::ui_io::{
    erase_line, get_input_confirmation_with_abort, get_key_input, put_string_clear_to_eol, terminal_restore_screen, terminal_save_screen,
};

fn plural(c: u16, ss: &'static str, sp: &'static str) -> &'static str {
    if c == 1 {
        ss
    } else {
        sp
    }
}

// Number of kills needed for information.
// the higher the level of the monster, the fewer the attacks you need,
// the more damage an attack does, the more attacks you need.
fn knowdamage(l: i32, a: i32, d: i32) -> bool {
    (4 + l) * a > 80 * d
}

// Buffers up printed text and word-wraps it across screen lines,
// mirroring the C `memoryPrint()`/`roff_buffer` behavior. -CJS-
struct Roff {
    buffer: Vec<u8>,
    print_line: i32,
}

impl Roff {
    fn new() -> Self {
        Roff { buffer: Vec::new(), print_line: 0 }
    }

    // Print out strings, filling up lines as we go.
    fn print(&mut self, text: &str) {
        for &byte in text.as_bytes() {
            self.buffer.push(byte);

            if byte == b'\n' || self.buffer.len() >= MORIA_MESSAGE_SIZE {
                let mut q = self.buffer.len() - 1;

                if byte != b'\n' {
                    while self.buffer[q] != b' ' {
                        q -= 1;
                    }
                }

                let line = String::from_utf8_lossy(&self.buffer[0..q]).into_owned();
                put_string_clear_to_eol(&line, Coord::new(self.print_line, 0));
                self.print_line += 1;

                self.buffer = self.buffer[q + 1..].to_vec();
            }
        }
    }
}

// Do we know anything about this monster?
fn memory_monster_known(memory: &Recall) -> bool {
    if game().wizard_mode {
        return true;
    }

    if memory.movement != 0 || memory.defenses != 0 || memory.kills != 0 || memory.spells != 0 || memory.deaths != 0 {
        return true;
    }

    for &attack in memory.attacks.iter() {
        if attack != 0 {
            return true;
        }
    }

    false
}

fn memory_wizard_mode_init(memory: &mut Recall, creature: &Creature) {
    memory.kills = i16::MAX as u16;
    memory.wake = u8::MAX;
    memory.ignore = u8::MAX;

    let mut mv: u32 = ((creature.movement & CM_4D2_OBJ) != 0) as u32 * 8;
    mv += ((creature.movement & CM_2D2_OBJ) != 0) as u32 * 4;
    mv += ((creature.movement & CM_1D2_OBJ) != 0) as u32 * 2;
    mv += ((creature.movement & CM_90_RANDOM) != 0) as u32;
    mv += ((creature.movement & CM_60_RANDOM) != 0) as u32;

    memory.movement = (creature.movement & !CM_TREASURE) | (mv << CM_TR_SHIFT);
    memory.defenses = creature.defenses;

    if (creature.spells & CS_FREQ) != 0 {
        memory.spells = creature.spells | CS_FREQ;
    } else {
        memory.spells = creature.spells;
    }

    for i in 0..MON_MAX_ATTACKS {
        if creature.damage[i] == 0 {
            break;
        }
        memory.attacks[i] = u8::MAX;
    }

    // A little hack to enable the display of info for Quylthulgs.
    if (memory.movement & CM_ONLY_MAGIC) != 0 {
        memory.attacks[0] = u8::MAX;
    }
}

// Conflict history.
fn memory_conflict_history(roff: &mut Roff, deaths: u16, kills: u16) {
    if deaths != 0 {
        roff.print(&format!(
            "{} of the contributors to your monster memory {}",
            deaths,
            plural(deaths, "has", "have")
        ));
        roff.print(" been killed by this creature, and ");
        if kills == 0 {
            roff.print("it is not ever known to have been defeated.");
        } else {
            roff.print(&format!(
                "at least {} of the beasts {} been exterminated.",
                kills,
                plural(kills, "has", "have")
            ));
        }
    } else if kills != 0 {
        roff.print(&format!("At least {} of these creatures {}", kills, plural(kills, "has", "have")));
        roff.print(" been killed by contributors to your monster memory.");
    } else {
        roff.print("No known battles to the death are recalled.");
    }
}

// Immediately obvious.
fn memory_depth_found_at(roff: &mut Roff, mut level: u8, kills: u16) -> bool {
    let mut known = false;

    if level == 0 {
        known = true;
        roff.print(" It lives in the town");
    } else if kills != 0 {
        known = true;

        // The Balrog is a level 100 monster, but appears at 50 feet.
        if level > MON_ENDGAME_LEVEL {
            level = MON_ENDGAME_LEVEL;
        }

        roff.print(&format!(" It is normally found at depths of {} feet", level as i32 * 50));
    }

    known
}

fn memory_movement(roff: &mut Roff, rc_move: u32, monster_speed_raw: i32, is_known_in: bool) -> bool {
    let mut is_known = is_known_in;
    // the creatures_list speed value is 10 greater, so that it can be a uint8_t
    let monster_speed = monster_speed_raw - 10;

    if (rc_move & CM_ALL_MV_FLAGS) != 0 {
        if is_known {
            roff.print(", and");
        } else {
            roff.print(" It");
            is_known = true;
        }

        roff.print(" moves");

        if (rc_move & CM_RANDOM_MOVE) != 0 {
            roff.print(RECALL_DESCRIPTION_HOW_MUCH[((rc_move & CM_RANDOM_MOVE) >> 3) as usize]);
            roff.print(" erratically");
        }

        if monster_speed == 1 {
            roff.print(" at normal speed");
        } else {
            if (rc_move & CM_RANDOM_MOVE) != 0 {
                roff.print(", and");
            }

            if monster_speed <= 0 {
                if monster_speed == -1 {
                    roff.print(" very");
                } else if monster_speed < -1 {
                    roff.print(" incredibly");
                }
                roff.print(" slowly");
            } else {
                if monster_speed == 3 {
                    roff.print(" very");
                } else if monster_speed > 3 {
                    roff.print(" unbelievably");
                }
                roff.print(" quickly");
            }
        }
    }

    if (rc_move & CM_ATTACK_ONLY) != 0 {
        if is_known {
            roff.print(", but");
        } else {
            roff.print(" It");
            is_known = true;
        }

        roff.print(" does not deign to chase intruders");
    }

    if (rc_move & CM_ONLY_MAGIC) != 0 {
        if is_known {
            roff.print(", but");
        } else {
            roff.print(" It");
            is_known = true;
        }

        roff.print(" always moves and attacks by using magic");
    }

    is_known
}

// Kill it once to know experience, and quality (evil, undead, monstrous).
// The quality of being a dragon is obvious.
fn memory_kill_points(roff: &mut Roff, creature_defense: u16, monster_exp: u16, level: u8) {
    roff.print(" A kill of this");

    if (creature_defense & CD_ANIMAL) != 0 {
        roff.print(" natural");
    }
    if (creature_defense & CD_EVIL) != 0 {
        roff.print(" evil");
    }
    if (creature_defense & CD_UNDEAD) != 0 {
        roff.print(" undead");
    }

    let player_level = py().misc.level as i32;

    // calculate the integer exp part, can be larger than 64K when first
    // level character looks at Balrog info, so must store in long
    let quotient = monster_exp as i32 * level as i32 / player_level;

    // calculate the fractional exp part scaled by 100,
    // must use long arithmetic to avoid overflow
    let remainder = ((((monster_exp as i32 * level as i32) % player_level) * 1000 / player_level + 5) / 10) as u32;

    let plural_char = if quotient == 1 && remainder == 0 { '\0' } else { 's' };

    roff.print(&format!(" creature is worth {}.{:02} point{}", quotient, remainder, plural_char));

    let p: &str = if player_level / 10 == 1 {
        "th"
    } else {
        match player_level % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };

    let q: &str = if player_level == 8 || player_level == 11 || player_level == 18 { "n" } else { "" };

    roff.print(&format!(" for a{} {}{} level character.", q, player_level, p));
}

// Spells known, if have been used against us.
// Breath weapons or resistance might be known only because we cast spells at it.
fn memory_magic_skills(roff: &mut Roff, memory_spell_flags: u32, monster_spell_flags: u32, creature_spell_flags: u32) {
    let mut known = true;

    let mut spell_flags = memory_spell_flags;

    let mut i: u32 = 0;
    while (spell_flags & CS_BREATHE) != 0 {
        if (spell_flags & (CS_BR_LIGHT << i)) != 0 {
            spell_flags &= !(CS_BR_LIGHT << i);

            if known {
                if (monster_spell_flags & CS_FREQ) != 0 {
                    roff.print(" It can breathe ");
                } else {
                    roff.print(" It is resistant to ");
                }
                known = false;
            } else if (spell_flags & CS_BREATHE) != 0 {
                roff.print(", ");
            } else {
                roff.print(" and ");
            }
            roff.print(RECALL_DESCRIPTION_BREATH[i as usize]);
        }
        i += 1;
    }

    known = true;

    let mut j: u32 = 0;
    while (spell_flags & CS_SPELLS) != 0 {
        if (spell_flags & (CS_TEL_SHORT << j)) != 0 {
            spell_flags &= !(CS_TEL_SHORT << j);

            if known {
                if (memory_spell_flags & CS_BREATHE) != 0 {
                    roff.print(", and is also");
                } else {
                    roff.print(" It is");
                }
                roff.print(" magical, casting spells which ");
                known = false;
            } else if (spell_flags & CS_SPELLS) != 0 {
                roff.print(", ");
            } else {
                roff.print(" or ");
            }
            roff.print(RECALL_DESCRIPTION_SPELL[j as usize]);
        }
        j += 1;
    }

    if (memory_spell_flags & (CS_BREATHE | CS_SPELLS)) != 0 {
        // Could offset by level
        if (monster_spell_flags & CS_FREQ) > 5 {
            roff.print(&format!("; 1 time in {}", creature_spell_flags & CS_FREQ));
        }
        roff.print(".");
    }
}

// Do we know how hard they are to kill? Armor class, hit die.
fn memory_kill_difficulty(roff: &mut Roff, creature: &Creature, monster_kills: u16) {
    // the higher the level of the monster, the fewer the kills you need
    if monster_kills as u32 <= 304 / (4 + creature.level as u32) {
        return;
    }

    roff.print(&format!(" It has an armor rating of {}", creature.ac));

    roff.print(&format!(
        " and a{} life rating of {}d{}.",
        if (creature.defenses & CD_MAX_HP) != 0 { " maximized" } else { "" },
        creature.hit_die.dice,
        creature.hit_die.sides
    ));
}

// Do we know how clever they are? Special abilities.
fn memory_special_abilities(roff: &mut Roff, move_in: u32) {
    let mut known = true;
    let mut move_val = move_in;

    let mut i: u32 = 0;
    while (move_val & CM_SPECIAL) != 0 {
        if (move_val & (CM_INVISIBLE << i)) != 0 {
            move_val &= !(CM_INVISIBLE << i);

            if known {
                roff.print(" It can ");
                known = false;
            } else if (move_val & CM_SPECIAL) != 0 {
                roff.print(", ");
            } else {
                roff.print(" and ");
            }
            roff.print(RECALL_DESCRIPTION_MOVE[i as usize]);
        }
        i += 1;
    }

    if !known {
        roff.print(".");
    }
}

// Do we know its special weaknesses? Most defenses flags.
fn memory_weaknesses(roff: &mut Roff, defense_in: u16) {
    let mut known = true;
    let mut defense = defense_in;

    let mut i: u32 = 0;
    while (defense & CD_WEAKNESS) != 0 {
        if (defense & (CD_FROST << i)) != 0 {
            defense &= !(CD_FROST << i);
            if known {
                roff.print(" It is susceptible to ");
                known = false;
            } else if (defense & CD_WEAKNESS) != 0 {
                roff.print(", ");
            } else {
                roff.print(" and ");
            }
            roff.print(RECALL_DESCRIPTION_WEAKNESS[i as usize]);
        }
        i += 1;
    }

    if !known {
        roff.print(".");
    }
}

// Do we know how aware it is?
fn memory_awareness(roff: &mut Roff, creature: &Creature, memory: &Recall) {
    let wake = memory.wake as i32;

    if wake * wake > creature.sleep_counter as i32 || memory.ignore == u8::MAX || (creature.sleep_counter == 0 && memory.kills >= 10) {
        roff.print(" It ");

        if creature.sleep_counter > 200 {
            roff.print("prefers to ignore");
        } else if creature.sleep_counter > 95 {
            roff.print("pays very little attention to");
        } else if creature.sleep_counter > 75 {
            roff.print("pays little attention to");
        } else if creature.sleep_counter > 45 {
            roff.print("tends to overlook");
        } else if creature.sleep_counter > 25 {
            roff.print("takes quite a while to see");
        } else if creature.sleep_counter > 10 {
            roff.print("takes a while to see");
        } else if creature.sleep_counter > 5 {
            roff.print("is fairly observant of");
        } else if creature.sleep_counter > 3 {
            roff.print("is observant of");
        } else if creature.sleep_counter > 1 {
            roff.print("is very observant of");
        } else if creature.sleep_counter != 0 {
            roff.print("is vigilant for");
        } else {
            roff.print("is ever vigilant for");
        }

        roff.print(&format!(
            " intruders, which it may notice from {} feet.",
            10 * creature.area_affect_radius as i32
        ));
    }
}

// Do we know what it might carry?
fn memory_loot_carried(roff: &mut Roff, creature_move: u32, memory_move: u32) {
    if (memory_move & (CM_CARRY_OBJ | CM_CARRY_GOLD)) == 0 {
        return;
    }

    roff.print(" It may");

    let carrying_chance = (memory_move & CM_TREASURE) >> CM_TR_SHIFT;

    if carrying_chance == 1 {
        if (creature_move & CM_TREASURE) == CM_60_RANDOM {
            roff.print(" sometimes");
        } else {
            roff.print(" often");
        }
    } else if carrying_chance == 2 && (creature_move & CM_TREASURE) == (CM_60_RANDOM | CM_90_RANDOM) {
        roff.print(" often");
    }

    roff.print(" carry");

    let mut p: &str = if (memory_move & CM_SMALL_OBJ) != 0 { " small objects" } else { " objects" };

    if carrying_chance == 1 {
        p = if (memory_move & CM_SMALL_OBJ) != 0 { " a small object" } else { " an object" };
    } else if carrying_chance == 2 {
        roff.print(" one or two");
    } else {
        roff.print(&format!(" up to {}", carrying_chance));
    }

    if (memory_move & CM_CARRY_OBJ) != 0 {
        roff.print(p);
        if (memory_move & CM_CARRY_GOLD) != 0 {
            roff.print(" or treasure");
            if carrying_chance > 1 {
                roff.print("s");
            }
        }
        roff.print(".");
    } else if carrying_chance != 1 {
        roff.print(" treasures.");
    } else {
        roff.print(" treasure.");
    }
}

fn memory_attack_number_and_damage(roff: &mut Roff, memory: &Recall, creature: &Creature) {
    // We know about attacks it has used on us, and maybe the damage they do.
    // known_attacks is the total number of known attacks, used for punctuation
    let known_attacks = memory.attacks.iter().filter(|&&attack| attack != 0).count();

    // attack_count counts the attacks as printed, used for punctuation
    let mut attack_count = 0;

    for i in 0..MON_MAX_ATTACKS {
        let attack_id = creature.damage[i];
        if attack_id == 0 {
            break;
        }

        // don't print out unknown attacks
        if memory.attacks[i] == 0 {
            continue;
        }

        let mut attack_type = MONSTER_ATTACKS[attack_id as usize].type_id;
        let mut attack_description_id = MONSTER_ATTACKS[attack_id as usize].description_id;
        let dice = MONSTER_ATTACKS[attack_id as usize].dice;

        attack_count += 1;

        if attack_count == 1 {
            roff.print(" It can ");
        } else if attack_count == known_attacks {
            roff.print(", and ");
        } else {
            roff.print(", ");
        }

        if attack_description_id > 19 {
            attack_description_id = 0;
        }

        roff.print(RECALL_DESCRIPTION_ATTACK_METHOD[attack_description_id as usize]);

        if attack_type != 1 || (dice.dice > 0 && dice.sides > 0) {
            roff.print(" to ");

            if attack_type > 24 {
                attack_type = 0;
            }

            roff.print(RECALL_DESCRIPTION_ATTACK_TYPE[attack_type as usize]);

            if dice.dice != 0 && dice.sides != 0 {
                if knowdamage(creature.level as i32, memory.attacks[i] as i32, dice.dice as i32 * dice.sides as i32) {
                    // Loss of experience
                    if attack_type == 19 {
                        roff.print(" by");
                    } else {
                        roff.print(" with damage");
                    }

                    roff.print(&format!(" {}d{}", dice.dice, dice.sides));
                }
            }
        }
    }

    if attack_count != 0 {
        roff.print(".");
    } else if known_attacks > 0 && memory.attacks[0] >= 10 {
        roff.print(" It has no physical attacks.");
    } else {
        roff.print(" Nothing is known about its attack.");
    }
}

// Print out what we have discovered about this monster.
pub fn memory_recall(monster_id: i32) -> char {
    let idx = monster_id as usize;
    let creature = &CREATURES_LIST[idx];

    let mut saved_memory: Option<Recall> = None;

    if game().wizard_mode {
        saved_memory = Some(creature_recall()[idx]);
        memory_wizard_mode_init(&mut creature_recall()[idx], creature);
    }

    let memory = creature_recall()[idx];

    let mut roff = Roff::new();

    let spells = memory.spells & creature.spells & !CS_FREQ;

    // the CM_WIN property is always known, set it if a win monster
    let move_val = memory.movement | (creature.movement & CM_WIN);

    let defense = memory.defenses & creature.defenses;

    // Start the paragraph for the core monster description
    roff.print(&format!("The {}:\n", creature.name));

    memory_conflict_history(&mut roff, memory.deaths, memory.kills);
    let known = memory_depth_found_at(&mut roff, creature.level, memory.kills);
    let known = memory_movement(&mut roff, move_val, creature.speed as i32, known);

    // Finish off the paragraph with a period!
    if known {
        roff.print(".");
    }

    if memory.kills != 0 {
        memory_kill_points(&mut roff, creature.defenses, creature.kill_exp_value, creature.level);
    }

    memory_magic_skills(&mut roff, spells, memory.spells, creature.spells);

    memory_kill_difficulty(&mut roff, creature, memory.kills);

    memory_special_abilities(&mut roff, move_val);

    memory_weaknesses(&mut roff, defense);

    if (defense & CD_INFRA) != 0 {
        roff.print(" It is warm blooded");
    }

    if (defense & CD_NO_SLEEP) != 0 {
        if (defense & CD_INFRA) != 0 {
            roff.print(", and");
        } else {
            roff.print(" It");
        }
        roff.print(" cannot be charmed or slept");
    }

    if (defense & (CD_NO_SLEEP | CD_INFRA)) != 0 {
        roff.print(".");
    }

    memory_awareness(&mut roff, creature, &memory);

    memory_loot_carried(&mut roff, creature.movement, move_val);

    memory_attack_number_and_damage(&mut roff, &memory, creature);

    // Always know the win creature.
    if (creature.movement & CM_WIN) != 0 {
        roff.print(" Killing one of these wins the game!");
    }

    roff.print("\n");
    put_string_clear_to_eol("--pause--", Coord::new(roff.print_line, 0));

    if let Some(saved) = saved_memory {
        creature_recall()[idx] = saved;
    }

    get_key_input()
}

// Allow access to monster memory. -CJS-
pub fn recall_monster_attributes(command: char) {
    let mut n = 0;

    for i in (0..MON_MAX_CREATURES).rev() {
        if CREATURES_LIST[i].sprite as char == command && memory_monster_known(&creature_recall()[i]) {
            if n == 0 {
                let confirmed = get_input_confirmation_with_abort(40, "You recall those details?");
                if confirmed != 1 {
                    break;
                }

                erase_line(Coord::new(0, 40));
                terminal_save_screen();
            }
            n += 1;

            let query = memory_recall(i as i32);
            terminal_restore_screen();
            if query == ESCAPE {
                break;
            }
        }
    }
}
