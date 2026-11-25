use std::io;
use std::time::{Duration, Instant};

use crate::game::Game;
use crate::grid::Grid;
use crate::input::{self, Arrow, Input, MOUSE_BUTTONS, MOUSE_MOTION, MOUSE_WHEEL};
use crate::math::{clamp, js_round};
use crate::renderer::Renderer;
use crate::screen::Screen;
use crate::term;
use crate::world::World;

pub const STEP: f64 = 1.0 / 60.0;
const MAX_STEPS: u32 = 4;
/// How long a lone Esc waits to rule out being the start of a sequence.
const ESC_WAIT: Duration = Duration::from_millis(25);
const CTRL_C: u8 = 0x03;
const CTRL_Z: u8 = 0x1a;

/// A fixed timestep: how many steps `dt` more seconds take. At most MAX_STEPS,
/// so a stall doesn't snowball, unless slow frames or a fast --speed need more
/// every frame.
fn steps_for(acc: &mut f64, dt: f64) -> u32 {
    *acc += dt;
    let max = MAX_STEPS.max((dt / STEP).ceil() as u32);
    let mut steps = 0;
    while *acc >= STEP && steps < max {
        *acc -= STEP;
        steps += 1;
    }
    if steps == max {
        *acc = 0.0;
    }
    steps
}

pub struct Scene {
    pub world: World,
    pub game: Game,
    pub renderer: Renderer,
    pub aspect: f64,
}

impl Scene {
    /// A cell is width / 124 world units wide, clamped to 8–13; returns its
    /// width and height.
    fn cell_size(&self, cols: usize) -> (f64, f64) {
        let cw = (8..=13)
            .rev()
            .map(f64::from)
            .find(|&w| clamp(js_round(cols as f64 * w / 124.0), 8.0, 13.0) == w)
            .unwrap_or(8.0);
        (cw, (cw + 1.0).max(js_round(cw * self.aspect)))
    }

    fn resize(&mut self, cols: usize, rows: usize, (cw, ch): (f64, f64)) {
        self.renderer.resize(cols, rows, cw, ch);
        self.world
            .resize(cols as f64 * cw, rows as f64 * ch, cw, ch);
    }

    fn render(&mut self, dt: f64, grid: &mut Grid) {
        self.renderer.render(&mut self.world, dt, grid);
        self.game.draw(&self.world, grid);
    }
}

pub struct Settings {
    pub fps: f64,
    pub speed: f64,
}

pub struct App {
    scene: Scene,
    speed: f64,
    grid: Grid,
    screen: Screen,
    cell: Option<(f64, f64)>,
    paused: bool,
    frame: Duration,
    last: Instant,
    next: Instant,
    acc: f64,
    pending: Vec<u8>,
    inputs: Vec<Input>,
}

impl App {
    pub fn new(scene: Scene, settings: Settings, screen: Screen) -> Self {
        let now = Instant::now();
        App {
            scene,
            speed: settings.speed,
            frame: Duration::from_secs_f64(1.0 / settings.fps),
            grid: Grid::new(),
            screen,
            cell: None,
            paused: false,
            last: now,
            next: now,
            acc: 0.0,
            pending: Vec::new(),
            inputs: Vec::new(),
        }
    }

    fn layout(&mut self) -> io::Result<()> {
        let (cols, rows) = term::size();
        let cell = *self.cell.get_or_insert_with(|| self.scene.cell_size(cols));
        self.scene.resize(cols, rows, cell);
        self.grid.resize(cols, rows);
        self.screen.reset(cols * rows);
        if self.paused { self.draw(0.0) } else { Ok(()) }
    }

    fn draw(&mut self, dt: f64) -> io::Result<()> {
        self.scene.render(dt, &mut self.grid);
        self.screen.flush(&self.grid)
    }

    fn tick(&mut self, now: Instant) -> io::Result<()> {
        let elapsed = (now - self.last)
            .as_secs_f64()
            .min(self.frame.as_secs_f64().max(0.1));
        let dt = elapsed * self.speed;
        self.last = now;
        for _ in 0..steps_for(&mut self.acc, dt) {
            self.scene.world.step(STEP);
        }
        self.scene.game.update(&mut self.scene.world, dt);
        self.draw(dt)
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.show()?;
        loop {
            let signals = term::take_signals();
            if signals.terminate {
                return Ok(());
            }
            if signals.suspend {
                term::suspend()?;
                self.show()?;
            } else if signals.continued {
                term::enter()?;
                self.show()?;
            } else if signals.resized {
                self.layout()?;
            }

            let timeout = if self.paused {
                None
            } else {
                let now = Instant::now();
                if now >= self.next {
                    self.tick(now)?;
                    self.next += self.frame;
                    if self.next < now {
                        self.next = now + self.frame;
                    }
                }
                Some(self.next.saturating_duration_since(Instant::now()))
            };
            if term::wait_input(timeout) && self.read_input()? {
                return Ok(());
            }
        }
    }

