//! sharks-game: a shark of your own in a terminal sea, growing on the fish it eats.

mod app;
mod cli;
#[cfg(test)]
mod demo;
mod game;
mod grid;
mod input;
mod math;
mod renderer;
mod screen;
mod species;
mod term;
#[cfg(test)]
mod tests;
mod world;

use std::process::exit;

use app::{App, Scene, Settings};
use cli::{Options, fail, t};
use game::Game;
use renderer::Renderer;
use screen::{ColorMode, Screen};
use world::{SharkKind, SharkSpec, World};

const FALLBACK_ASPECT: f64 = 2.1;
const MIN_SIZE: f64 = 0.55;

/// All different, and none of them `except`, while there are enough kinds to go round.
fn random_kinds(n: usize, except: Option<SharkKind>) -> Vec<SharkKind> {
    let mut kinds = SharkKind::ALL.to_vec();
    kinds.retain(|&k| Some(k) != except);
    for i in (1..kinds.len()).rev() {
        kinds.swap(i, (math::random() * (i + 1) as f64) as usize);
    }
    (0..n)
        .map(|i| {
            kinds
                .get(i)
                .copied()
                .unwrap_or_else(|| kinds[(math::random() * kinds.len() as f64) as usize])
        })
        .collect()
}

/// One shark at full size, the rest smaller. Size doubles as depth: smaller
/// sharks swim deeper and dimmer.
fn random_sizes(kinds: &[SharkKind]) -> Vec<SharkSpec> {
    let biggest = (math::random() * kinds.len() as f64) as usize;
    kinds
        .iter()
        .enumerate()
        .map(|(i, &kind)| SharkSpec {
            kind,
            size: if i == biggest {
                1.0
            } else {
                math::rand(MIN_SIZE, 1.0)
            },
            player: false,
        })
        .collect()
}

fn scene(opts: &Options, aspect: f64) -> Scene {
    let zh = cli::zh() && !opts.ascii;
    let player = opts.shark.unwrap_or_else(|| random_kinds(1, None)[0]);
    let others = opts
        .others
        .clone()
        .unwrap_or_else(|| random_kinds(opts.count, Some(player)));
    let mut sharks = random_sizes(&others);
    sharks.push(SharkSpec {
        kind: player,
        size: game::START_SIZE,
        player: true,
    });
    let mut renderer = Renderer::new();
    renderer.calm = opts.calm;
    Scene {
        world: World::new(sharks, opts.fish),
        game: Game::new(player, zh),
        renderer,
        aspect,
    }
}

fn main() {
    let opts = cli::parse();
    let color = opts.color.unwrap_or_else(ColorMode::detect);
    if let Some(seed) = opts.seed {
        math::seed(seed);
    }
    if !term::is_tty() {
        fail(t("needs to run in a terminal", "需要在終端機裡執行"));
    }

    term::install_signals();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        term::leave();
        default_hook(info);
    }));
    if let Err(e) = term::enter() {
        fail(&format!(
            "{}: {e}",
            t("can't set up the terminal", "無法設定終端機")
        ));
    }
    let probe = term::probe();
    let aspect = opts.aspect.or(probe.aspect).unwrap_or(FALLBACK_ASPECT);
    let settings = Settings {
        fps: opts.fps,
        speed: opts.speed,
    };
    let screen = Screen::new(color, opts.ascii || probe.wide_ambiguous);
    let result = App::new(scene(&opts, aspect), settings, screen).run();
    term::leave();
    if let Err(e) = result {
        eprintln!("sharks-game: {e}");
        exit(1);
    }
}
