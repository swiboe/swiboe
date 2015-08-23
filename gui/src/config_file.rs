extern crate libc;
extern crate lua;
extern crate swiboe;

use ::keymap_handler::KeymapHandler;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::path;
use std::sync::{Arc, RwLock};
use std::thread;

use libc::c_int;
use lua::ffi::lua_State;
use lua::{State, Function};

// simple binding to Rust's tan function
unsafe extern "C" fn lua_call(lua_state: *mut lua_State) -> c_int {
  let mut state = lua::State::from_ptr(lua_state);
  let s = state.to_str(-1);
  println!("#sirver string: {:#?}", s);
  0
}

// mapping of function name to function pointer
const SWIBOE_LIB: [(&'static str, Function); 1] = [
  ("call", Some(lua_call)),
];

type Key = String;

struct ConfigFileRunner {
    lua_state: lua::State,
}

impl ConfigFileRunner {
    fn new() -> Self {
        let mut state = lua::State::new();
        state.open_libs();

        state.new_lib(&SWIBOE_LIB);
        state.set_global("swiboe");

        let mut this = ConfigFileRunner {
            lua_state: state,
        };

        // Save a reference to the object saver
        unsafe {
            let this_pointer: *mut ConfigFileRunner = mem::transmute(&mut this);
            this.lua_state.push_light_userdata(this_pointer);
        }
        this.lua_state.set_field(lua::ffi::LUA_REGISTRYINDEX, "this");

        this
    }

    fn run(&mut self, path: &path::Path) {
        let path = path.to_string_lossy();
        match self.lua_state.do_file(&path) {
            lua::ThreadStatus::Ok => (),
            err => println!("#sirver err.description(): {:#?}", err),
        }
    }
}

pub fn test_it() {
    let mut config_file_runner = ConfigFileRunner::new();

    config_file_runner.run(path::Path::new("test.lua"));


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
