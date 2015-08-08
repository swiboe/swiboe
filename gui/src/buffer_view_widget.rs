use ::buffer_views::{self, BufferViews, BufferView};
use cairo::Context;
use cairo::enums::{FontSlant, FontWeight};
use cairo;
use gtk::signal;
use gtk::traits::*;
use gtk;
use serde::json;
use std::cell::{RefCell, Cell};
use std::clone::Clone;
use std::cmp;
use std::collections::HashMap;
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
use time;

pub struct BufferViewWidget {
    drawing_area: gtk::DrawingArea,
}

fn draw(buffer_view: &BufferView, widget: gtk::Widget, cr: Context) {
    // Coordinate frame at the beginning is in pixels, top-left is 0, 0.
    let start = time::precise_time_ns();
    println!("#sirver draw: {}", start);

    let b = cr.get_antialias();
    cr.set_antialias(cairo::Antialias::Subpixel);
    println!("#sirver b: {:#?}", b);

    cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Normal);
    cr.set_font_size(12.);

    let font_extent = cr.font_extents();
    println!("#sirver font_extent.height: {:#?}", font_extent.height);

    // TODO(sirver): This uses the Cairo "Toy" API. The correct solution will be to use Pango to
    // convert our text into glyphs and then use cr.draw_glyphs() to actually get them on screen.
    // Of interest (maybe):
    // http://www.codeproject.com/Articles/796132/Programming-Cairo-text-output-beyond-the-toy-text
    cr.move_to(0., font_extent.ascent);
    for (y, line) in buffer_view.lines[buffer_view.top_line_index as usize..].iter().enumerate() {
        cr.move_to(0., font_extent.ascent + (y as f64) * font_extent.height);
        cr.show_text(line);
    }

    let duration = time::precise_time_ns() - start;
    println!("Drawing took {:#?}μs", (duration as f64)/ 1000.);
}

impl BufferViewWidget {
    pub fn new(buffer_views: Arc<RwLock<BufferViews>>) -> Self {
        let drawing_area = gtk::DrawingArea::new().unwrap();

        drawing_area.connect_draw(move |widget, cr| {
            let mut buffer_views = buffer_views.write().unwrap();
            // TODO(sirver): make the index configurable.
            let buffer_view = buffer_views.get_or_create(0);
            draw(&buffer_view, widget, cr);
            signal::Inhibit(false)
        });

        BufferViewWidget {
            drawing_area: drawing_area,
        }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.drawing_area
    }
}
