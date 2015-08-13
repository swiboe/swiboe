use ::command::GuiCommand;
use cairo::Context;
use cairo::enums::{FontSlant, FontWeight};
use cairo;
use gtk::signal;
use gtk::traits::*;
use gtk;
use serde::json;
use serde;
use std::cell::{RefCell, Cell};
use std::clone::Clone;
use std::cmp;
use std::collections::HashMap;
use std::convert;
use std::f64::consts::PI;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::ops;
use std::path;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{RwLock, Arc, Mutex};
use switchboard::client;
use switchboard::ipc;
use switchboard::plugin_buffer;
use time;
use uuid::Uuid;

// NOCOM(#sirver): nothing in this file is tested .
#[derive(Debug)]
pub enum BufferViewError {
    UnknownCursor,
}

impl From<BufferViewError> for ipc::RpcError {
     fn from(error: BufferViewError) -> Self {
         use switchboard::ipc::RpcErrorKind::*;

         let (kind, details) = match error {
             BufferViewError::UnknownCursor => (InvalidArgs, format!("unknown_cursor")),
         };

         ipc::RpcError {
             kind: kind,
             details: Some(json::to_value(&details)),
         }
     }
}

// NOCOM(#sirver): add a test for this.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScrollRequest {
    pub buffer_view_id: String,
    pub delta: isize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ScrollResponse;

struct Scroll {
    buffer_views: Arc<RwLock<BufferViews>>,
}

impl client::RemoteProcedure for Scroll {
    fn call(&mut self, mut sender: client::RpcSender, args: json::Value) {
        let request: ScrollRequest = try_rpc!(sender, json::from_value(args));

        let mut buffer_views = self.buffer_views.write().unwrap();
        buffer_views.scroll(&request.buffer_view_id, request.delta);

        let response = ScrollResponse;
        sender.finish(ipc::RpcResult::success(response))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Position {
    /// 0 based line index into the buffer.
    pub line_index: isize,

    /// 0 based glyph index into the line. A multibyte character only counts as one here.
    pub column_index: isize,
}

impl ops::Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Position {
            line_index: self.line_index + rhs.line_index,
            column_index: self.column_index + rhs.column_index,
        }
    }
}

