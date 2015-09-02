extern crate libc;
extern crate lua;
extern crate serde_json;
extern crate swiboe;

use ::keymap_handler;
use std::collections::HashSet;
use std::mem;
use std::path;
use std::ptr;
use std::string;
use swiboe::client;

const REGISTRY_NAME_FOR_CONFIG_FILE_RUNNER: &'static str = "config_file_runner";

/// Returns a reference to the ConfigFileRunner that must have been pushed into the registry on
/// creation. The 'static is a lie, but the ConfigFileRunner always outlives the lua_state, so that
/// is safe.
fn get_config_file_runner(lua_state: &mut lua::State) -> Option<&'static mut ConfigFileRunner> {
    lua_state.get_field(lua::ffi::LUA_REGISTRYINDEX, REGISTRY_NAME_FOR_CONFIG_FILE_RUNNER);
    let pointer = lua_state.to_userdata(-1);
    lua_state.pop(1);
    if pointer.is_null() {
        return None;
    }
    unsafe {
        Some(mem::transmute(pointer))
    }
}

// Map a key to a Lua function.
unsafe extern "C" fn lua_map(lua_state: *mut lua::ffi::lua_State) -> libc::c_int {
  let mut state = lua::State::from_ptr(lua_state);
  let mut config_file_runner = get_config_file_runner(&mut state).unwrap();

  let is_table = state.is_table(-1);
  state.arg_check(is_table, -1, "Expected a table.");

  let mut table = LuaTable::new(&mut state);
  // NOCOM(#sirver): this should not crash if the key is not there.
  let mapping = {
      let mut keys_table = table.get_table("keys").unwrap();
      let keys = keys_table.get_array_values::<&str>().unwrap();
      // NOCOM(#sirver): this feels really weird, is this conversion really needed.
      let ref_keys: Vec<&str> = keys.iter().map(|ref_str| ref_str as &str).collect();
      // NOCOM(#sirver): should not crash.
      keymap_handler::Arpeggio::from_vec(&ref_keys).unwrap()
  };

  // NOCOM(#sirver): error handling
  let mut func = table.get_function("execute").unwrap();

  let kmh = &mut config_file_runner.keymap_handler;

  let thin_client_clone = config_file_runner.thin_client.clone();
  kmh.insert(keymap_handler::Mapping::new(
          mapping, Box::new(move || {
              let rv = func.prepare_call()
                  .push_thin_client(thin_client_clone.clone())
                  .call();
              if rv != lua::ThreadStatus::Ok {
                  println!("#sirver {}", func.lua_state.to_str(-1).unwrap())
              };
          })));
  0
}

unsafe extern "C" fn lua_call(lua_state: *mut lua::ffi::lua_State) -> libc::c_int {
    let mut state = lua::State::from_ptr(lua_state);
    let function_name = state.check_string(2);
    let json_arguments_as_string = state.check_string(3);

    let thin_client: &mut client::ThinClient = unsafe {
        state.check_userdata_typed(1, "swiboe.ThinClient")
    };


    // NOCOM(#sirver): should not crash if json_arguments_as_string is malformed.
    let args: serde_json::Value = serde_json::from_str(&json_arguments_as_string).unwrap();

    let mut rpc = thin_client.call(&function_name, &args);
    // NOCOM(#sirver): return failure information to Lua.
    rpc.wait();
    0
}

unsafe extern "C" fn lua_thin_client___gc(lua_state: *mut lua::ffi::lua_State) -> libc::c_int {
    let mut state = lua::State::from_ptr(lua_state);
    let thin_client: &mut client::ThinClient = unsafe {
        state.check_userdata_typed(1, "swiboe.ThinClient")
    };
    drop(thin_client);
    0
}

