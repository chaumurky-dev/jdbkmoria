// Copyright (c) 1981-86 Robert A. Koeneke
// Copyright (c) 1987-94 James E. Wilson
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Integration test for the jdbkmoria player-centered viewport extension
// (coord_outside_panel_enlarged in ui.rs), exercised without a terminal:
// the panel math only writes dg().panel fields, no curses drawing.
//
// The game's global state is process-wide (see `globals.rs`), so everything
// here runs in a single #[test] fn.

use jdbkmoria::config::options::options;
use jdbkmoria::dungeon::dg;
use jdbkmoria::player::py;
use jdbkmoria::types::Coord;
use jdbkmoria::ui::{coord_outside_panel, set_view_size};

// Screen position of a map coordinate, per panel_put_tile's math.
fn screen_pos_of(coord: Coord) -> (i32, i32) {
    (coord.y - dg().panel.row_prt, coord.x - dg().panel.col_prt)
}

#[test]
fn player_centered_viewport() {
    // Ensure the recompute path doesn't try to end a (nonexistent) run.
    options().find_bound = false;

    // A 40x100 viewport: not the classic 22x66, so coord_outside_panel
    // dispatches to the enlarged, player-centered path.
    set_view_size(40, 100);
    let (view_h, view_w) = (40, 100);
    // The player must land on this screen cell whenever the view (re)centers:
    // window top row -> screen line 1, player row wtop + view_h/2 -> line
    // view_h/2 + 1; column 13 is the sidebar width.
    let center = (view_h / 2 + 1, 13 + view_w / 2);

    // --- Full-size dungeon level, player near the top-right corner (the
    // spot that used to render pinned to the screen corner when the panel
    // was clamped to the map).
    dg().height = 132;
    dg().width = 396;
    py().pos = Coord::new(5, 380);

    assert!(coord_outside_panel(py().pos, false), "fresh level must recompute the panel");
    assert_eq!(screen_pos_of(py().pos), center, "player must be dead center after a recompute");

    // Panel bounds stay the window/map intersection, so consumers indexing
    // dg().floor by them never go out of bounds.
    assert!(dg().panel.top >= 0 && dg().panel.left >= 0);
    assert!(dg().panel.bottom < dg().height as i32 && dg().panel.right < dg().width as i32);
    // The window genuinely overhangs the map here (player near a corner).
    assert!(dg().panel.row_prt < -1, "window must extend above the map top");
    assert_eq!(dg().panel.right, dg().width as i32 - 1, "window must extend past the map's right edge");

    // --- Hysteresis: a single step inside the window must not recompute.
    py().pos = Coord::new(6, 379);
    assert!(!coord_outside_panel(py().pos, false), "one step inside the window must not re-center");

    // --- Walking toward the window's reachable (bottom) edge re-centers,
    // back to dead center. (The window's top edge is off-map here, so
    // walking up can never trigger a re-center — nothing more to show.)
    let window_top = dg().panel.row_prt + 1;
    py().pos = Coord::new(window_top + view_h - 2, 379);
    assert!(coord_outside_panel(py().pos, false), "nearing the window edge must re-center");
    assert_eq!(screen_pos_of(py().pos), center);

    // --- Town-sized level (22x66, smaller than the viewport), simulating
    // generate_cave()'s panel reset: bounds zeroed, row_prt/col_prt stale
    // from the previous level. Must read as invalid and re-center on the
    // player rather than centering the town's content.
    dg().height = 22;
    dg().width = 66;
    dg().panel.top = 0;
    dg().panel.bottom = 0;
    dg().panel.left = 0;
    dg().panel.right = 0;
    py().pos = Coord::new(10, 30);

    assert!(coord_outside_panel(py().pos, false), "freshly reset panel must recompute");
    assert_eq!(screen_pos_of(py().pos), center, "player must be dead center at town entry too");
    // The whole town is inside the panel...
    assert_eq!((dg().panel.top, dg().panel.bottom), (0, 21));
    assert_eq!((dg().panel.left, dg().panel.right), (0, 65));
    // ...and walking around a level fully inside the window never re-centers.
    py().pos = Coord::new(21, 1);
    assert!(!coord_outside_panel(py().pos, false), "town walk must not re-center");
}
