use std::{env, sync::Arc};

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopBuilder,
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
        events.with_wayland().build()
    } else if settings.window.force_x11 {
        events.with_x11().build()
    } else {
        events.build()
    };

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
        .expect("Failed to open a window");

    let window = Arc::new(window);

    let mut graphics = graphics::Graphics::init(&settings, window.clone())
        .await
        .unwrap();

    let mut settings = RuntimeSettings { f: 0 };

    window.set_visible(true);

    events.run(move |event, _events, control| {
        control.set_poll();

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => control.set_exit(),
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => match key {
                VirtualKeyCode::F1 => settings.f ^= 1,
                VirtualKeyCode::F2 => settings.f ^= 1 << 1,
                VirtualKeyCode::F3 => settings.f ^= 1 << 2,
                VirtualKeyCode::F4 => settings.f ^= 1 << 3,
                VirtualKeyCode::F5 => settings.f ^= 1 << 4,
                VirtualKeyCode::F6 => settings.f ^= 1 << 5,
                VirtualKeyCode::F7 => settings.f ^= 1 << 6,
                VirtualKeyCode::F8 => settings.f ^= 1 << 7,
                VirtualKeyCode::F9 => settings.f ^= 1 << 8,
                VirtualKeyCode::F10 => settings.f ^= 1 << 9,
                VirtualKeyCode::F11 => settings.f ^= 1 << 10,
                VirtualKeyCode::F12 => settings.f ^= 1 << 11,

                VirtualKeyCode::Escape => {
                    control.set_exit();
                }
                _ => {}
            },
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
            Event::MainEventsCleared => graphics.frame(&settings),
            _ => {}
        };
    });
}