// mapping of function name to function pointer
const THIN_CLIENT_FNS: [(&'static str, lua::Function); 2] = [
  ("call", Some(lua_call)),
  ("__gc", Some(lua_thin_client___gc)),
];

// mapping of function name to function pointer
const SWIBOE_LIB: [(&'static str, lua::Function); 1] = [
  ("map", Some(lua_map)),
];

pub struct ConfigFileRunner {
    lua_state: lua::State,
    pub keymap_handler: keymap_handler::KeymapHandler,
    thin_client: client::ThinClient,
}

impl ConfigFileRunner {
    pub fn new(thin_client: client::ThinClient) -> Box<Self> {
    // This is boxed so that we can save a pointer to it in the Lua registry.
        let mut state = lua::State::new();
        state.open_libs();

        state.new_lib(&SWIBOE_LIB);
        state.set_global("swiboe");

        // Create the metatable for all of our wrapped classes.
        state.new_metatable("swiboe.ThinClient");
        state.push_string("__index");
        state.push_value(-2);  // pushes the metatable
        state.set_table(-3);  // metatable.__index = metatable
        state.set_fns(&THIN_CLIENT_FNS, 0);

        let mut this = Box::new(ConfigFileRunner {
            lua_state: state,
            keymap_handler: keymap_handler::KeymapHandler::new(),
            thin_client: thin_client,
        });

        // Save a reference to the ConfigFileRunner.
        unsafe {
            let this_pointer: *mut ConfigFileRunner = mem::transmute(&mut *this);
            this.lua_state.push_light_userdata(this_pointer);
        }
        this.lua_state.set_field(lua::ffi::LUA_REGISTRYINDEX, REGISTRY_NAME_FOR_CONFIG_FILE_RUNNER);

        this
    }

    pub fn run(&mut self, path: &path::Path) {
        let path = path.to_string_lossy();
        match self.lua_state.do_file(&path) {
            lua::ThreadStatus::Ok => (),
            err => panic!("#sirver {:#?}: {}", err, self.lua_state.to_str(-1).unwrap()),
        }
    }
}

pub struct LuaTable<'a> {
    table_index: lua::Index,
    lua_state: &'a mut lua::State,
}

#[derive(Debug,Eq,PartialEq)]
pub enum LuaTableError {
    UnknownKey(String),
    InvalidType,
}

pub trait Key: string::ToString {
    type IntoType;

    fn push(&self, lua_state: &mut lua::State);
    fn pop(lua_state: &mut lua::State) -> Result<Self::IntoType, LuaTableError>;
}

impl<'a> Key for &'a str {
    type IntoType = String;

    fn push(&self, lua_state: &mut lua::State) {
        lua_state.push_string(self);
    }

    fn pop(lua_state: &mut lua::State) -> Result<Self::IntoType, LuaTableError> {
        let rv = lua_state.to_str(-1).ok_or(LuaTableError::InvalidType);
        lua_state.pop(1);
        rv
    }
}

impl Key for lua::Integer {
    type IntoType = lua::Integer;

    fn push(&self, lua_state: &mut lua::State) {
        lua_state.push_integer(*self);
    }

    fn pop(lua_state: &mut lua::State) -> Result<Self::IntoType, LuaTableError> {
        let rv = lua_state.to_integerx(-1).ok_or(LuaTableError::InvalidType);
        lua_state.pop(1);
        rv
    }
}

impl Key for lua::Number {
    type IntoType = lua::Number;

    fn push(&self, lua_state: &mut lua::State) {
        lua_state.push_number(*self);
    }

    fn pop(lua_state: &mut lua::State) -> Result<Self::IntoType, LuaTableError> {
        let rv = lua_state.to_numberx(-1).ok_or(LuaTableError::InvalidType);
        lua_state.pop(1);
        rv
    }
}

pub struct LuaFunction {
    lua_state: lua::State,
    reference: lua::Reference,
}

impl LuaFunction {
    fn prepare_call(&mut self) -> PrepareCall {
        let index = self.reference.value();
        self.lua_state.raw_geti(lua::ffi::LUA_REGISTRYINDEX, index as lua::Integer); // S: function
        PrepareCall {
            lua_function: self,
            num_arguments: 0,
        }
    }
}

struct PrepareCall<'a> {
    lua_function: &'a mut LuaFunction,
    num_arguments: i32,
}

