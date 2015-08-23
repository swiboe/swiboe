#[macro_use]
extern crate clap;
extern crate libc;
extern crate lua;
extern crate swiboe;

use std::path;
use std::thread;
use swiboe::client;

use libc::c_int;
use lua::ffi::lua_State;
use lua::{State, Function};

// simple binding to Rust's tan function
#[allow(non_snake_case)]
unsafe extern "C" fn tan(L: *mut lua_State) -> c_int {
  let mut state = lua::State::from_ptr(L);
  let num = state.to_number(-1);
  state.push_number(num.tan());
  1
}

// mapping of function name to function pointer
const MATHX_LIB: [(&'static str, Function); 1] = [
  ("tan", Some(tan)),
];

fn main() {
    let matches = clap::App::new("term_gui")
        .about("Key bindings plugin for Swiboe")
        .version(&crate_version!()[..])
        .arg(clap::Arg::with_name("SOCKET")
             .short("s")
             .long("socket")
             .help("Socket at which the master listens.")
             .required(true)
             .takes_value(true))
        .get_matches();


    let path = path::Path::new(matches.value_of("SOCKET").unwrap());
    let client = client::Client::connect(path).unwrap();

    let mut state = lua::State::new();
    state.open_libs();

    state.new_lib(&MATHX_LIB);
    state.set_global("swiboe");

    state.do_file("test.lua");


    // NOCOM(#sirver): a client.spin_forever would be cool.
    loop {
        thread::sleep_ms(100);
    }
}
