use std::rc::Rc;

use log::error;
use update::UpdateResult;
use winit::{event::Event, event_loop::EventLoop};

mod components;
mod data;
mod draw;
mod game;
mod map;
mod pixel_helper;
mod render;
mod resources;
mod rng;
mod shapes;
mod spawn;
mod spritegrid;
mod sprites;
mod systems;
mod update;
mod utils;

use data::*;
use systems::*;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Trace).expect("error initializing logger");

        wasm_bindgen_futures::spawn_local(run());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();

        pollster::block_on(run());
    }
}

async fn run() {
    // wasm example
    // https://github.com/parasyte/pixels/blob/main/examples/minimal-web/src/main.rs
    let event_loop = EventLoop::new();

    let (window, p_width, p_height, mut _hidpi_factor) = pixel_helper::create_window(
        SCREEN_WIDTH as f64,
        SCREEN_HEIGHT as f64,
        "Zombie Hunter",
        &event_loop,
    );

    let window = Rc::new(window);

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;

        // Retrieve current width and height dimensions of browser client window
        let get_window_size = || {
            let client_window = web_sys::window().unwrap();
            LogicalSize::new(
                client_window.inner_width().unwrap().as_f64().unwrap(),
                client_window.inner_height().unwrap().as_f64().unwrap(),
            )
        };

        let window = Rc::clone(&window);

        // Initialize winit window with current dimensions of browser client
        window.set_inner_size(get_window_size());

        let client_window = web_sys::window().unwrap();

        // Attach winit canvas to body element
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = get_window_size();
            window.set_inner_size(size)
        }) as Box<dyn FnMut(_)>);
        client_window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let mut game = game::init(Rc::clone(&window)).await;

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        // render
        if let Event::RedrawRequested(_) = event {
            let r = render::do_render(&mut game);

            if let Err(e) = r {
                error!("pixels.render() failed: {}", e);
                control_flow.set_exit();
            }
        }

        // update
        // Handle input events
        if game.input.update(&event) {
            use UpdateResult::*;

            let r = update::do_update(&mut game);

            match r {
                Ok(Exit) => control_flow.set_exit(),
                Ok(None) => (),
                Err(e) => {
                    error!("{}", e);
                    control_flow.set_exit();
                }
            }

            window.request_redraw();
        }
    });
}