// NOCOM(#sirver): add push_ functions here.
// NOCOM(#sirver): deal with return values.. right now we just discard them.
impl<'a> PrepareCall<'a> {
    fn call(self) -> lua::ThreadStatus {
        // S: <function> <args...>
        let rv = self.lua_function.lua_state.pcall(self.num_arguments, 0, 0);
        rv
    }

    // NOCOM(#sirver): should we instead just pass a reference to Lua?
    fn push_thin_client(mut self, thin_client: client::ThinClient) -> Self {
        {
            let mut lua_state = &mut self.lua_function.lua_state;
            unsafe {
                let new_thin_client = lua_state.new_userdata(mem::size_of::<client::ThinClient>() as libc::size_t) as *mut client::ThinClient;
                ptr::write(&mut *new_thin_client, thin_client);
            }
            lua_state.set_metatable_from_registry("swiboe.ThinClient");
            self.num_arguments += 1;
        }
        self
    }
}

// NOCOM(#sirver): should remove table on drop.
impl<'a> LuaTable<'a> {
    pub fn new(lua_state: &'a mut lua::State) -> Self {
        LuaTable {
            table_index: lua_state.get_top(),
            lua_state: lua_state,
        }
    }

    fn push_value_for_existing_key<T: Key>(&mut self, key: T) -> Result<(), LuaTableError> {
        key.push(self.lua_state); // S: key
        self.lua_state.raw_get(self.table_index); // S: value
        if self.lua_state.is_nil(-1) {
            self.lua_state.pop(1);
            return Err(LuaTableError::UnknownKey(key.to_string()));
        }
        Ok(())
    }

    pub fn has_key<T: Key>(&mut self, key: T) -> bool {
        if self.push_value_for_existing_key(key).is_err() {
            return false;
        }
        self.lua_state.pop(1);
        return true;
    }

    pub fn get_string<T: Key>(&mut self, key: T) -> Result<String, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        let rv = self.lua_state.to_str(-1).ok_or(LuaTableError::InvalidType);
        self.lua_state.pop(1);
        rv
    }

    pub fn get_double<T: Key>(&mut self, key: T) -> Result<lua::Number, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        lua::Number::pop(self.lua_state)
    }

    pub fn get_int<T: Key>(&mut self, key: T) -> Result<lua::Integer, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        lua::Integer::pop(self.lua_state)
    }

    pub fn get_function<T: Key>(&mut self, key: T) -> Result<LuaFunction, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        // S: ... <function>
        if !self.lua_state.is_fn(-1) {
            self.lua_state.pop(1);
            return Err(LuaTableError::InvalidType);
        }
        let reference = self.lua_state.reference(lua::ffi::LUA_REGISTRYINDEX); // S: ...

        Ok(LuaFunction {
            // NOCOM(#sirver): ouch
            lua_state: lua::State::from_ptr(self.lua_state.as_ptr()),
            reference: reference,
        })
    }

    pub fn get_table<'b, T: Key>(&'b mut self, key: T) -> Result<LuaTable<'b>, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        if !self.lua_state.is_table(-1) {
            self.lua_state.pop(1);
            return Err(LuaTableError::InvalidType);
        }
        Ok(LuaTable::new(self.lua_state))
    }

    pub fn get_keys(&mut self) -> Result<HashSet<String>, LuaTableError> {
        let mut table_keys = HashSet::new();
        self.lua_state.push_nil(); // S: table ... nil
		while self.lua_state.next(self.table_index) {   // S: key value
			self.lua_state.pop(1);               // S: key
            match self.lua_state.to_str(-1) {
                Some(key) => {
                    table_keys.insert(key);
                },
                None => {
                    self.lua_state.pop(1); // S: table
                    return Err(LuaTableError::InvalidType);
                },
            }
		}
        Ok(table_keys)
    }

    pub fn get_array_values<Value: Key>(&mut self) -> Result<Vec<Value::IntoType>, LuaTableError> {
        let mut values = Vec::new();
        let mut key = 1;
        loop {
            self.lua_state.push_integer(key);  // S: table ... <first key>
            self.lua_state.raw_get(self.table_index);  // S: table ... <value>
            if self.lua_state.is_nil(-1) {
                self.lua_state.pop(1);
                return Ok(values);
            }
            values.push(try!(Value::pop(&mut self.lua_state)));
            key += 1;
        }
    }
}

