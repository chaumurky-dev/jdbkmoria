// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{SystemTime, UNIX_EPOCH};

// Returns position of first set bit and clears that bit -RAK-
pub fn get_and_clear_first_bit(flag: &mut u32) -> i32 {
    let mut mask: u32 = 0x1;

    for i in 0..32 {
        if (*flag & mask) != 0 {
            *flag &= !mask;
            return i;
        }
        mask <<= 1;
    }

    // no one bits found
    -1
}

// Insert a long number into a string (was `insert_lnum()` function)
pub fn insert_number_into_string(to_string: &mut String, from_string: &str, number: i32, show_sign: bool) {
    if let Some(pos) = to_string.find(from_string) {
        let prefix = &to_string[..pos];
        let suffix = &to_string[pos + from_string.len()..];

        let replaced = if number >= 0 && show_sign {
            format!("{}+{}{}", prefix, number, suffix)
        } else {
            format!("{}{}{}", prefix, number, suffix)
        };

        *to_string = replaced;
    }
}

// Inserts a string into a string
pub fn insert_string_into_string(to_string: &mut String, from_string: &str, str_to_insert: &str) {
    if let Some(pos) = to_string.find(from_string) {
        let prefix = &to_string[..pos];
        let suffix = &to_string[pos + from_string.len()..];

        *to_string = format!("{}{}{}", prefix, str_to_insert, suffix);
    }
}

pub fn is_vowel(ch: char) -> bool {
    matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')
}

pub fn string_to_number(text: &str, number: &mut i32) -> bool {
    match text.trim().parse::<i32>() {
        Ok(value) => {
            *number = value;
            true
        }
        Err(_) => false,
    }
}

pub fn get_current_unix_time() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as u32).unwrap_or(0)
}

// Returns the current date formatted as e.g. "Mon Jan  1"
pub fn human_date_string() -> String {
    // days since Unix epoch, which fell on a Thursday
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);

    // convert to local time using the TZ offset from libc localtime is not
    // available without unsafe/libc, so compute a civil date from UTC. The
    // original used localtime(); the difference is only visible within a few
    // hours of midnight.
    let days = secs.div_euclid(86400);
    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days.rem_euclid(7)) as usize];

    // civil-from-days algorithm (Howard Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month_index = if mp < 10 { mp + 2 } else { mp - 10 } as usize;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][month_index];

    format!("{} {} {:2}", weekday, month, day)
}
