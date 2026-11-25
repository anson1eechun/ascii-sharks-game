use super::*;
use crate::app::STEP;
use crate::grid::Grid;

/// The first shark is biggest, the rest step down.
pub fn demo_sizes(kinds: &[SharkKind]) -> Vec<SharkSpec> {
    kinds
        .iter()
        .enumerate()
        .map(|(i, &kind)| SharkSpec {
            kind,
            size: (1.0 - i as f64 * 0.1).max(0.3),
            player: false,
        })
        .collect()
}

fn sized_scene(kinds: &[SharkKind], cols: usize, rows: usize) -> (World, Renderer, Grid) {
    let (cw, ch) = (13.0, 27.0);
    let mut world = World::new(demo_sizes(kinds), None);
    let mut renderer = Renderer::new();
    let mut grid = Grid::new();
    renderer.resize(cols, rows, cw, ch);
    world.resize(cols as f64 * cw, rows as f64 * ch, cw, ch);
    grid.resize(cols, rows);
    (world, renderer, grid)
}

#[test]
fn random_kinds_are_distinct_until_they_run_out() {
    for n in 1..=8 {
        let mut kinds = random_kinds(n, None);
        assert_eq!(kinds.len(), n);
        kinds.sort_by_key(|k| k.name());
        kinds.dedup();
        assert_eq!(kinds.len(), n, "repeated a kind among {n}");
    }
    assert_eq!(random_kinds(12, None).len(), 12);
    assert!(!random_kinds(7, Some(SharkKind::Tiger)).contains(&SharkKind::Tiger));
}

#[test]
fn random_sizes_have_one_full_size_shark() {
    for _ in 0..200 {
        let specs = random_sizes(&random_kinds(5, None));
        assert_eq!(specs.iter().filter(|s| s.size == 1.0).count(), 1);
        assert!(specs.iter().all(|s| (MIN_SIZE..=1.0).contains(&s.size)));
    }
    assert!(random_sizes(&[]).is_empty());
}

/// The writer's budget: a seeded 200×55 scene of four classics, in true colour.
#[test]
fn output_stays_within_budget() {
    let (cols, rows) = (200, 55);
    math::seed(42);
    let (mut world, mut renderer, mut grid) = sized_scene(&[SharkKind::Classic; 4], cols, rows);
    let mut screen = screen::Screen::new(screen::ColorMode::TrueColor, false);
    screen.reset(cols * rows);
    let (mut bytes, mut frames) = (0, 0);
    let start = std::time::Instant::now();
    for i in 0..900 {
        world.step(STEP);
        if i % 2 == 1 {
            renderer.render(&mut world, 1.0 / 30.0, &mut grid);
            let n = screen.encode(&grid).len();
            if i > 300 {
                bytes += n;
                frames += 1;
            }
        }
    }
    let per_frame = bytes / frames;
    eprintln!(
        "{per_frame} B/frame, {} µs/frame",
        start.elapsed().as_micros() / 450
    );
    assert!(per_frame < 9000, "{per_frame} B/frame");
}

#[test]
fn resizing_grows_and_shrinks_the_schools() {
    math::seed(5);
    let (mut world, ..) = sized_scene(&[SharkKind::Classic], 100, 30);
    let base = world.fish.len();
    world.resize(240.0 * 13.0, 60.0 * 27.0, 13.0, 27.0);
    assert!(world.fish.len() > base);
    for _ in 0..1200 {
        world.step(STEP);
    }
    assert!(world.fish.iter().filter(|f| f.alive).count() > base);
    world.resize(80.0 * 13.0, 24.0 * 27.0, 13.0, 27.0);
    assert!(world.fish.len() <= base);
    assert!(
        world
            .sharks
            .iter()
            .all(|s| s.target.is_none_or(|t| t < world.fish.len()))
    );
}

#[test]
fn tiny_and_odd_sizes_do_not_panic() {
    for (cols, rows) in [
        (0, 5),
        (5, 0),
        (1, 1),
        (2, 1),
        (1, 2),
        (5, 5),
        (3, 40),
        (200, 2),
    ] {
        for zh in [false, true] {
            math::seed(1);
            let mut world = game::sea(SharkKind::Nurse, &[(SharkKind::Tiger, 1.0)]);
            let mut renderer = Renderer::new();
            let mut grid = Grid::new();
            let (cw, ch) = (13.0, 27.0);
            renderer.resize(cols, rows, cw, ch);
            world.resize(cols as f64 * cw, rows as f64 * ch, cw, ch);
            grid.resize(cols, rows);
            let mut game = Game::new(SharkKind::Nurse, zh);
            game.paused = true;
            Game::aim(&mut world, 6.0, 13.0);
            game.dash(&mut world);
            for _ in 0..3 {
                world.step(STEP);
                game.update(&mut world, STEP);
                renderer.render(&mut world, STEP, &mut grid);
                game.draw(&world, &mut grid);
            }
        }
    }
}
