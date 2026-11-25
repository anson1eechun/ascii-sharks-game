//! Drawing with depth: raised slabs that cast shadows, inset wells, an embossed
//! logo with the sharks' ASCII skin, and caustic light moving over all of it.

use crate::grid::{DEFAULT_BG, Grid, rgb, str_width};
use crate::math::{clamp, hash3, to_i32};

pub const INK: u32 = rgb(232, 238, 248);
pub const DIM: u32 = rgb(122, 138, 162);
pub const GOLD: u32 = rgb(255, 196, 80);
pub const PANEL: u32 = rgb(25, 32, 44);

const SLAB_TOP: u32 = rgb(34, 52, 78);
const SLAB_BOTTOM: u32 = rgb(13, 21, 35);
const RIM_LIGHT: u32 = rgb(104, 142, 186);
const RIM_DARK: u32 = rgb(5, 8, 14);
const WELL_TOP: u32 = rgb(2, 6, 12);
const WELL_BOTTOM: u32 = rgb(5, 18, 32);
const PILL: u32 = rgb(48, 76, 112);
const PILL_EDGE: u32 = rgb(30, 48, 72);
const KEYCAP: u32 = rgb(78, 98, 132);
const KEYCAP_SIDE: u32 = rgb(34, 44, 62);
const LIGHT: [f64; 3] = [70.0, 130.0, 160.0];
const LOGO_DEEP: [f64; 3] = [22.0, 50.0, 84.0];
const LOGO_MID: [f64; 3] = [78.0, 128.0, 170.0];
const LOGO_HIGH: [f64; 3] = [220.0, 238.0, 250.0];
const EXTRUDE: u32 = rgb(13, 30, 50);
const SKIN: &[u8] = b":;t%SX8#@";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn contains(&self, col: usize, row: usize) -> bool {
        (self.x..self.x + self.w).contains(&col) && (self.y..self.y + self.h).contains(&row)
    }
}

fn channels(c: u32) -> [f64; 3] {
    [16, 8, 0].map(|s| ((c >> s) & 255) as f64)
}

fn pack([r, g, b]: [f64; 3]) -> u32 {
    let c = |v: f64| clamp(v.round(), 0.0, 255.0) as u32;
    rgb(c(r), c(g), c(b))
}

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

pub fn mix(a: u32, b: u32, t: f64) -> u32 {
    pack(lerp3(channels(a), channels(b), t))
}

fn scale(c: u32, k: f64) -> u32 {
    pack(channels(c).map(|v| v * k))
}

fn lit(c: u32, amount: f64) -> u32 {
    let base = channels(c);
    pack([0, 1, 2].map(|i| base[i] + LIGHT[i] * amount))
}

pub fn at(grid: &Grid, col: usize, row: usize) -> Option<usize> {
    (col < grid.cols && row < grid.rows).then(|| row * grid.cols + col)
}

/// Light through a rippling surface: bright filaments that drift with time.
pub fn caustic(col: f64, row: f64, t: f64) -> f64 {
    let (x, y) = (col * 0.22, row * 0.44);
    let v = (x * 1.7 + t * 0.6).sin()
        + (y * 1.9 - t * 0.45).sin()
        + ((x + y) * 1.3 + t * 0.35).sin()
        + ((x - y) * 1.1 - t * 0.8).sin();
    let n = (v * 0.25 + 1.0) * 0.5;
    n * n * n * n
}

fn grain(col: usize, row: usize) -> f64 {
    hash3(col as i32, row as i32, 17) - 0.5
}

/// Pushes everything back: darker, greyer, and darkest at the edges.
pub fn veil(grid: &mut Grid, keep: f64, grey: f64) {
    let (cols, rows) = (grid.cols as f64, grid.rows as f64);
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let dx = (col as f64 + 0.5) / cols * 2.0 - 1.0;
            let dy = (row as f64 + 0.5) / rows * 2.0 - 1.0;
            let k = keep * (1.0 - 0.45 * (dx * dx + dy * dy) * 0.5);
            let i = row * grid.cols + col;
            let fade = |c: u32| {
                let ch = channels(c);
                let luma = 0.3 * ch[0] + 0.59 * ch[1] + 0.11 * ch[2];
                pack(ch.map(|v| (v + (luma - v) * grey) * k))
            };
            grid.fg[i] = fade(grid.fg[i]);
            if grid.bg[i] != DEFAULT_BG {
                grid.bg[i] = fade(grid.bg[i]);
            }
        }
    }
}

/// Darkens what lies under `r`, as if something floated above it.
fn shade(grid: &mut Grid, r: Rect, k: f64, skip: Rect) {
    for row in r.y..r.y + r.h {
        for col in r.x..r.x + r.w {
            let Some(i) = at(grid, col, row) else {
                continue;
            };
            if skip.contains(col, row) {
                continue;
            }
            grid.fg[i] = scale(grid.fg[i], k);
            if grid.bg[i] != DEFAULT_BG {
                grid.bg[i] = scale(grid.bg[i], k);
            }
        }
    }
}

