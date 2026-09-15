//! wgpu frontend: winit 0.30 window, wgpu 30 rendering (Vulkan, Metal,
//! DX12 or OpenGL, whichever the platform provides).

use std::sync::Arc;
use std::time::Instant;

use frustration_tetris::render::Painter;
use frustration_tetris::{Game, Key, controls};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// A window's swapchain plus the painter that fills it.
struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    painter: Painter,
}

impl Renderer {
    fn new(event_loop: &ActiveEventLoop, window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(event_loop.owned_display_handle()),
        ));
        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("no compatible GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("frustration-tetris"),
            ..Default::default()
        }))
        .expect("request device");
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface is supported by the adapter");
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);
        let painter = Painter::new(
            device.clone(),
            queue.clone(),
            config.format,
            config.width,
            config.height,
        );
        Renderer {
            window,
            surface,
            device,
            queue,
            config,
            painter,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.painter.resize(width, height);
    }

    fn render(&mut self, game: &Game) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.painter.paint(game, &target);
        self.queue.present(frame);
    }
}

struct App {
    renderer: Option<Renderer>,
    game: Game,
    last_frame: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Frustration Tetris")
            .with_inner_size(LogicalSize::new(600, 800));
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));
        self.renderer = Some(Renderer::new(event_loop, window));
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
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
                renderer.render(&self.game);
                // Presenting with vsync blocks until the next vertical blank,
                // so a continuous redraw runs at the display refresh rate.
                renderer.window.request_redraw();
            }
            _ => {}
        }
    }
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
        renderer: None,
        game: Game::new(),
        last_frame: Instant::now(),
    };
    event_loop.run_app(&mut app).expect("event loop");
}
