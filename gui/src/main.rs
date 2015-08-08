extern crate cairo;
extern crate gdk;
extern crate glib;
extern crate gtk;
extern crate serde;
extern crate switchboard;
extern crate time;

use cairo::Context;
use cairo::enums::{FontSlant, FontWeight};
use gtk::signal;
use gtk::traits::*;
use serde::json;
use std::cell::{RefCell, Cell};
use std::clone::Clone;
use std::cmp;
use std::convert;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path;
use std::rc::Rc;
use std::sync::{RwLock, Arc};
use switchboard::client;
use switchboard::ipc;
use switchboard::plugin_buffer;

// NOCOM(#sirver): duplicated. Export from client.
macro_rules! try_rpc {
    ($expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => {
            return ipc::RpcResult::Err(convert::From::from(err))
        }
    })
}

struct SwitchboardGtkGui {
    // NOCOM(#sirver): Is the Arc needed?
    buffer: Arc<RwLock<Buffer>>,
}

struct Buffer {
    start_line_index: isize,
    lines: Vec<String>,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            start_line_index: 0,
            lines: Vec::new(),
        }
    }

    fn set_contents(&mut self, text: &str) {
        self.lines = text.split("\n").map(|s| s.into()).collect();
    }
}

struct OnBufferCreated {
    buffer: Arc<RwLock<Buffer>>,
    rpc_caller: client::RpcCaller,
}

impl client::RemoteProcedure for OnBufferCreated {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        let info: plugin_buffer::BufferCreated = try_rpc!(json::from_value(args));
        println!("#sirver info.buffer_index: {:#?}", info.buffer_index);

        let rpc = self.rpc_caller.call("buffer.get_content", &plugin_buffer::GetContentRequest {
            buffer_index: info.buffer_index,
        });
        match rpc.wait().unwrap() {
            ipc::RpcResult::Ok(value) => {
                let response: plugin_buffer::GetContentResponse = json::from_value(value).unwrap();
                let mut buffer = self.buffer.write().unwrap();
                buffer.set_contents(&response.content);
            }
            _ => {},
        }
        ipc::RpcResult::success(())
    }
}


impl SwitchboardGtkGui {
    fn new() -> Self {
        let mut gui = SwitchboardGtkGui {
            buffer: Arc::new(RwLock::new(Buffer::new())),
        };

        let window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
        window.set_title("Switchboard");
        window.set_window_position(gtk::WindowPosition::Center);
        window.set_default_size(400, 300);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();

        let drawing_area = Rc::new(RefCell::new(gtk::DrawingArea::new().unwrap()));
        vbox.pack_start(&*drawing_area.borrow(), true, true, 0);

        let buffer_clone = gui.buffer.clone();
        drawing_area.borrow().connect_draw(move |widget, cr| {
            let buffer = buffer_clone.read().unwrap();
            draw(&buffer, widget, cr)
        });

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            signal::Inhibit(true)
        });
        window.add(&vbox);

        let client = client::Client::connect(path::Path::new("/tmp/sb.socket"));

        let on_buffer_created = OnBufferCreated {
            buffer: gui.buffer.clone(),
            rpc_caller: client.new_rpc_caller(),
        };
        client.new_rpc("on.buffer.new", Box::new(on_buffer_created));

        let rpc = client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
            buffer_index: 0,
        });
        match rpc.wait().unwrap() {
            ipc::RpcResult::Ok(value) => {
                let response: plugin_buffer::GetContentResponse = json::from_value(value).unwrap();
                let mut buffer = gui.buffer.write().unwrap();
                buffer.set_contents(&response.content);
            }
            _ => {
                let rpc = client.call("buffer.open", &plugin_buffer::OpenRequest {
                    uri: "file:///Users/sirver/Desktop/Programming/rust/Switchboard/gui/src/main.rs".into(),
                });
                let response = rpc.wait().unwrap();
                println!("#sirver response: {:#?}", response);
            }
        }
        drawing_area.borrow().queue_draw();

        // NOCOM(#sirver): maybe a custom gtk event?
        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            signal::Inhibit(true)
        });

        let drawing_area_clone = drawing_area.clone();
        let buffer_clone = gui.buffer.clone();
        window.connect_key_press_event(move |_, key| {
            // println!("#sirver key: {:#?}", key);
            let state = (*key).state;
            println!("#sirver state: {:#?}", state);
            // if state.contains(gdk::ModifierType::from_bits_truncate(::utils::META_KEY)) {
                // let keyval = (*key).keyval;
                if let Some(name_str) = gdk::keyval_name(key.keyval) {
                    match &name_str as &str {
                        "Up" => {
                            let mut buffer = buffer_clone.write().unwrap();
                            let index = cmp::max(buffer.start_line_index - 1, 0);
                            buffer.start_line_index = index;
                        },
                        "Down" => {
                            let mut buffer = buffer_clone.write().unwrap();
                            buffer.start_line_index += 1;
                        }
                        _ => (),
                    // // if let Some(button) = shortcuts.get(&name_str) {
                        // // button.clicked();
                        // return signal::Inhibit(true);
                    }
                    drawing_area_clone.borrow().queue_draw();
                }

            // }
            signal::Inhibit(false)
        });


        // NOCOM(#sirver): bring back
        // glib::source::timeout_add(100, move || {
            // while let Some(msg) = client.poll() {
            // }
            // glib::source::Continue(true)
        // });

        window.show_all();
        gui
    }
}

fn draw(buffer: &Buffer, widget: gtk::Widget, cr: Context) -> signal::Inhibit {
    let start = time::precise_time_ns();

    cr.scale(50f64, 50f64);

    cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Normal);
    cr.set_font_size(0.35);

    for (y, line) in buffer.lines[buffer.start_line_index as usize..].iter().enumerate() {
        cr.move_to(0.04, y as f64 * 0.53);
        cr.show_text(line);
        // cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Bold);
        // cr.show_text("select_sun_shine");
    }

    // cr.move_to(0.27, 0.65);
    // cr.text_path("void");
    // cr.set_source_rgb(0.5, 0.5, 1.0);
    // cr.fill_preserve();
    // cr.set_source_rgb(0.0, 0.0, 0.0);
    // cr.set_line_width(0.01);
    // cr.stroke();

    // cr.set_source_rgba(1.0, 0.2, 0.2, 0.6);
    // cr.arc(0.04, 0.53, 0.02, 0.0, PI * 2.);
    // cr.arc(0.27, 0.65, 0.02, 0.0, PI * 2.);
    // cr.fill();

    let duration = time::precise_time_ns() - start;
    println!("Drawing took {:#?}μs", (duration as f64)/ 1000.);
    signal::Inhibit(false)
}

fn main() {
    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let mut switchboard = SwitchboardGtkGui::new();

    gtk::main();
}