/// A soft shadow under and to the right of `r`, with a lighter penumbra.
pub fn shadow(grid: &mut Grid, r: Rect) {
    let penumbra = Rect {
        x: r.x + 1,
        y: r.y + 1,
        w: r.w + 3,
        h: r.h + 1,
    };
    let umbra = Rect {
        x: r.x + 2,
        y: r.y + 1,
        w: r.w,
        h: r.h,
    };
    shade(grid, penumbra, 0.6, r);
    shade(grid, umbra, 0.55, r);
}

/// A raised slab: lit from above, with a bright top lip, a dark underside, and
/// caustics playing over its face.
pub fn slab(grid: &mut Grid, r: Rect, t: f64, fine: bool) {
    if r.w < 2 || r.h < 2 {
        return;
    }
    shadow(grid, r);
    let (right, bottom) = (r.x + r.w - 1, r.y + r.h - 1);
    for row in r.y..=bottom {
        let v = (row - r.y) as f64 / (r.h - 1) as f64;
        for col in r.x..=right {
            let Some(i) = at(grid, col, row) else {
                continue;
            };
            let light = caustic(col as f64, row as f64, t) * 0.22 + grain(col, row) * 0.03;
            let face = lit(mix(SLAB_TOP, SLAB_BOTTOM, v), light);
            let face = if col == r.x {
                mix(face, RIM_LIGHT, 0.3)
            } else if col == right {
                mix(face, RIM_DARK, 0.45)
            } else {
                face
            };
            if row == r.y {
                lip(grid, i, face, mix(face, RIM_LIGHT, 0.7), '▄', fine);
            } else if row == bottom {
                lip(grid, i, face, mix(face, RIM_DARK, 0.8), '▀', fine);
            } else {
                grid.block(i, face);
            }
        }
    }
}

/// A half-cell rim where the terminal can draw block halves, else a full one.
fn lip(grid: &mut Grid, i: usize, face: u32, rim: u32, half: char, fine: bool) {
    if fine {
        grid.block(i, rim);
        grid.text(i, half, face, 1.0);
    } else {
        grid.block(i, mix(face, rim, 0.55));
    }
}

/// A recessed well: shadowed along its top and left, shafts of light from above.
pub fn well(grid: &mut Grid, r: Rect, t: f64) {
    for row in r.y..r.y + r.h {
        let v = (row - r.y) as f64 / r.h.max(2).saturating_sub(1) as f64;
        for col in r.x..r.x + r.w {
            let Some(i) = at(grid, col, row) else {
                continue;
            };
            let ray = ((col as f64 * 0.21 + row as f64 * 0.32 - t * 0.25)
                .sin()
                .max(0.0))
            .powi(6);
            let light = ray * (1.0 - v) * 0.18 + caustic(col as f64, row as f64, t * 0.7) * 0.05;
            let mut c = lit(mix(WELL_TOP, WELL_BOTTOM, v), light);
            if row == r.y || col == r.x {
                c = scale(c, 0.45);
            } else if row == r.y + r.h - 1 || col == r.x + r.w - 1 {
                c = mix(c, RIM_LIGHT, 0.12);
            }
            grid.block(i, c);
        }
    }
}

/// The selection: a raised bar with a gold cap, casting a thin shadow below.
pub fn pill(grid: &mut Grid, row: usize, x0: usize, x1: usize, slab_bottom: usize) {
    for col in x0..=x1 {
        let Some(i) = at(grid, col, row) else {
            continue;
        };
        let along = (col - x0) as f64 / (x1 - x0).max(1) as f64;
        let c = if col == x0 {
            GOLD
        } else {
            mix(PILL, PILL_EDGE, along * 0.7)
        };
        grid.block(i, c);
        if row + 1 < slab_bottom
            && let Some(j) = at(grid, col + 1, row + 1)
        {
            grid.bg[j] = scale(grid.bg[j], 0.72);
        }
    }
    if let Some(i) = at(grid, x0, row) {
        grid.text(i, '>', PANEL, 1.0);
    }
}

/// A key drawn as a little raised cap with its side showing; returns the
/// columns it took.
pub fn keycap(grid: &mut Grid, row: usize, col: usize, label: &str) -> usize {
    let text = format!(" {label} ");
    let w = str_width(&text);
    grid.put(row, col, &text, INK, Some(KEYCAP));
    if let Some(i) = at(grid, col + w, row) {
        grid.block(i, KEYCAP_SIDE);
    }
    w + 1
}

pub fn centre(grid: &mut Grid, row: usize, text: &str, fg: u32) {
    let col = grid.cols.saturating_sub(str_width(text)) / 2;
    grid.put(row, col, text, fg, None);
}

const FONT: [[&str; 5]; 6] = [
    [".###", "#...", ".##.", "...#", "###."],
    ["#..#", "#..#", "####", "#..#", "#..#"],
    [".##.", "#..#", "####", "#..#", "#..#"],
    ["###.", "#..#", "###.", "#.#.", "#..#"],
    ["#..#", "#.#.", "##..", "#.#.", "#..#"],
    [".###", "#...", ".##.", "...#", "###."],
];
pub const LOGO_ROWS: usize = 6;

