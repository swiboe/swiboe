extern crate libc;
extern crate lua;
extern crate swiboe;

use std::collections::HashSet;
use std::mem;
use std::path;
use std::string;

// simple binding to Rust's tan function
unsafe extern "C" fn lua_call(lua_state: *mut lua::ffi::lua_State) -> libc::c_int {
  let mut state = lua::State::from_ptr(lua_state);
  let s = state.to_str(-1);
  println!("#sirver string: {:#?}", s);
  0
}

// mapping of function name to function pointer
const SWIBOE_LIB: [(&'static str, lua::Function); 1] = [
  ("call", Some(lua_call)),
];

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
        let rv = match lua_state.to_str(-1) {
            Some(rv) => Ok(rv),
            None => Err(LuaTableError::InvalidType),
        };
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
        let rv = match self.lua_state.to_str(-1) {
            Some(rv) => Ok(rv),
            None => Err(LuaTableError::InvalidType),
        };
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

    pub fn get_table<'b, T: Key>(&'b mut self, key: T) -> Result<LuaTable<'b>, LuaTableError> {
        try!(self.push_value_for_existing_key(key));
        let l = self.lua_state.type_of(-1);
        println!("#sirver l: {:#?}", l);
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
}
