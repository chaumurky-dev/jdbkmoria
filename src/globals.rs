// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Global state infrastructure for the port.
//
// The original C++ sources keep the entire game state in file-scope globals
// (`game`, `dg`, `py`, `monsters`, ...) that are freely read and written from
// every module. The game is strictly single threaded, so this port mirrors
// those globals with `RacyCell` statics: an `UnsafeCell` wrapper that hands
// out mutable references.
//
// SAFETY INVARIANT: the game must remain single threaded, and callers must
// not hold a returned reference across a call that could touch the same
// global through another path. This matches how the C code behaves.

use std::cell::UnsafeCell;

pub struct RacyCell<T>(UnsafeCell<T>);

// SAFETY: the game is single threaded; see module documentation.
unsafe impl<T> Sync for RacyCell<T> {}

impl<T> RacyCell<T> {
    pub const fn new(value: T) -> Self {
        RacyCell(UnsafeCell::new(value))
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}