    fn show(&mut self) -> io::Result<()> {
        term::write_all(term::MOUSE_ON)?;
        self.layout()
    }

    /// Reads what's waiting on stdin and acts on it; true means quit.
    fn read_input(&mut self) -> io::Result<bool> {
        let mut buf = [0u8; 4096];
        self.inputs.clear();
        loop {
            match term::read_input(&mut buf) {
                Ok(0) => return Ok(true),
                Ok(n) => self.pending.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => return Err(e),
            }
            let used = input::parse(&self.pending, &mut self.inputs);
            self.pending.drain(..used);
            if self.pending == [0x1b] && !term::wait_input(Some(ESC_WAIT)) {
                self.pending.clear();
                self.inputs.push(Input::Esc);
            }
            if self.pending.len() > 64 {
                self.pending.clear();
            }
            if self.pending.is_empty() || !term::wait_input(Some(Duration::ZERO)) {
                break;
            }
        }
        for i in 0..self.inputs.len() {
            if self.apply(i)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn apply(&mut self, i: usize) -> io::Result<bool> {
        let arrow = match self.inputs[i] {
            Input::Key(b'q' | b'Q' | CTRL_C) | Input::Esc => return Ok(true),
            Input::Key(CTRL_Z) => {
                term::suspend()?;
                self.show()?;
                return Ok(false);
            }
            Input::Key(b'p' | b'P') => {
                self.toggle_pause()?;
                return Ok(false);
            }
            Input::Key(b' ') => {
                self.dash();
                return Ok(false);
            }
            Input::Mouse {
                button,
                col,
                row,
                press,
            } => {
                self.mouse(button, col, row, press);
                return Ok(false);
            }
            Input::Arrow(a) => a,
            Input::Key(b'w' | b'W' | b'k' | b'K') => Arrow::Up,
            Input::Key(b's' | b'S' | b'j' | b'J') => Arrow::Down,
            Input::Key(b'a' | b'A' | b'h' | b'H') => Arrow::Left,
            Input::Key(b'd' | b'D' | b'l' | b'L') => Arrow::Right,
            _ => return Ok(false),
        };
        if !self.paused {
            Game::steer(&mut self.scene.world, arrow);
        }
        Ok(false)
    }

    fn dash(&mut self) {
        if !self.paused {
            self.scene.game.dash(&mut self.scene.world);
        }
    }

    fn toggle_pause(&mut self) -> io::Result<()> {
        self.paused = !self.paused;
        self.scene.game.paused = self.paused;
        if self.paused {
            return self.draw(0.0);
        }
        self.last = Instant::now();
        self.next = self.last;
        Ok(())
    }

    fn mouse(&mut self, button: u32, col: u32, row: u32, press: bool) {
        let Some((cw, ch)) = self.cell else { return };
        if self.paused || button & MOUSE_WHEEL != 0 {
            return;
        }
        let (x, y) = ((col as f64 - 0.5) * cw, (row as f64 - 0.5) * ch);
        Game::aim(&mut self.scene.world, x, y);
        if press && button & (MOUSE_MOTION | MOUSE_BUTTONS) == 0 {
            self.dash();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sea_keeps_time_at_any_frame_rate_and_speed() {
        for (fps, speed) in [
            (30.0, 1.0),
            (10.0, 1.0),
            (5.0, 1.0),
            (30.0, 3.0),
            (30.0, 5.0),
            (120.0, 0.5),
        ] {
            let (mut acc, mut steps) = (0.0, 0);
            for _ in 0..(fps as usize * 10) {
                steps += steps_for(&mut acc, speed / fps);
            }
            let rate = steps as f64 * STEP / 10.0;
            assert!(
                (rate - speed).abs() < 0.05 * speed,
                "fps {fps} speed {speed}: {rate:.2}×"
            );
        }
    }

    #[test]
    fn a_stall_does_not_snowball() {
        let mut acc = 5.0;
        assert_eq!(
            steps_for(&mut acc, 0.1),
            6,
            "a tenth of a second is six steps"
        );
        assert_eq!(acc, 0.0);
    }
}
