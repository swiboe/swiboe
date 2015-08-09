#![cfg(not(test))]
#![feature(const_fn)]
#![feature(custom_derive, plugin)]
#![feature(str_char)]
#![feature(str_split_at)]
#![plugin(serde_macros)]

extern crate cairo;
extern crate gdk;
extern crate glib;
extern crate gtk;
extern crate serde;
extern crate switchboard;
extern crate time;
extern crate uuid;

pub mod buffer_views;
pub mod buffer_view_widget;
pub mod command;
