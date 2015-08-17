extern crate rustbox;
extern crate switchboard;

use std::default::Default;

use rustbox::Key;
use rustbox::{Color, RustBox};
use std::path;
use switchboard::client;

fn main() {
    let rustbox = match RustBox::init(Default::default()) {
        Result::Ok(v) => v,
        Result::Err(e) => panic!("{}", e),
    };

    let client = client::Client::connect(path::Path::new("/tmp/sb.socket"));

    rustbox.print(1, 1, rustbox::RB_BOLD, Color::White, Color::Black, "Hello, world!");
    rustbox.print(1, 3, rustbox::RB_BOLD, Color::White, Color::Black,
                  "Press 'q' to quit.");

    let (mut w, mut h) = (rustbox.width() as i32, rustbox.height() as i32);
    loop {
        rustbox.present();
        match rustbox.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    Some(Key::Char('q')) => break,
                    _ => rustbox.print(0, 7, rustbox::RB_NORMAL, Color::Default, Color::Default, &format!("{:?}", key)),
                }
            },
            Ok(rustbox::Event::ResizeEvent(width, height)) => {
                w = width;
                h = height;
            },
            Ok(rustbox::Event::MouseEvent(m, a, b)) => {
                rustbox.print(0, (h - 1) as usize, rustbox::RB_NORMAL, Color::White, Color::White,
                              &format!("{:?} {:?} {:?}", m, a, b));
            },
            Err(e) => panic!("{}", e),
            _ => { }
        }
    }
}
