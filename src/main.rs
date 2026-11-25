//! sharks-game: a shark of your own in a terminal sea, growing on the fish it eats.

mod app;
mod cli;
#[cfg(test)]
mod demo;
mod game;
mod grid;
mod input;
mod math;
mod menu;
mod renderer;
mod screen;
mod setup;
mod species;
mod term;
#[cfg(test)]
mod tests;
mod ui;
mod world;

use std::process::exit;

use app::{App, Scene, Settings};
use cli::{fail, t};
use game::Game;
use menu::Menu;
use renderer::Renderer;
use screen::{ColorMode, Screen};
use setup::Setup;

const FALLBACK_ASPECT: f64 = 2.1;

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
    let ascii = opts.ascii || probe.wide_ambiguous;
    let zh = cli::zh() && !opts.ascii;
    let kind = opts
        .shark
        .unwrap_or_else(|| setup::random_kinds(1, None)[0]);
    let rivals = opts.others.as_ref().map_or(opts.count, Vec::len);
    let setup = Setup {
        others: opts.others.clone(),
        fish: opts.fish,
    };
    let mut menu = Menu::new(kind, rivals, zh, !ascii);
    let player = opts.play.then_some(kind);
    if opts.play {
        menu.close();
    }
    let mut renderer = Renderer::new();
    renderer.calm = opts.calm;
    let scene = Scene {
        world: setup.sea(player, rivals),
        game: player.map(|kind| Game::new(kind, zh)),
        renderer,
        aspect,
    };
    let settings = Settings {
        fps: opts.fps,
        speed: opts.speed,
        zh,
    };
    let screen = Screen::new(color, ascii);
    let result = App::new(scene, setup, menu, settings, screen).run();
    term::leave();
    if let Err(e) = result {
        eprintln!("sharks-game: {e}");
        exit(1);
    }
}
