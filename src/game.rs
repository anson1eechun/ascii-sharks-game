//! The rules: steering, dashes, growth, rank, and the status line.

use crate::grid::{Grid, str_width};
use crate::input::Arrow;
use crate::math::{PI, clamp};
use crate::ui::{DIM, GOLD, INK, PANEL};
use crate::world::{Course, Shark, SharkKind, World};

/// The player's size at the start and at most, as fractions of the biggest
/// shark the sea starts with.
pub const START_SIZE: f64 = 0.5;
const FULL_SIZE: f64 = 1.3;
/// Meals that take the shark 1 − 1/e of the way from start to full size.
const APPETITE: f64 = 70.0;
const DASH_SECONDS: f64 = 0.6;
const DASH_EVERY: f64 = 2.0;
const HINT_SECONDS: f64 = 12.0;
const BANNER_SECONDS: f64 = 5.0;
const GROW_RATE: f64 = 1.5;

const BAR: usize = 10;

pub struct Game {
    kind: SharkKind,
    zh: bool,
    start: Option<f64>,
    ready_at: f64,
    biggest_at: Option<f64>,
}

fn size_for(eaten: u32) -> f64 {
    START_SIZE + (FULL_SIZE - START_SIZE) * (1.0 - (-f64::from(eaten) / APPETITE).exp())
}

fn player(world: &World) -> Option<&Shark> {
    world.sharks.iter().find(|s| s.player)
}

impl Game {
    pub fn new(kind: SharkKind, zh: bool) -> Self {
        Game {
            kind,
            zh,
            start: None,
            ready_at: 0.0,
            biggest_at: None,
        }
    }

    fn t<'a>(&self, en: &'a str, zh: &'a str) -> &'a str {
        if self.zh { zh } else { en }
    }

    pub fn steer(world: &mut World, arrow: Arrow) {
        world.helm.course = Course::Heading(match arrow {
            Arrow::Right => 0.0,
            Arrow::Down => PI / 2.0,
            Arrow::Left => PI,
            Arrow::Up => -PI / 2.0,
        });
    }

    pub fn aim(world: &mut World, x: f64, y: f64) {
        world.helm.course = Course::Toward(x, y);
    }

    pub fn dash(&mut self, world: &mut World) {
        if world.time >= self.ready_at {
            world.helm.dash_until = world.time + DASH_SECONDS;
            self.ready_at = world.time + DASH_EVERY;
        }
    }

    /// Grows the shark toward the size its meals have earned.
    pub fn update(&mut self, world: &mut World, dt: f64) {
        let want = size_for(world.helm.eaten) / START_SIZE;
        let Some(sh) = world.sharks.iter_mut().find(|s| s.player) else {
            return;
        };
        let start = *self.start.get_or_insert(sh.length);
        sh.length += (start * want - sh.length) * (dt * GROW_RATE).min(1.0);
        if self.biggest_at.is_none() && self.rank(world).0 == 1 {
            self.biggest_at = Some(world.time);
        }
    }

    /// The player's place by length, and how many sharks there are.
    fn rank(&self, world: &World) -> (usize, usize) {
        let Some(me) = player(world) else {
            return (0, 0);
        };
        let bigger = world.sharks.iter().filter(|s| s.length > me.length).count();
        (bigger + 1, world.sharks.len())
    }

    fn growth(&self, world: &World) -> f64 {
        match (self.start, player(world)) {
            (Some(start), Some(me)) => clamp(
                (me.length / start * START_SIZE - START_SIZE) / (FULL_SIZE - START_SIZE),
                0.0,
                1.0,
            ),
            _ => 0.0,
        }
    }

    pub fn draw(&self, world: &World, grid: &mut Grid) {
        if grid.rows == 0 || grid.cols == 0 {
            return;
        }
        self.draw_status(world, grid);
        let time = world.time;
        if time < HINT_SECONDS {
            self.tag(world, grid);
        }
        if self.biggest_at.is_some_and(|at| time - at < BANNER_SECONDS) {
            self.banner(
                grid,
                self.t(
                    "You're the biggest shark in the sea!",
                    "你是這片海裡最大的鯊魚！",
                ),
            );
        }
        if time < HINT_SECONDS {
            let hint = self.t(
                "arrows/WASD steer · mouse leads · space/click dash · Esc menu · q quit",
                "方向鍵/WASD 轉向 · 滑鼠帶路 · 空白鍵/點擊 衝刺 · Esc 選單 · q 離開",
            );
            let row = grid.rows - 1;
            grid.put(
                row,
                grid.cols.saturating_sub(str_width(hint)) / 2,
                hint,
                DIM,
                None,
            );
        }
    }