pub struct Cursor {
    id: String,
    pub wanted_position: Position,
    pub position: Position,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {
            id: Uuid::new_v4().to_hyphenated_string(),
            wanted_position: Position { line_index: 0, column_index: 0 },
            position: Position { line_index: 0, column_index: 0 },
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MoveCursorRequest {
    pub cursor_id: String,
    // NOCOM(#sirver): this could also be absolute or so.
    pub delta: Position,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MoveCursorResponse;

struct MoveCursor {
    buffer_views: Arc<RwLock<BufferViews>>,
}

impl client::RemoteProcedure for MoveCursor {
    fn call(&mut self,  mut sender: client::RpcSender, args: json::Value) {
        println!("#sirver Beginning of MoveCursor: {:#?}", time::precise_time_ns());
        let request: MoveCursorRequest = try_rpc!(sender, json::from_value(args));

        let mut buffer_views = self.buffer_views.write().unwrap();

        try_rpc!(sender, buffer_views.move_cursor(&request.cursor_id, request.delta));
        println!("#sirver End of MoveCursor: {:#?}", time::precise_time_ns());
        sender.finish(ipc::RpcResult::success(MoveCursorResponse))
    }
}

pub struct BufferView {
    id: String,
    pub cursor: Cursor,
    pub top_line_index: isize,
    pub lines: Vec<String>,
}

impl BufferView {
    pub fn new() -> Self {
        BufferView {
            id: Uuid::new_v4().to_hyphenated_string(),
            top_line_index: 0,
            lines: Vec::new(),
            cursor: Cursor::new(),
        }
    }

    pub fn set_contents(&mut self, text: &str) {
        self.lines = text.split("\n").map(|s| s.into()).collect();
    }

    fn scroll(&mut self, delta: isize) {
        self.top_line_index += delta;
        self.top_line_index = cmp::min(self.top_line_index, (self.lines.len() - 1) as isize);
        self.top_line_index = cmp::max(self.top_line_index, 0);
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Handles all buffer views and dispatches RPC calls to them.
pub struct BufferViews {
    // NOCOM(#sirver): is the gui id needed?
    gui_id: String,
    buffer_views: HashMap<usize, BufferView>,
    sender: client::Sender,
    // NOCOM(#sirver): are these mutex really needed?
    commands: Mutex<mpsc::Sender<GuiCommand>>,
}

impl BufferViews {
    pub fn new(gui_id: &str, commands: mpsc::Sender<GuiCommand>, client: &client::Client) -> Arc<RwLock<Self>> {
        let mut buffer_view = Arc::new(RwLock::new(BufferViews {
            gui_id: gui_id.to_string(),
            buffer_views: HashMap::new(),
            sender: client.new_sender(),
            commands: Mutex::new(commands),
        }));

        let scroll = Scroll {
            buffer_views: buffer_view.clone(),
        };
        client.new_rpc("gui.buffer_view.scroll", Box::new(scroll));

        let move_cursor = MoveCursor {
            buffer_views: buffer_view.clone(),
        };
        client.new_rpc("gui.buffer_view.move_cursor", Box::new(move_cursor));

        let on_buffer_created = OnBufferCreated {
            buffer_views: buffer_view.clone(),
            sender: client.new_sender(),
        };
        client.new_rpc("on.buffer.new", Box::new(on_buffer_created));


        {
            let mut bv = buffer_view.write().unwrap();
            bv.update_all_buffers_blocking();
        }
        buffer_view
    }

    fn update_all_buffers_blocking(&mut self) {
        // NOCOM(#sirver): all these unwraps are very dangerous.
        let mut rpc = self.sender.call("buffer.list", &plugin_buffer::ListRequest);
        let result: plugin_buffer::ListResponse = rpc.wait_for().unwrap();

        for buffer_index in result.buffer_indices {
            let mut rpc = self.sender.call("buffer.get_content", &plugin_buffer::GetContentRequest {
                buffer_index: buffer_index,
            });
            let response: plugin_buffer::GetContentResponse = rpc.wait_for().unwrap();
            let buffer = self.get_or_create(buffer_index);
            buffer.set_contents(&response.content);
        }
    }

    // NOCOM(#sirver): write tests for move cursor.
    fn move_cursor(&mut self, id: &str, delta: Position) -> Result<(), BufferViewError> {
        println!("#sirver id: {:#?},delta: {:#?}", id, delta);
        for (_, buffer_view) in self.buffer_views.iter_mut() {
            if buffer_view.cursor.id == id {
                buffer_view.cursor.wanted_position = buffer_view.cursor.wanted_position + delta;
                let mut new_pos = buffer_view.cursor.wanted_position;

                new_pos.line_index = cmp::max(0, cmp::min(buffer_view.lines.len() as isize - 1, new_pos.line_index));
                new_pos.column_index = match buffer_view.lines.get(new_pos.line_index as usize) {
                    None => 0,
                    Some(line) => {
                        cmp::max(0, cmp::min(line.len() as isize - 1, new_pos.column_index))
                    }
                };

                buffer_view.cursor.position = new_pos;

                // TODO(sirver): Publish the cursor position for other GUIs.

                self.commands.lock().unwrap().send(GuiCommand::Redraw).unwrap();

                return Ok(());
            }
        }
        Err(BufferViewError::UnknownCursor)
    }

    pub fn insert(&mut self, buffer_index: usize, buffer_view: BufferView) {
        self.buffer_views.insert(buffer_index, buffer_view);
    }

    // NOCOM(#sirver): does this need to be public?
    pub fn get_or_create(&mut self, buffer_index: usize) -> &mut BufferView {
        self.buffer_views.entry(buffer_index).or_insert_with(BufferView::new)
    }

    fn get_by_id(&self, id: &str) -> Option<&BufferView> {
        for (_, buffer_view) in self.buffer_views.iter() {
            if buffer_view.id == id {
                return Some(buffer_view);
            }
        }
        None
    }

    fn get_mut_by_id(&mut self, id: &str) -> Option<&mut BufferView> {
        for (_, buffer_view) in self.buffer_views.iter_mut() {
            if buffer_view.id == id {
                return Some(buffer_view);
            }
        }
        None
    }

    fn scroll(&mut self, buffer_view_id: &str, delta: isize) {
        self.get_mut_by_id(&buffer_view_id).and_then(|view| {
            Some(view.scroll(delta))
        }).and_then(|_| {
            let c = self.commands.lock().unwrap();
            let before_send = time::precise_time_ns();
            c.send(GuiCommand::Redraw).unwrap();
            let after_send = time::precise_time_ns();
            println!("#sirver before_send: {:#?},after_send: {:#?},diff: {:#?}", before_send, after_send, (after_send - before_send));
            Some(())
        });
    }
}

// NOCOM(#sirver): reconsider if the client takes ownership of the RPCs. If it would not, handing
// out references to the RPCs accessing their owners would be much simpler. On the other hand the
// lieftimes of the RPCs could be non-expressible inside the client. Maybe if the client would take
// weak_refs?
struct OnBufferCreated {
    buffer_views: Arc<RwLock<BufferViews>>,
    sender: client::Sender,
}

impl client::RemoteProcedure for OnBufferCreated {
    fn call(&mut self, mut sender: client::RpcSender, args: json::Value) {
        let info: plugin_buffer::BufferCreated = try_rpc!(sender, json::from_value(args));

        let mut rpc = self.sender.call("buffer.get_content", &plugin_buffer::GetContentRequest {
            buffer_index: info.buffer_index,
        });
        match rpc.wait().unwrap() {
            ipc::RpcResult::Ok(value) => {
                let response: plugin_buffer::GetContentResponse = json::from_value(value).unwrap();
                let mut buffer_views = self.buffer_views.write().unwrap();
                buffer_views.get_or_create(info.buffer_index)
                    .set_contents(&response.content);
            }
            _ => {},
        }
        sender.finish(ipc::RpcResult::success(()))
    }
}
