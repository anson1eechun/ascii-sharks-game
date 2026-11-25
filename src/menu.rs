//! The title, pause and help screens, laid over the sea.

use crate::cli::MAX_OTHERS;
use crate::grid::{DEFAULT_BG, Grid, str_width};
use crate::input::{Arrow, Input, MOUSE_BUTTONS, MOUSE_MOTION, MOUSE_WHEEL};
use crate::renderer::Renderer;
use crate::ui::{self, DIM, GOLD, INK, Rect};
use crate::world::{SHARK_NODES, SharkKind, SharkSpec, World};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Title,
    Pause,
    Help { paused: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Start,
    Resume,
    Restart,
    Title,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Item {
    Start,
    Shark,
    Rivals,
    Help,
    Quit,
    Resume,
    Restart,
    Title,
}

const TITLE: [Item; 5] = [
    Item::Start,
    Item::Shark,
    Item::Rivals,
    Item::Help,
    Item::Quit,
];
const PAUSE: [Item; 5] = [
    Item::Resume,
    Item::Restart,
    Item::Help,
    Item::Title,
    Item::Quit,
];
const MENU_W: usize = 40;
const CARD_W: usize = 38;
const GAP: usize = 4;
const EASE: f64 = 16.0;

struct Hit {
    row: usize,
    x0: usize,
    x1: usize,
    item: usize,
    step: i32,
}

pub struct Menu {
    pub page: Option<Page>,
    pub kind: SharkKind,
    pub rivals: usize,
    zh: bool,
    fine: bool,
    row: usize,
    cursor: f64,
    clock: f64,
    hits: Vec<Hit>,
    preview: Preview,
}

impl Menu {
    pub fn new(kind: SharkKind, rivals: usize, zh: bool, fine: bool) -> Self {
        Menu {
            page: Some(Page::Title),
            kind,
            rivals,
            zh,
            fine,
            row: 0,
            cursor: 0.0,
            clock: 0.0,
            hits: Vec::new(),
            preview: Preview::new(),
        }
    }

    fn t<'a>(&self, en: &'a str, zh: &'a str) -> &'a str {
        if self.zh { zh } else { en }
    }

    fn items(&self) -> &'static [Item] {
        match self.page {
            Some(Page::Pause) => &PAUSE,
            _ => &TITLE,
        }
    }

    pub fn open(&mut self, page: Page) {
        if !matches!(page, Page::Help { .. }) {
            self.row = 0;
            self.cursor = 0.0;
        }
        self.page = Some(page);
    }

    pub fn close(&mut self) {
        self.page = None;
    }

    /// The sea stands still behind this page.
    pub fn frozen(&self) -> bool {
        matches!(self.page, Some(Page::Pause | Page::Help { paused: true }))
    }

    /// The selection is still sliding into place.
    pub fn moving(&self) -> bool {
        self.cursor != self.row as f64
    }

    pub fn input(&mut self, input: Input) -> Option<Action> {
        let page = self.page?;
        if let Page::Help { paused } = page {
            let back = match input {
                Input::Mouse { button, press, .. } => {
                    press && button & (MOUSE_MOTION | MOUSE_WHEEL) == 0
                }
                _ => true,
            };
            if back {
                self.page = Some(if paused { Page::Pause } else { Page::Title });
            }
            return None;
        }
        let n = self.items().len();
        match input {
            Input::Arrow(Arrow::Up) | Input::Key(b'w' | b'W' | b'k' | b'K') => {
                self.row = (self.row + n - 1) % n;
            }
            Input::Arrow(Arrow::Down) | Input::Key(b's' | b'S' | b'j' | b'J') => {
                self.row = (self.row + 1) % n;
            }
            Input::Arrow(Arrow::Left) | Input::Key(b'a' | b'A' | b'h' | b'H') => self.adjust(-1),
            Input::Arrow(Arrow::Right) | Input::Key(b'd' | b'D' | b'l' | b'L') => self.adjust(1),
            Input::Key(b'\r' | b'\n' | b' ') => return self.activate(),
            Input::Esc if page == Page::Pause => return Some(Action::Resume),
            Input::Esc => return Some(Action::Quit),
            Input::Key(b'p' | b'P') if page == Page::Pause => return Some(Action::Resume),
            Input::Mouse {
                button,
                col,
                row,
                press,
            } => return self.mouse(button, col as usize, row as usize, press),
            _ => {}
        }
        None
    }

    fn mouse(&mut self, button: u32, col: usize, row: usize, press: bool) -> Option<Action> {
        if button & MOUSE_WHEEL != 0 {
            if press {
                let n = self.items().len();
                self.row = (self.row + if button & 1 == 0 { n - 1 } else { 1 }) % n;
            }
            return None;
        }
        let (col, row) = (col.checked_sub(1)?, row.checked_sub(1)?);
        let hit = self
            .hits
            .iter()
            .find(|h| h.row == row && (h.x0..h.x1).contains(&col))?;
        let (item, step) = (hit.item, hit.step);
        self.row = item;
        if !press || button & (MOUSE_MOTION | MOUSE_BUTTONS) != 0 {
            return None;
        }
        if step != 0 {
            self.adjust(step);
            return None;
        }
        self.activate()
    }

    fn activate(&mut self) -> Option<Action> {
        match self.items()[self.row] {
            Item::Start => Some(Action::Start),
            Item::Shark | Item::Rivals => {
                self.adjust(1);
                None
            }
            Item::Help => {
                self.page = Some(Page::Help {
                    paused: self.page == Some(Page::Pause),
                });
                None
            }
            Item::Quit => Some(Action::Quit),
            Item::Resume => Some(Action::Resume),
            Item::Restart => Some(Action::Restart),
            Item::Title => Some(Action::Title),
        }
    }

    fn adjust(&mut self, step: i32) {
        match self.items()[self.row] {
            Item::Shark => {
                let all = SharkKind::ALL;
                let i = all.iter().position(|&k| k == self.kind).unwrap_or(0) as i32;
                self.kind = all[(i + step).rem_euclid(all.len() as i32) as usize];
            }
            Item::Rivals => {
                self.rivals = (self.rivals as i32 + step).clamp(0, MAX_OTHERS as i32) as usize;
            }
            _ => {}
        }
    }

    fn label(&self, item: Item) -> &'static str {
        match item {
            Item::Start => self.t("Start", "開始遊戲"),
            Item::Shark => self.t("Your shark", "你的鯊魚"),
            Item::Rivals => self.t("Rivals", "對手"),
            Item::Help => self.t("How to play", "操作說明"),
            Item::Quit => self.t("Quit", "離開"),
            Item::Resume => self.t("Resume", "繼續"),
            Item::Restart => self.t("Restart", "重新開始"),
            Item::Title => self.t("Main menu", "回主選單"),
        }
    }

    fn value(&self, item: Item) -> Option<String> {
        match item {
            Item::Shark => Some(self.kind.short(self.zh).to_owned()),
            Item::Rivals if self.rivals == 0 => Some(self.t("none", "無").to_owned()),
            Item::Rivals if self.zh => Some(format!("{} 隻", self.rivals)),
            Item::Rivals => Some(self.rivals.to_string()),
            _ => None,
        }
    }

    pub fn draw(&mut self, grid: &mut Grid, dt: f64, cell: (f64, f64), stats: Option<&str>) {
        let Some(page) = self.page else {
            return;
        };
        self.clock += dt;
        let target = self.row as f64;
        self.cursor += (target - self.cursor) * (dt * EASE).min(1.0);
        if (self.cursor - target).abs() < 0.05 {
            self.cursor = target;
        }
        self.hits.clear();
        if grid.cols == 0 || grid.rows == 0 {
            return;
        }
        match page {
            Page::Title => self.draw_title(grid, cell),
            Page::Pause => self.draw_pause(grid, stats),
            Page::Help { .. } => self.draw_help(grid),
        }
    }

    fn draw_items(&mut self, grid: &mut Grid, slab: Rect, first: usize, spacing: usize) {
        let (end, bottom) = (slab.x + slab.w, (slab.y + slab.h).saturating_sub(1));
        let pill = first + (self.cursor * spacing as f64).round() as usize;
        ui::pill(grid, pill, slab.x + 2, end.saturating_sub(3), bottom);
        for (k, &item) in self.items().iter().enumerate() {
            let row = first + k * spacing;
            let (fg, value_fg) = if k == self.row {
                (INK, GOLD)
            } else {
                (DIM, DIM)
            };
            grid.put(row, slab.x + 5, self.label(item), fg, None);
            if let Some(value) = self.value(item) {
                let text = format!("<  {value}  >");
                let right = end.saturating_sub(4);
                let x0 = right.saturating_sub(str_width(&text));
                grid.put(row, x0, &text, value_fg, None);
                self.hits.push(Hit {
                    row,
                    x0,
                    x1: x0 + 2,
                    item: k,
                    step: -1,
                });
                self.hits.push(Hit {
                    row,
                    x0: right.saturating_sub(2),
                    x1: right,
                    item: k,
                    step: 1,
                });
            }
            self.hits.push(Hit {
                row,
                x0: slab.x + 1,
                x1: end.saturating_sub(1),
                item: k,
                step: 0,
            });
        }
    }

    fn draw_title(&mut self, grid: &mut Grid, cell: (f64, f64)) {
        let (cols, rows) = (grid.cols, grid.rows);
        let n = TITLE.len();
        let card = cols >= MENU_W + GAP + CARD_W + 4;
        let logo = rows >= n + 4 + ui::LOGO_ROWS + 7 && cols >= 32;
        let head = if logo { ui::LOGO_ROWS + 3 } else { 3 };
        let spacing = if rows >= head + 2 * n + 6 { 2 } else { 1 };
        let items_h = (n - 1) * spacing + 1;
        let slab_h = items_h + 4;
        let top = rows.saturating_sub(head + slab_h + 1) / 2;
        if logo {
            ui::logo(grid, top, self.clock);
        } else {
            ui::centre(grid, top, "S H A R K S", GOLD);
        }
        ui::centre(
            grid,
            top + head - 2,
            self.t(
                "eat fish · grow · rule the sea",
                "吃魚 · 長大 · 成為海裡最大的鯊魚",
            ),
            DIM,
        );
        let width = if card {
            MENU_W + GAP + CARD_W
        } else {
            MENU_W.min(cols)
        };
        let x = cols.saturating_sub(width) / 2;
        let y = top + head;
        let menu = Rect {
            x,
            y,
            w: MENU_W.min(cols),
            h: slab_h,
        };
        ui::slab(grid, menu, self.clock, self.fine);
        self.draw_items(grid, menu, y + (slab_h - items_h) / 2, spacing);
        if card {
            self.draw_card(
                grid,
                Rect {
                    x: x + MENU_W + GAP,
                    y,
                    w: CARD_W,
                    h: slab_h,
                },
                cell,
            );
        }
        let keys = if self.zh {
            [("方向鍵", "選擇／調整"), ("Enter", "確認"), ("q", "離開")]
        } else {
            [
                ("arrows", "move / change"),
                ("Enter", "select"),
                ("q", "quit"),
            ]
        };
        footer(grid, &keys);
    }

    /// The chosen shark swimming in a window into the deep, with its name.
    fn draw_card(&mut self, grid: &mut Grid, r: Rect, cell: (f64, f64)) {
        ui::slab(grid, r, self.clock + 3.0, self.fine);
        let well = Rect {
            x: r.x + 2,
            y: r.y + 1,
            w: r.w - 4,
            h: r.h.saturating_sub(4),
        };
        ui::well(grid, well, self.clock);
        let water = Rect {
            x: well.x + 1,
            y: well.y + 1,
            w: well.w - 2,
            h: well.h.saturating_sub(1),
        };
        self.preview.draw(grid, water, self.kind, self.clock, cell);
        let inner = (r.x + 2, r.w - 4);
        put_centred(grid, r.y + r.h - 3, inner, self.kind.label(self.zh), GOLD);
        put_centred(grid, r.y + r.h - 2, inner, self.kind.blurb(self.zh), DIM);
    }

    fn draw_pause(&mut self, grid: &mut Grid, stats: Option<&str>) {
        let spacing = if grid.rows >= 22 { 2 } else { 1 };
        let h = (PAUSE.len() - 1) * spacing + 7;
        let w = 36.min(grid.cols);
        let r = Rect {
            x: grid.cols.saturating_sub(w) / 2,
            y: grid.rows.saturating_sub(h + 1) / 2,
            w,
            h,
        };
        ui::slab(grid, r, self.clock, self.fine);
        let inner = (r.x + 2, r.w.saturating_sub(4));
        put_centred(grid, r.y + 1, inner, self.t("Paused", "暫停"), GOLD);
        if let Some(stats) = stats {
            put_centred(grid, r.y + 2, inner, stats, DIM);
        }
        self.draw_items(grid, r, r.y + 4, spacing);
        let keys = if self.zh {
            [("Enter", "確認"), ("Esc", "繼續"), ("q", "離開")]
        } else {
            [("Enter", "select"), ("Esc", "resume"), ("q", "quit")]
        };
        footer(grid, &keys);
    }

    fn draw_help(&mut self, grid: &mut Grid) {
        let rows: [(&[&str], &str); 5] = if self.zh {
            [
                (&["方向鍵", "WASD", "hjkl"], "往那個方向游"),
                (&["滑鼠"], "你的鯊魚跟著游標"),
                (&["空白鍵", "點擊"], "衝刺，每 2 秒一次"),
                (&["Esc", "p"], "暫停選單"),
                (&["q"], "離開"),
            ]
        } else {
            [
                (&["arrows", "WASD", "hjkl"], "swim that way"),
                (&["mouse"], "your shark follows it"),
                (&["space", "click"], "dash, every 2 s"),
                (&["Esc", "p"], "pause menu"),
                (&["q"], "quit"),
            ]
        };
        let notes = if self.zh {
            [
                "吃魚長大，超過所有鯊魚就是海裡最大的鯊魚。",
                "魚會逃：從側面切入，或在最後一刻衝刺。",
            ]
        } else {
            [
                "Eat fish to grow; outgrow every other shark.",
                "Fish flee: cut in from the side, or dash late.",
            ]
        };
        let w = 56.min(grid.cols);
        let h = rows.len() + notes.len() + 7;
        let r = Rect {
            x: grid.cols.saturating_sub(w) / 2,
            y: grid.rows.saturating_sub(h + 1) / 2,
            w,
            h,
        };
        ui::slab(grid, r, self.clock, self.fine);
        let inner = (r.x + 2, r.w.saturating_sub(4));
        put_centred(
            grid,
            r.y + 1,
            inner,
            self.t("How to play", "操作說明"),
            GOLD,
        );
        let keys_w = |keys: &[&str]| keys.iter().map(|k| str_width(k) + 4).sum::<usize>();
        let desc = r.x + 6 + rows.iter().map(|(k, _)| keys_w(k)).max().unwrap_or(0);
        for (k, (keys, what)) in rows.iter().enumerate() {
            let row = r.y + 3 + k;
            let mut col = r.x + 4;
            for key in *keys {
                col += ui::keycap(grid, row, col, key) + 1;
            }
            grid.put(row, desc, what, INK, None);
        }
        for (k, note) in notes.iter().enumerate() {
            put_centred(grid, r.y + 4 + rows.len() + k, inner, note, DIM);
        }
        footer(
            grid,
            &[(self.t("any key", "任意鍵"), self.t("go back", "返回"))],
        );
    }
}