/// SHARKS in embossed letters: an extruded back, a face lit from the top left
/// and textured with the sharks' skin, glinting where the caustics pass.
pub fn logo(grid: &mut Grid, top: usize, t: f64) {
    let wide = grid.cols >= 66;
    let px = if wide { 2 } else { 1 };
    let width = FONT.len() * 5 * px;
    let left = grid.cols.saturating_sub(width) / 2;
    let on = |lx: isize, ly: isize| -> bool {
        if lx < 0 || !(0..5).contains(&ly) {
            return false;
        }
        let (letter, x) = (lx as usize / 5, lx as usize % 5);
        x < 4 && letter < FONT.len() && FONT[letter][ly as usize].as_bytes()[x] == b'#'
    };
    let cells = (0..5usize).flat_map(|ly| (0..FONT.len() * 5).map(move |lx| (lx, ly)));
    for (lx, ly) in cells
        .clone()
        .filter(|&(lx, ly)| on(lx as isize, ly as isize))
    {
        for k in 0..px {
            if let Some(i) = at(grid, left + lx * px + k + px / 2, top + ly + 1) {
                grid.block(i, EXTRUDE);
            }
        }
    }
    let tick = to_i32((t * 2.0).floor());
    for (lx, ly) in cells.filter(|&(lx, ly)| on(lx as isize, ly as isize)) {
        let (sx, sy) = (lx as isize, ly as isize);
        for k in 0..px {
            let (col, row) = (left + lx * px + k, top + ly);
            let Some(i) = at(grid, col, row) else {
                continue;
            };
            let edge_left = k == 0 && !on(sx - 1, sy);
            let edge_right = k == px - 1 && !on(sx + 1, sy);
            let mut s = 0.82 - 0.4 * ly as f64 / 4.0;
            s += 0.24 * (!on(sx, sy - 1) as u8 as f64 + edge_left as u8 as f64 * 0.6);
            s -= 0.24 * (!on(sx, sy + 1) as u8 as f64 + edge_right as u8 as f64 * 0.6);
            s += caustic(col as f64, row as f64, t) * 0.35;
            let s = clamp(s, 0.0, 1.0);
            let face = if s < 0.5 {
                lerp3(LOGO_DEEP, LOGO_MID, s * 2.0)
            } else {
                lerp3(LOGO_MID, LOGO_HIGH, s * 2.0 - 1.0)
            };
            let face = pack(face);
            grid.block(i, face);
            let grain = hash3(col as i32, row as i32, 3);
            if grain < 0.3 {
                continue;
            }
            let idx = (((1.0 - s) * 8.99) as i64 + (grain * 3.0) as i64 - 1)
                .clamp(0, SKIN.len() as i64 - 1);
            let glint = hash3(col as i32, row as i32, tick) < 0.06 && s > 0.75;
            let ink = if glint { 0xffffff } else { scale(face, 0.62) };
            grid.text(i, SKIN[idx as usize] as char, ink, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caustics_stay_in_range_and_move() {
        let samples: Vec<f64> = (0..400)
            .map(|k| caustic((k % 40) as f64, (k / 40) as f64, 1.3))
            .collect();
        assert!(samples.iter().all(|c| (0.0..=1.0).contains(c)));
        assert!(caustic(5.0, 3.0, 0.0) != caustic(5.0, 3.0, 2.0));
    }

    #[test]
    fn a_slab_is_lighter_on_top_and_shadows_what_is_below() {
        let mut g = Grid::new();
        g.resize(30, 12);
        for i in 0..g.bg.len() {
            g.bg[i] = rgb(100, 100, 100);
        }
        let r = Rect {
            x: 4,
            y: 2,
            w: 12,
            h: 6,
        };
        slab(&mut g, r, 0.0, false);
        let luma = |c: u32| channels(c).iter().sum::<f64>();
        let (top, low) = (g.bg[3 * 30 + 8], g.bg[6 * 30 + 8]);
        assert!(luma(top) > luma(low), "lit from above");
        assert!(
            luma(g.bg[8 * 30 + 10]) < luma(rgb(100, 100, 100)),
            "shadow below"
        );
        assert_eq!(g.bg[11 * 30 + 29], rgb(100, 100, 100), "untouched far away");
    }

    #[test]
    fn the_logo_fits_narrow_and_wide_screens() {
        for cols in [30, 66, 120] {
            let mut g = Grid::new();
            g.resize(cols, LOGO_ROWS);
            logo(&mut g, 0, 0.5);
            let painted = g.bg.iter().filter(|&&c| c != DEFAULT_BG).count();
            assert!(painted > 40, "{cols} cols: {painted} cells");
            assert!(SKIN.iter().any(|&b| g.glyph.contains(&(b as char))));
        }
    }
}
