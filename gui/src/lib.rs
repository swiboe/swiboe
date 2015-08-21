#![cfg(not(test))]
#![feature(const_fn)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

#[macro_use] extern crate swiboe;
extern crate serde;
extern crate serde_json;
extern crate time;
extern crate uuid;

pub mod buffer_views;
pub mod command;