    /// Fish eaten and rank, for the pause menu.
    pub fn summary(&self, world: &World) -> String {
        let (place, of) = self.rank(world);
        let eaten = world.helm.eaten;
        if self.zh {
            format!("吃了 {eaten} 條 · 第 {place} 大／共 {of} 隻")
        } else {
            format!("fish {eaten} · rank {place}/{of}")
        }
    }

    fn draw_status(&self, world: &World, grid: &mut Grid) {
        let filled = (self.growth(world) * BAR as f64).round() as usize;
        let (place, of) = self.rank(world);
        let eaten = world.helm.eaten;
        let ready = world.time >= self.ready_at;
        let parts: [(String, u32); 7] = [
            (format!(" {} ", self.kind.label(self.zh)), INK),
            (format!(" {} [", self.t("size", "體型")), DIM),
            ("=".repeat(filled), GOLD),
            ("-".repeat(BAR - filled), DIM),
            (
                if self.zh {
                    format!("]  吃了 {eaten} 條  第 {place} 大／共 {of} 隻  ")
                } else {
                    format!("]  fish {eaten}  rank {place}/{of}  ")
                },
                DIM,
            ),
            (
                self.t("dash", "衝刺").to_owned(),
                if ready { GOLD } else { DIM },
            ),
            (" ".to_owned(), INK),
        ];
        let mut col = 0;
        for (text, fg) in &parts {
            grid.put(0, col, text, *fg, Some(PANEL));
            col += str_width(text);
        }
    }

    /// Points out which shark is yours.
    fn tag(&self, world: &World, grid: &mut Grid) {
        let Some(me) = player(world) else {
            return;
        };
        let (cw, ch) = (
            world.width / grid.cols as f64,
            world.height / grid.rows as f64,
        );
        let (col, row) = (
            (me.nodes[0] as f64 / cw) as usize,
            (me.nodes[1] as f64 / ch) as usize,
        );
        let text = self.t(" you ", " 你 ");
        let row = if row >= 3 { row - 2 } else { row + 2 };
        let col = col
            .saturating_sub(str_width(text) / 2)
            .min(grid.cols.saturating_sub(str_width(text)));
        grid.put(row, col, text, PANEL, Some(GOLD));
    }

    fn banner(&self, grid: &mut Grid, text: &str) {
        let text = format!("  {text}  ");
        let col = grid.cols.saturating_sub(str_width(&text)) / 2;
        grid.put(grid.rows / 3, col, &text, GOLD, Some(PANEL));
    }
}

/// Plays like a person who goes for the thickest school nearby, looking up
/// four times a second.
#[cfg(test)]
pub struct Bot {
    pub keyboard: bool,
    next_look: f64,
}

#[cfg(test)]
impl Bot {
    pub fn new(keyboard: bool) -> Self {
        Bot {
            keyboard,
            next_look: 0.0,
        }
    }

    pub fn drive(&mut self, world: &mut World, game: &mut Game) {
        if world.time < self.next_look {
            return;
        }
        self.next_look = world.time + 0.25;
        let Some(me) = player(world).copied() else {
            return;
        };
        let (hx, hy) = (me.nodes[0] as f64, me.nodes[1] as f64);
        let alive: Vec<(f64, f64)> = world
            .fish
            .iter()
            .filter(|f| f.alive)
            .map(|f| (f.x, f.y))
            .collect();
        let score = |&(x, y): &(f64, f64)| {
            let crowd = alive
                .iter()
                .filter(|o| (o.0 - x).hypot(o.1 - y) < 90.0)
                .count();
            (x - hx).hypot(y - hy) / (1.0 + 0.25 * crowd as f64)
        };
        let Some((fx, fy)) = alive
            .iter()
            .copied()
            .min_by(|a, b| score(a).total_cmp(&score(b)))
        else {
            return;
        };
        let (dx, dy) = (fx - hx, fy - hy);
        if self.keyboard {
            let arrow = match (dx.abs() > dy.abs(), dx > 0.0, dy > 0.0) {
                (true, true, _) => Arrow::Right,
                (true, false, _) => Arrow::Left,
                (false, _, true) => Arrow::Down,
                (false, _, false) => Arrow::Up,
            };
            Game::steer(world, arrow);
        } else {
            Game::aim(world, fx, fy);
        }
        if dx.hypot(dy) < me.length * 0.8 {
            game.dash(world);
        }
    }
}

