#![feature(slice_patterns, advanced_slice_patterns)]

extern crate num;
extern crate nalgebra as na;
extern crate ncollide;
extern crate nphysics2d;
extern crate rand;

#[macro_use] extern crate glium;
extern crate glium_text;

use std::time::{Duration, Instant};

use glium::{DisplayBuild, Surface};

mod consts;
mod controls;
mod game;
mod graphics;

use controls::Controls;
use game::*;
use graphics::{show_loading_screen, GraphicsProperties};


fn main() {
    let display = glium::glutin::WindowBuilder::new()
        .with_title("Glium Tetris")
        .with_dimensions(600, 800)
        .with_min_dimensions(600, 800)
        .with_max_dimensions(600, 800)
        .with_multisampling(4)
        .with_vsync()
        .build_glium().unwrap();

    let mut target = display.draw();
    show_loading_screen(&display, &mut target);
    target.finish().unwrap();

    let props = GraphicsProperties::new(&display);

    let mut game = Game::new();
    let mut last_update = Instant::now();

    'mainloop: loop {
        // Handle events
        for event in display.poll_events() {
            use glium::glutin::Event::*;
            use glium::glutin::ElementState::*;
            use glium::glutin::VirtualKeyCode::*;
            match event {
                Closed | KeyboardInput(Pressed, _, Some(Escape)) => break 'mainloop,
                KeyboardInput(Pressed, _, Some(Back)) => game.reset(),

                KeyboardInput(Pressed, _, Some(keycode)) => if let Some(action) = Controls.resolve_press(keycode) {
                    game.execute_action(action);
                },
                KeyboardInput(Released, _, Some(keycode)) => if let Some(action) = Controls.resolve_release(keycode) {
                    game.execute_action(action);
                },

                _ => {},
            }
        }

        // Update the game logic
        game.update();

        // Draw everything
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        game.draw(&display, &mut target, &props);
        target.finish().unwrap();


        // TODO?
        // display.swap_buffers().unwrap();
        // Wait for next loop
        let now = Instant::now();
        let delta = now - last_update;
        if delta < Duration::from_millis(16) {
            ::std::thread::sleep(Duration::from_millis(16) - delta);
        }
        last_update = now;
    }
}