fn put_centred(grid: &mut Grid, row: usize, (x, w): (usize, usize), text: &str, fg: u32) {
    grid.put(
        row,
        x + w.saturating_sub(str_width(text)) / 2,
        text,
        fg,
        None,
    );
}

fn footer(grid: &mut Grid, keys: &[(&str, &str)]) {
    let width: usize = keys
        .iter()
        .map(|(k, d)| str_width(k) + 4 + str_width(d) + 3)
        .sum();
    let row = grid.rows - 1;
    let mut col = grid.cols.saturating_sub(width) / 2;
    for (key, what) in keys {
        col += ui::keycap(grid, row, col, key) + 1;
        grid.put(row, col, what, DIM, None);
        col += str_width(what) + 3;
    }
}

/// One shark posed in a small sea of its own, drawn by the game's renderer.
struct Preview {
    world: Option<World>,
    renderer: Renderer,
    grid: Grid,
    kind: Option<SharkKind>,
    size: (usize, usize),
}

impl Preview {
    fn new() -> Self {
        Preview {
            world: None,
            renderer: Renderer::new(),
            grid: Grid::new(),
            kind: None,
            size: (0, 0),
        }
    }

    fn draw(&mut self, grid: &mut Grid, r: Rect, kind: SharkKind, t: f64, (cw, ch): (f64, f64)) {
        if r.w < 4 || r.h < 2 {
            return;
        }
        if self.kind != Some(kind) || self.size != (r.w, r.h) {
            let spec = SharkSpec {
                kind,
                size: 1.0,
                player: false,
            };
            let mut world = World::new(vec![spec], Some(0));
            world.resize(r.w as f64 * cw, r.h as f64 * ch, cw, ch);
            self.renderer.resize(r.w, r.h, cw, ch);
            self.grid.resize(r.w, r.h);
            (self.world, self.kind, self.size) = (Some(world), Some(kind), (r.w, r.h));
        }
        let Some(world) = &mut self.world else {
            return;
        };
        let (w, h) = (world.width, world.height);
        let sh = &mut world.sharks[0];
        let l = (w * 0.9).min(h * 1.9);
        sh.length = l;
        sh.heading = 0.1 * (t * 0.5).sin();
        sh.speed = 44.0;
        sh.phase = t * 3.0;
        sh.turn = 0.0;
        let (dx, dy) = (sh.heading.cos(), sh.heading.sin());
        let (cx, cy) = (w * 0.5, h * 0.5 + (t * 0.8).sin() * ch * 0.25);
        let seg = l / (SHARK_NODES - 1) as f64;
        for k in 0..SHARK_NODES {
            let along = l * 0.5 - seg * k as f64;
            sh.nodes[k * 2] = (cx + dx * along) as f32;
            sh.nodes[k * 2 + 1] = (cy + dy * along) as f32;
        }
        world.time = t;
        self.renderer.render(world, 1.0 / 30.0, &mut self.grid);
        for row in 0..r.h {
            for col in 0..r.w {
                let Some(j) = ui::at(grid, r.x + col, r.y + row) else {
                    continue;
                };
                let i = row * r.w + col;
                if self.grid.bg[i] != DEFAULT_BG {
                    grid.block(j, self.grid.bg[i]);
                }
                if self.grid.glyph[i] != ' ' {
                    grid.text(j, self.grid.glyph[i], self.grid.fg[i], 1.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Menu {
        Menu::new(SharkKind::Tiger, 3, false, true)
    }

    fn press(m: &mut Menu, keys: &[Input]) -> Option<Action> {
        keys.iter().fold(None, |_, &k| m.input(k))
    }

    #[test]
    fn moving_wraps_and_values_change() {
        let mut m = menu();
        assert_eq!(press(&mut m, &[Input::Arrow(Arrow::Up)]), None);
        assert_eq!(m.items()[m.row], Item::Quit, "up from the top wraps");
        press(&mut m, &[Input::Key(b'j'), Input::Key(b'j')]);
        assert_eq!(m.items()[m.row], Item::Shark);
        press(&mut m, &[Input::Arrow(Arrow::Left)]);
        assert_eq!(m.kind, SharkKind::GreatHammer);
        press(&mut m, &[Input::Key(b'\r'), Input::Key(b'\r')]);
        assert_eq!(m.kind, SharkKind::Mako, "enter steps a value along");
        press(&mut m, &[Input::Key(b's')]);
        for _ in 0..20 {
            press(&mut m, &[Input::Arrow(Arrow::Right)]);
        }
        assert_eq!(m.rivals, MAX_OTHERS);
        for _ in 0..20 {
            press(&mut m, &[Input::Key(b'a')]);
        }
        assert_eq!(m.rivals, 0);
    }

    #[test]
    fn pages_lead_where_they_say() {
        let mut m = menu();
        assert_eq!(press(&mut m, &[Input::Key(b'\r')]), Some(Action::Start));
        press(&mut m, &[Input::Arrow(Arrow::Up), Input::Arrow(Arrow::Up)]);
        assert_eq!(press(&mut m, &[Input::Key(b' ')]), None);
        assert_eq!(m.page, Some(Page::Help { paused: false }));
        press(&mut m, &[Input::Key(b'x')]);
        assert_eq!(m.page, Some(Page::Title));
        assert_eq!(m.items()[m.row], Item::Help, "back where it was");
        assert_eq!(press(&mut m, &[Input::Esc]), Some(Action::Quit));

        m.open(Page::Pause);
        assert!(m.frozen());
        assert_eq!(press(&mut m, &[Input::Esc]), Some(Action::Resume));
        assert_eq!(press(&mut m, &[Input::Key(b'p')]), Some(Action::Resume));
        let pick = |m: &mut Menu, n| {
            m.open(Page::Pause);
            for _ in 0..n {
                m.input(Input::Arrow(Arrow::Down));
            }
            m.input(Input::Key(b'\r'))
        };
        assert_eq!(pick(&mut m, 1), Some(Action::Restart));
        assert_eq!(pick(&mut m, 2), None);
        assert_eq!(m.page, Some(Page::Help { paused: true }));
        assert!(m.frozen());
        assert_eq!(pick(&mut m, 3), Some(Action::Title));
        assert_eq!(pick(&mut m, 4), Some(Action::Quit));
    }

    #[test]
    fn the_mouse_points_and_clicks() {
        let mut m = menu();
        let mut g = Grid::new();
        g.resize(120, 40);
        m.draw(&mut g, 0.0, (13.0, 27.0), None);
        let hit = |m: &Menu, item: usize, step: i32| {
            let h = m
                .hits
                .iter()
                .find(|h| h.item == item && h.step == step)
                .unwrap();
            (h.x0 as u32 + 1, h.row as u32 + 1)
        };
        let (col, row) = hit(&m, 1, 1);
        let click = |col, row| Input::Mouse {
            button: 0,
            col,
            row,
            press: true,
        };
        assert_eq!(m.input(click(col, row)), None);
        assert_eq!(m.kind, SharkKind::Mako, "the > steps the shark along");
        let (col, row) = hit(&m, 4, 0);
        m.input(Input::Mouse {
            button: 35,
            col,
            row,
            press: true,
        });
        assert_eq!(m.items()[m.row], Item::Quit, "hovering picks");
        assert_eq!(m.input(click(col, row)), Some(Action::Quit));
    }

    #[test]
    fn the_selection_slides_then_settles() {
        let mut m = menu();
        let mut g = Grid::new();
        g.resize(100, 30);
        m.input(Input::Arrow(Arrow::Down));
        assert!(m.moving());
        for _ in 0..30 {
            m.draw(&mut g, 1.0 / 30.0, (13.0, 27.0), None);
        }
        assert!(!m.moving());
    }

    #[test]
    fn every_page_draws_at_any_size() {
        for (cols, rows) in [
            (0, 0),
            (1, 1),
            (10, 4),
            (40, 12),
            (80, 24),
            (100, 30),
            (200, 55),
        ] {
            for page in [Page::Title, Page::Pause, Page::Help { paused: true }] {
                for zh in [false, true] {
                    let mut m = Menu::new(SharkKind::Zebra, 2, zh, zh);
                    m.open(page);
                    let mut g = Grid::new();
                    g.resize(cols, rows);
                    m.draw(&mut g, 0.1, (13.0, 27.0), Some("fish 3 · rank 2/3"));
                }
            }
        }
    }
}
