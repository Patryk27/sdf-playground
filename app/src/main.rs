mod compiler;
mod renderer;

use self::compiler::*;
use self::renderer::*;
use pixels::{Pixels, SurfaceTexture};
use sdf_playground_common::Params;
use std::mem;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("sdf-playground")
        .with_inner_size(LogicalSize::new(700, 700))
        .build(&event_loop)
        .unwrap();

    let mut params = Params {
        width: window.inner_size().width,
        height: window.inner_size().height,
        time: 0.0,
    };

    let mut pixels = {
        let surface = SurfaceTexture::new(
            params.width,
            params.height,
            &window,
        );

        Pixels::new(params.width, params.height, surface)
            .unwrap()
    };

    let compiler = Compiler::spawn();
    let mut renderer: Option<Renderer> = None;
    let mut input = WinitInputHelper::new();
    let mut delta = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        if let Some(path) = compiler.poll() {
            renderer = Some(Renderer::new(
                &pixels,
                params.width,
                params.height,
                path,
            ));
        }

        if let Event::RedrawRequested(_) = event {
            if let Some(renderer) = &renderer {
                pixels
                    .render_with(
                        |encoder, target, context| {
                            let texture =
                                renderer.texture_view();

                            context
                                .scaling_renderer
                                .render(encoder, texture);

                            renderer.update(
                                &context.queue,
                                &params,
                            );

                            renderer
                                .render(encoder, target);

                            let delta = mem::replace(
                                &mut delta,
                                Instant::now(),
                            );

                            params.time += delta
                                .elapsed()
                                .as_secs_f32();

                            Ok(())
                        },
                    )
                    .unwrap();
            } else {
                pixels.render().unwrap();
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape)
                || input.close_requested()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if let Some(window_size) =
                input.window_resized()
            {
                params.width = window_size.width;
                params.height = window_size.height;

                pixels
                    .resize_surface(
                        params.width,
                        params.height,
                    )
                    .unwrap();

                if let Some(renderer) = &mut renderer {
                    renderer.resize(
                        &pixels,
                        params.width,
                        params.height,
                    );
                }
            }

            window.request_redraw();
        }
    });
}
