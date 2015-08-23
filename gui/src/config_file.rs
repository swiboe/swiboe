extern crate libc;
extern crate lua;
extern crate swiboe;

use ::keymap_handler::KeymapHandler;
use std::collections::{HashMap, HashSet};
use std::path;
use std::sync::{Arc, RwLock};
use std::thread;

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

type Key = String;

pub fn test_it() {
    let mut state = lua::State::new();
    state.open_libs();

    state.new_lib(&MATHX_LIB);
    state.set_global("swiboe");

    state.do_file("test.lua");


    // let plugin = BufferPlugin {
        // buffers: Arc::new(RwLock::new(BuffersManager::new(client.clone()))),
        // client: client,
    // };

    // let new = Box::new(New { buffers: plugin.buffers.clone() });
    // plugin.client.new_rpc("buffer.new", new);

    // let delete = Box::new(Delete { buffers: plugin.buffers.clone() });
    // plugin.client.new_rpc("buffer.delete", delete);

    // let get_content = Box::new(GetContent { buffers: plugin.buffers.clone() });
    // plugin.client.new_rpc("buffer.get_content", get_content);

    // let open = Box::new(Open { buffers: plugin.buffers.clone() });
    // plugin.client.new_rpc("buffer.open", open);

    // let list = Box::new(List { buffers: plugin.buffers.clone() });
    // plugin.client.new_rpc("buffer.list", list);


    // NOCOM(#sirver): a client.spin_forever would be cool.
}