// NOCOM(#sirver): should warn about unused keys.
#[cfg(test)]
mod tests {
    use lua;
    use std::collections::HashSet;
    use super::*;

    #[test]
    fn get_non_existing_key() {
        let mut state = lua::State::new();
        state.do_string(r#"return {}"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Err(LuaTableError::UnknownKey("a".into())), t.get_string("a"));
    }

    #[test]
    fn get_string_with_string_key() {
        let mut state = lua::State::new();
        state.do_string(r#"
        return {
            a = "blub",
        }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok("blub".into()), t.get_string("a"));
    }

    #[test]
    fn get_string_with_integer_key() {
        let mut state = lua::State::new();
        state.do_string(r#" return { "blub", "blah", "fasel" } "#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok("blub".into()), t.get_string(1));
        assert_eq!(Ok("blah".into()), t.get_string(2));
        assert_eq!(Ok("fasel".into()), t.get_string(3));
    }

    #[test]
    fn get_double_with_string_key() {
        let mut state = lua::State::new();
        state.do_string(r#"
        return {
            a = 3.131,
        }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok("3.131".into()), t.get_string("a"));
        assert_eq!(Ok(3.131), t.get_double("a"));
    }

    #[test]
    fn get_int_with_string_key() {
        let mut state = lua::State::new();
        state.do_string(r#"return { a = 3 }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok("3".into()), t.get_string("a"));
        assert_eq!(Ok(3), t.get_int("a"));
    }

    #[test]
    fn get_int_on_double() {
        let mut state = lua::State::new();
        state.do_string(r#"return { a = 3.13 }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Err(LuaTableError::InvalidType),
            t.get_int("a"));
    }

    #[test]
    fn has_key_with_string_key() {
        let mut state = lua::State::new();
        state.do_string(r#"return { a = 3.13 }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(true, t.has_key("a"));
        assert_eq!(false, t.has_key("b"));
    }

    #[test]
    fn has_key_with_integer_key() {
        let mut state = lua::State::new();
        state.do_string(r#"return { 3.13 }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(false, t.has_key(0));
        assert_eq!(true, t.has_key(1));
        assert_eq!(false, t.has_key(2));
    }

    #[test]
    fn get_table() {
        let mut state = lua::State::new();
        state.do_string(r#"return {
            a = 1,
            b = {
                blub = "blah"
            },
            c = 3,
        }"#);

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok(1), t.get_int("a"));
        {
            let mut t1 = t.get_table("b").unwrap();
            assert_eq!(Ok("blah".into()), t1.get_string("blub"));
        }
        assert_eq!(Ok(3), t.get_int("c"));
    }

    #[test]
    fn keys() {
        let mut state = lua::State::new();
        state.do_string(r#"return { a = 3.13, blub = 1, z = true }"#);

        let mut golden = HashSet::new();
        golden.insert("a".into());
        golden.insert("blub".into());
        golden.insert("z".into());

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok(golden), t.get_keys());
    }

    #[test]
    fn get_array_values() {
        let mut state = lua::State::new();
        state.do_string(r#"return { "1", "2", "3", "4" }"#);

        let mut t = LuaTable::new(&mut state);
        let as_strings = t.get_array_values::<&str>();
        assert_eq!(Ok(vec!["1".into(), "2".into(), "3".into(), "4".into()]), as_strings);

        let as_ints = t.get_array_values::<lua::Integer>();
        assert_eq!(Ok(vec![1, 2, 3, 4]), as_ints);
    }

    #[test]
    fn get_function() {
        let mut state = lua::State::new();
        state.do_string(r#"

        b = {
            a = function()
                b.blub = "Hi"
            end,
        }

        return b
        "#);

        let mut func;
        {
            let mut t = LuaTable::new(&mut state);
            func = t.get_function("a").unwrap();
        }

        // Function outlives the table, but not the state.
        func.prepare_call().call();

        let mut t = LuaTable::new(&mut state);
        assert_eq!(Ok("Hi".into()), t.get_string("blub"));
    }
}
