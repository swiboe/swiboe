use ::command::GuiCommand;
use serde_json;
use std::clone::Clone;
use std::cmp;
use std::collections::HashMap;
use std::convert;
use std::ops;
use std::sync::mpsc;
use std::sync::{RwLock, Arc, Mutex};
use switchboard::client;
use switchboard::plugin_buffer;
use switchboard::rpc;
use time;
use uuid::Uuid;

// NOCOM(#sirver): nothing in this file is tested .
#[derive(Debug)]
pub enum BufferViewError {
    UnknownCursor,
}

impl From<BufferViewError> for rpc::Error {
     fn from(error: BufferViewError) -> Self {
         use switchboard::rpc::ErrorKind::*;

         let (kind, details) = match error {
             BufferViewError::UnknownCursor => (InvalidArgs, format!("unknown_cursor")),
         };

         rpc::Error {
             kind: kind,
             details: Some(serde_json::to_value(&details)),
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

impl client::rpc::server::Rpc for Scroll {
    fn call(&mut self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: ScrollRequest = try_rpc!(context, serde_json::from_value(args));

        let mut buffer_views = self.buffer_views.write().unwrap();
        buffer_views.scroll(&request.buffer_view_id, request.delta);

        let response = ScrollResponse;
        context.finish(rpc::Result::success(response)).unwrap();
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

impl client::rpc::server::Rpc for MoveCursor {
    fn call(&mut self,  mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: MoveCursorRequest = try_rpc!(context, serde_json::from_value(args));

        let mut buffer_views = self.buffer_views.write().unwrap();

        try_rpc!(context, buffer_views.move_cursor(&request.cursor_id, request.delta));
        context.finish(rpc::Result::success(MoveCursorResponse)).unwrap();
    }
}

pub struct BufferView {
    id: String,
    pub cursor: Cursor,
    pub width: usize,
    pub height: usize,
    pub top_line_index: isize,
    pub lines: Vec<String>,
}

impl BufferView {
    pub fn new(width: usize, height: usize, content: &str) -> Self {
        BufferView {
            id: Uuid::new_v4().to_hyphenated_string(),
            top_line_index: 0,
            width: width,
            height: height,
            lines: content.split("\n").map(|s| s.into()).collect(),
            cursor: Cursor::new(),
        }
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
    buffer_views: HashMap<String, BufferView>,
    client: client::ThinClient,
    // NOCOM(#sirver): are these mutex really needed?
    commands: Mutex<mpsc::Sender<GuiCommand>>,
}

impl BufferViews {
    pub fn new(gui_id: &str, commands: mpsc::Sender<GuiCommand>, client: &client::Client) -> Arc<RwLock<Self>> {
        let buffer_view = Arc::new(RwLock::new(BufferViews {
            gui_id: gui_id.to_string(),
            buffer_views: HashMap::new(),
            client: client.clone(),
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

        buffer_view
    }

    // NOCOM(#sirver): write tests for move cursor.
    fn move_cursor(&mut self, id: &str, delta: Position) -> Result<(), BufferViewError> {
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

    pub fn new_view(&mut self, buffer_index: usize, width: usize, height: usize) -> String {
        let mut rpc = self.client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
            buffer_index: buffer_index,
        });

        let response: plugin_buffer::GetContentResponse = rpc.wait_for().unwrap();
        let buffer_view = BufferView::new(width, height, &response.content);
        let view_id = buffer_view.id().to_string();
        self.buffer_views.insert(buffer_view.id().to_string(), buffer_view);
        view_id
    }


    pub fn get(&self, id: &str) -> Option<&BufferView> {
        self.buffer_views.get(id)
    }

    fn scroll(&mut self, buffer_view_id: &str, delta: isize) {
        self.buffer_views.get_mut(buffer_view_id).and_then(|view| {
            Some(view.scroll(delta))
        }).and_then(|_| {
            let c = self.commands.lock().unwrap();
            c.send(GuiCommand::Redraw).unwrap();
            Some(())
        });
    }
}