#[cfg(test)]
pub fn sea(kind: SharkKind, others: &[(SharkKind, f64)]) -> World {
    use crate::world::SharkSpec;
    let mut specs: Vec<SharkSpec> = others
        .iter()
        .map(|&(kind, size)| SharkSpec {
            kind,
            size,
            player: false,
        })
        .collect();
    specs.push(SharkSpec {
        kind,
        size: START_SIZE,
        player: true,
    });
    World::new(specs, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::STEP;
    use crate::math;

    const OTHERS: [(SharkKind, f64); 3] = [
        (SharkKind::Tiger, 1.0),
        (SharkKind::Mako, 0.8),
        (SharkKind::Nurse, 0.62),
    ];

    fn cell(cols: usize) -> (f64, f64) {
        let cw = (8..=13)
            .rev()
            .map(f64::from)
            .find(|&w| clamp((cols as f64 * w / 124.0 + 0.5).floor(), 8.0, 13.0) == w)
            .unwrap_or(8.0);
        (cw, (cw + 1.0).max((cw * 2.1 + 0.5).floor()))
    }

    /// Fish eaten, and when (if ever) the player became the biggest.
    fn play(cols: usize, rows: usize, keyboard: bool, seed: u32, secs: f64) -> (u32, Option<f64>) {
        math::seed(seed);
        let (cw, ch) = cell(cols);
        let mut world = sea(SharkKind::Classic, &OTHERS);
        world.resize(cols as f64 * cw, rows as f64 * ch, cw, ch);
        let mut game = Game::new(SharkKind::Classic, false);
        let mut bot = Bot::new(keyboard);
        while world.time < secs {
            bot.drive(&mut world, &mut game);
            for _ in 0..2 {
                world.step(STEP);
            }
            world.bites.clear();
            game.update(&mut world, 2.0 * STEP);
        }
        (world.helm.eaten, game.biggest_at)
    }

    #[test]
    fn growth_slows_and_tops_out() {
        assert_eq!(size_for(0), START_SIZE);
        let gains: Vec<f64> = [0, 20, 40, 60, 80]
            .map(size_for)
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();
        assert!(gains.windows(2).all(|w| w[1] < w[0]), "{gains:?}");
        assert!(size_for(10_000) <= FULL_SIZE);
        assert!(size_for(80) > 1.0, "eighty fish make it the biggest");
    }

    #[test]
    fn arrows_steer_and_the_walls_hold() {
        math::seed(3);
        let mut world = sea(SharkKind::Mako, &[]);
        world.resize(1000.0, 600.0, 10.0, 21.0);
        for (arrow, want) in [
            (Arrow::Down, PI / 2.0),
            (Arrow::Left, PI),
            (Arrow::Up, -PI / 2.0),
        ] {
            Game::steer(&mut world, arrow);
            for _ in 0..(10.0 / STEP) as usize {
                world.step(STEP);
                let me = player(&world).unwrap();
                let (x, y) = (me.nodes[0] as f64, me.nodes[1] as f64);
                assert!((0.0..=1000.0).contains(&x) && (0.0..=600.0).contains(&y));
            }
            let heading = player(&world).unwrap().heading;
            assert!(
                crate::math::angle_diff(heading, want).abs() < 1e-6,
                "{arrow:?}: {heading}"
            );
        }
    }

    #[test]
    fn eating_grows_the_shark_and_dashes_wait_their_turn() {
        math::seed(5);
        let mut world = sea(SharkKind::Tiger, &[]);
        world.resize(1200.0, 700.0, 10.0, 21.0);
        let mut game = Game::new(SharkKind::Tiger, false);
        game.update(&mut world, STEP);
        let start = player(&world).unwrap().length;
        for _ in 0..30 {
            let me = player(&world).unwrap();
            let (x, y) = (me.nodes[0] as f64, me.nodes[1] as f64);
            let f = world.fish.iter_mut().find(|f| f.alive).unwrap();
            (f.x, f.y) = (x, y);
            world.step(STEP);
        }
        assert_eq!(world.helm.eaten, 30);
        for _ in 0..300 {
            game.update(&mut world, STEP);
        }
        let grown = player(&world).unwrap().length / start;
        assert!((grown - size_for(30) / START_SIZE).abs() < 0.01, "{grown}");

        game.dash(&mut world);
        let until = world.helm.dash_until;
        assert!(until > world.time);
        game.dash(&mut world);
        assert_eq!(
            world.helm.dash_until, until,
            "no second dash before the first cools"
        );
    }

    #[test]
    fn a_keen_player_becomes_the_biggest_in_a_few_minutes() {
        let (eaten, biggest) = play(100, 30, true, 1, 240.0);
        assert!(
            biggest.is_some_and(|t| t > 30.0),
            "{eaten} fish, biggest at {biggest:?}"
        );
    }

    /// `cargo test --release balance -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn balance() {
        for (cols, rows) in [(90, 28), (140, 40), (200, 55)] {
            for keyboard in [true, false] {
                let runs: Vec<_> = (1..=4)
                    .map(|s| play(cols, rows, keyboard, s, 240.0))
                    .collect();
                println!(
                    "{cols}x{rows} {}: fish in 4 min {:?}, biggest at {:?}",
                    if keyboard { "keys " } else { "mouse" },
                    runs.iter().map(|r| r.0).collect::<Vec<_>>(),
                    runs.iter().map(|r| r.1.map(f64::round)).collect::<Vec<_>>()
                );
            }
        }
    }
}
