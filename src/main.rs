//! Headless smoke run: plays a few seconds of scripted input and reports the
//! result. The graphical frontends live on the renderer branches.

use std::time::Instant;

use frustration_tetris::consts::TIME_STEP;
use frustration_tetris::{Action, Game};

fn main() {
    let mut game = Game::new();
    let seconds: f32 = 30.0;
    let steps = (seconds / TIME_STEP) as usize;
    let started = Instant::now();
    for step in 0..steps {
        if step % 60 == 0 {
            game.execute(Action::TrySpawn);
            game.execute(if (step / 60) % 2 == 0 {
                Action::MoveLeft
            } else {
                Action::MoveRight
            });
            game.execute(Action::RotateCw);
        }
        game.step();
    }
    let elapsed = started.elapsed();
    println!(
        "simulated {seconds}s in {:.1?} ({:.0}x realtime): score {}, {} blocks on the field",
        elapsed,
        seconds / elapsed.as_secs_f32(),
        game.score(),
        game.blocks().count()
    );
}
