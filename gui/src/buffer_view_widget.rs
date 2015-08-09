use ::buffer_views::{self, BufferViews, BufferView};
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

struct Color {
    r: f64,
    g: f64,
    b: f64,
}

impl Color {
    const fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Color {
            r: (r as f64) / 255.,
            g: (g as f64) / 255.,
            b: (b as f64) / 255.,
        }
    }
}

// Colors are from Solarized: http://ethanschoonover.com/solarized. They are temporary until we can
// support themes, but are reasonable defaults for starters.
const BASE03  : Color = Color::from_rgb_u8(  0,  43,  54);
const BASE02  : Color = Color::from_rgb_u8(  7,  54,  66);
const BASE01  : Color = Color::from_rgb_u8( 88, 110, 117);
const BASE00  : Color = Color::from_rgb_u8(101, 123, 131);
const BASE0   : Color = Color::from_rgb_u8(131, 148, 150);
const BASE1   : Color = Color::from_rgb_u8(147, 161, 161);
const BASE2   : Color = Color::from_rgb_u8(238, 232, 213);
const BASE3   : Color = Color::from_rgb_u8(253, 246, 227);
const YELLOW  : Color = Color::from_rgb_u8(181, 137,   0);
const ORANGE  : Color = Color::from_rgb_u8(203,  75,  22);
const RED     : Color = Color::from_rgb_u8(220,  50,  47);
const MAGENTA : Color = Color::from_rgb_u8(211,  54, 130);
const VIOLET  : Color = Color::from_rgb_u8(108, 113, 196);
const BLUE    : Color = Color::from_rgb_u8( 38, 139, 210);
const CYAN    : Color = Color::from_rgb_u8( 42, 161, 152);
const GREEN   : Color = Color::from_rgb_u8(133, 153,   0);

trait CairoContexExt {
    fn set_source_color(&self, color: &Color);
}

impl CairoContexExt for cairo::Context {
    fn set_source_color(&self, color: &Color) {
        self.set_source_rgb(color.r, color.g, color.b);
    }
}

pub struct BufferViewWidget {
    drawing_area: gtk::DrawingArea,
    inner_size: Rc<Cell<cairo::RectangleInt>>,
}

fn draw(buffer_view: &BufferView, inner_size: cairo::RectangleInt, cr: cairo::Context) {
    // Coordinate frame at the beginning is in pixels, top-left is 0, 0.

    let start = time::precise_time_ns();
    println!("#sirver draw: {}", start);

    // Draw the background.
    cr.set_source_color(&BASE03);
    cr.rectangle(inner_size.x as f64, inner_size.y as f64, inner_size.width as f64, inner_size.height as f64);
    cr.fill();

    cr.select_font_face("Menlo", FontSlant::Normal, FontWeight::Normal);
    cr.set_font_size(12.);

    let font_extents = cr.font_extents();

    // Draw the cursor.
    {
        let position = buffer_view.cursor.position;
        let (x, y, c) = match buffer_view.lines.get(position.line_index as usize) {
            Some(line) => {
                let y = (position.line_index - buffer_view.top_line_index) as f64 * font_extents.height;
                // NOCOM(#sirver): can this ever fail?
                let (x, c) = match line.char_indices().skip(position.column_index as usize).next() {
                    None => { // Empty line
                        (0., '_')
                    },
                    Some((index, _)) => {
                        let (before, after) = line.split_at(index);
                        // NOCOM(#sirver): can this ever fail?
                        let (current_char, _) = after.slice_shift_char().unwrap();

                        println!("#sirver before: {:#?}", before);
                        println!("#sirver current_char: {:#?}", current_char);
                        let before_extends = cr.text_extents(before);
                        (before_extends.x_advance, current_char)
                    }
                };
                (x, y, c)
            },
            None => {
                (0., 0., '_')
            }
        };
        let char_extends = cr.text_extents(&c.to_string());
        cr.set_source_color(&ORANGE);
        cr.rectangle(x, y, char_extends.x_advance, font_extents.height);
        cr.fill();
    }

    // TODO(sirver): This uses the Cairo "Toy" API. The correct solution will be to use Pango to
    // convert our text into glyphs and then use cr.draw_glyphs() to actually get them on screen.
    // Of interest (maybe):
    // http://www.codeproject.com/Articles/796132/Programming-Cairo-text-output-beyond-the-toy-text
    // TODO(sirver): Properly support Unicode characters. The toy API is not rendering them
    // correctly.
    cr.set_source_color(&BASE0);
    cr.move_to(0., font_extents.ascent);
    for (y, line) in buffer_view.lines[buffer_view.top_line_index as usize..].iter().enumerate() {
        cr.move_to(0., font_extents.ascent + (y as f64) * font_extents.height);
        cr.show_text(line);
    }

    let duration = time::precise_time_ns() - start;
    println!("Drawing took {:#?}μs", (duration as f64)/ 1000.);
}

impl BufferViewWidget {
    pub fn new(buffer_views: Arc<RwLock<BufferViews>>) -> Self {
        let mut buffer_view_widget = BufferViewWidget {
            drawing_area: gtk::DrawingArea::new().unwrap(),
            inner_size: Rc::new(Cell::new(cairo::RectangleInt {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            })),
        };

        let inner_size_clone = buffer_view_widget.inner_size.clone();
        buffer_view_widget.drawing_area.connect_draw(move |_, cr| {
            let mut buffer_views = buffer_views.write().unwrap();
            // TODO(sirver): make the index configurable.
            let buffer_view = buffer_views.get_or_create(0);
            draw(&buffer_view, inner_size_clone.get(), cr);
            signal::Inhibit(false)
        });

        let inner_size_clone = buffer_view_widget.inner_size.clone();
        buffer_view_widget.drawing_area.connect_size_allocate(move |widget, rect| {
            println!("#sirver resize rect: {:#?}", rect);
            inner_size_clone.set(*rect);
            widget.queue_draw();
        });
        buffer_view_widget
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.drawing_area
    }
}
