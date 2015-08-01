extern crate cairo;
extern crate glib;
extern crate gtk;
extern crate switchboard;

use cairo::Context;
use cairo::enums::{FontSlant, FontWeight};
use gtk::signal::Inhibit;
use gtk::traits::*;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use switchboard::client::Client;

struct SwitchboardGtkGui;

impl SwitchboardGtkGui {
    fn new() -> Self {
        let window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
        window.set_title("Switchboard");
        window.set_window_position(gtk::WindowPosition::Center);
        window.set_default_size(400, 300);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();

        let drawing_area = gtk::DrawingArea::new().unwrap();
        vbox.pack_start(&drawing_area, true, true, 0);

        drawing_area.connect_draw(draw);

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(true)
        });
        window.add(&drawing_area);
        window.add(&vbox);

        let client = Client::connect("/tmp/sb.socket");

        // NOCOM(#sirver): maybe a custom gtk event?
        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(true)
        });

        // NOCOM(#sirver): bring back
        // glib::source::timeout_add(100, move || {
            // while let Some(msg) = client.poll() {
            // }
            // glib::source::Continue(true)
        // });

        window.show_all();
        SwitchboardGtkGui
    }
}

fn draw(widget: gtk::Widget, cr: Context) -> Inhibit {
        cr.scale(50f64, 50f64);

        cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Normal);
        cr.set_font_size(0.35);

        cr.move_to(0.04, 0.53);
        cr.show_text("cr.");
        cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Bold);
        cr.show_text("select_sun_shine");

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

        Inhibit(false)
}

fn main() {
    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let mut switchboard = SwitchboardGtkGui::new();

    gtk::main();
}
