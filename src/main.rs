//! OpenGL frontend: glium 0.36 on top of winit 0.30 and glutin 0.32.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use frustration_tetris::{Game, Key, controls};
use glium::Display;
use glium::glutin::config::ConfigTemplateBuilder;
use glium::glutin::context::{ContextApi, ContextAttributesBuilder};
use glium::glutin::display::GetGlDisplay;
use glium::glutin::prelude::*;
use glium::glutin::surface::{SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glium::winit::application::ApplicationHandler;
use glium::winit::dpi::LogicalSize;
use glium::winit::event::{ElementState, KeyEvent, WindowEvent};
use glium::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use glium::winit::keyboard::{KeyCode, PhysicalKey};
use glium::winit::window::{Window, WindowId};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;

mod graphics;
use graphics::Renderer;

/// Frame pacing used only when the driver refuses vsync.
const FALLBACK_FRAME: Duration = Duration::from_micros(16_667);

struct State {
    window: Window,
    display: Display<WindowSurface>,
    renderer: Renderer,
    vsync: bool,
}

struct App {
    state: Option<State>,
    game: Game,
    last_frame: Instant,
    next_frame: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let (window, display, vsync) = create_display(event_loop);
        let renderer = Renderer::new(&display);
        self.state = Some(State {
            window,
            display,
            renderer,
            vsync,
        });
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.display.resize(size.into());
                state.renderer.resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if code == KeyCode::Escape {
                    event_loop.exit();
                    return;
                }
                let Some(key) = map_key(code) else {
                    return;
                };
                let action = match key_state {
                    ElementState::Pressed => controls::on_press(key),
                    ElementState::Released => controls::on_release(key),
                };
                if let Some(action) = action {
                    self.game.execute(action);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.game.advance(now - self.last_frame);
                self.last_frame = now;
                state.renderer.draw(&state.display, &self.game);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &self.state else {
            return;
        };
        if state.vsync {
            // Swapping buffers blocks until the next vertical blank, so a
            // continuous redraw is naturally throttled to the refresh rate.
            state.window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FALLBACK_FRAME;
            state.window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

/// Open a window with a 4x multisampled, vsynced OpenGL context. Returns
/// whether vsync could be enabled.
fn create_display(event_loop: &ActiveEventLoop) -> (Window, Display<WindowSurface>, bool) {
    let attributes = Window::default_attributes()
        .with_title("Frustration Tetris")
        .with_inner_size(LogicalSize::new(600, 800));
    let (window, gl_config) = DisplayBuilder::new()
        .with_window_attributes(Some(attributes))
        .build(
            event_loop,
            ConfigTemplateBuilder::new().with_multisampling(4),
            |mut configs| configs.next().expect("at least one GL config"),
        )
        .expect("create window");
    let window = window.expect("window was requested");
    let window_handle = window.window_handle().expect("window handle");
    let gl_display = gl_config.display();

    let core = ContextAttributesBuilder::new().build(Some(window_handle.into()));
    let gles = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(window_handle.into()));
    // SAFETY: the window outlives the context; both are dropped together.
    let context = unsafe {
        gl_display
            .create_context(&gl_config, &core)
            .or_else(|_| gl_display.create_context(&gl_config, &gles))
            .expect("create GL context")
    };

    let size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window_handle.into(),
        NonZeroU32::new(size.width.max(1)).expect("non-zero"),
        NonZeroU32::new(size.height.max(1)).expect("non-zero"),
    );
    // SAFETY: as above, the window outlives the surface.
    let surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .expect("create GL surface")
    };
    let context = context
        .make_current(&surface)
        .expect("make context current");
    let vsync = surface
        .set_swap_interval(
            &context,
            SwapInterval::Wait(NonZeroU32::new(1).expect("non-zero")),
        )
        .is_ok();
    let display = Display::from_context_surface(context, surface).expect("glium display");
    (window, display, vsync)
}

fn map_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::KeyH => Key::H,
        KeyCode::KeyJ => Key::J,
        KeyCode::KeyK => Key::K,
        KeyCode::KeyL => Key::L,
        KeyCode::Space => Key::Space,
        KeyCode::Backspace => Key::Backspace,
        _ => return None,
    })
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App {
        state: None,
        game: Game::new(),
        last_frame: Instant::now(),
        next_frame: Instant::now(),
    };
    event_loop.run_app(&mut app).expect("event loop");
}
