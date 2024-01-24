use std::{env, sync::Arc};

use glam::Vec2;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::{KeyCode, PhysicalKey},
    platform::{wayland::EventLoopBuilderExtWayland, x11::EventLoopBuilderExtX11},
    window::WindowBuilder,
};

use crate::settings::GlobalSettings;

//

pub mod graphics;
pub mod settings;

//

#[derive(Debug)]
pub struct RuntimeSettings {
    pub f: u32,
}

//

#[tokio::main]
async fn main() {
    const SILENCE_WGPU: &str = "wgpu_core=error,wgpu_hal=error,naga=error,debug";

    let log = env::var("RUST_LOG")
        .map(|old| format!("{old},{SILENCE_WGPU}"))
        .unwrap_or_else(|_| SILENCE_WGPU.to_string());
    env::set_var("RUST_LOG", log);

    /* for (var, val) in env::vars() {
        println!("{var}={val}");
    } */

    tracing_subscriber::fmt::init();

    let settings = GlobalSettings::load();
    settings.autosave();

    tracing::debug!("{:#?}", &*settings);

    // use winit::platform::{wayland::*, x11::*};
    let mut events = EventLoopBuilder::new();
    let events = if settings.window.force_wayland {
        events.with_wayland()
    } else if settings.window.force_x11 {
        events.with_x11()
    } else {
        &mut events
    }
    .build()
    .expect("failed to create the event loop");

    let window = WindowBuilder::new()
        .with_title(settings.window.title.as_ref())
        .with_inner_size(LogicalSize::new(
            settings.window.resolution.0,
            settings.window.resolution.1,
        ))
        .with_transparent(true)
        /* .with_fullscreen(Some(Fullscreen::Exclusive(VideoMode::
        ))) */
        .with_visible(false)
        .build(&events)
        .expect("failed to open a window");

    let window = Arc::new(window);

    let mut graphics = graphics::Graphics::init(&settings, window.clone())
        .await
        .unwrap();

    let mut settings = RuntimeSettings { f: 0 };

    window.set_visible(true);

    events
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);

            // println!("{event:?}");

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    target.exit();
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    physical_key: PhysicalKey::Code(key),
                                    state: ElementState::Pressed,
                                    ..
                                },
                            // KeyboardInput {
                            //     state: ElementState::Pressed,
                            //     virtual_keycode: Some(key),
                            //     ..
                            // },
                            ..
                        },
                    ..
                } => {
                    match key {
                        KeyCode::F1 => settings.f ^= 1,
                        KeyCode::F2 => settings.f ^= 1 << 1,
                        KeyCode::F3 => settings.f ^= 1 << 2,
                        KeyCode::F4 => settings.f ^= 1 << 3,
                        KeyCode::F5 => settings.f ^= 1 << 4,
                        KeyCode::F6 => settings.f ^= 1 << 5,
                        KeyCode::F7 => settings.f ^= 1 << 6,
                        KeyCode::F8 => settings.f ^= 1 << 7,
                        KeyCode::F9 => settings.f ^= 1 << 8,
                        KeyCode::F10 => settings.f ^= 1 << 9,
                        KeyCode::F11 => settings.f ^= 1 << 10,
                        KeyCode::F12 => settings.f ^= 1 << 11,

                        KeyCode::Escape => {
                            target.exit();
                        }
                        _ => {}
                    };

                    println!();
                    println!("Keys:");
                    println!("F1 = long exposure ({})", settings.f & (1) == 0);
                    println!("F2 = sub mode ({})", settings.f & (1 << 1) != 0);
                    println!("F3 = heavy points ({})", settings.f & (1 << 2) != 0);
                    println!("F4 = cursor main toggle ({})", settings.f & (1 << 3) == 0);
                    println!("F5 = heavy cursor ({})", settings.f & (1 << 4) != 0);
                    println!("F6 = noise main toggle ({})", settings.f & (1 << 5) != 0);
                    println!("F7 = heavy noise ({})", settings.f & (1 << 6) != 0);
                    println!("F8 = freeze noise ({})", settings.f & (1 << 7) != 0);
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::MouseWheel {
                            delta: MouseScrollDelta::LineDelta(x, y),
                            ..
                        },
                    ..
                } => {
                    graphics.scrolled((x, y));
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(s),
                    ..
                } => {
                    graphics.resized((s.width, s.height));
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    graphics.cursor = Vec2::new(position.x as f32, position.y as f32);
                }

                Event::NewEvents(StartCause::Poll)
                | Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => graphics.frame(&settings),
                _ => {}
            };
        })
        .unwrap();
}
