#[macro_use]
extern crate clap;
extern crate rustbox;
extern crate serde_json;
extern crate subsequence_match;
extern crate swiboe;
extern crate swiboe_gui as gui;
extern crate time;
extern crate uuid;

use gui::buffer_views;
use rustbox::Key;
use rustbox::{Color, RustBox};
use std::cmp;
use std::env;
use std::path;
use std::sync::mpsc;
use swiboe::client;
use uuid::Uuid;

fn clamp<T: Copy + cmp::Ord + std::fmt::Debug>(min: T, max: T, v: &mut T) {
    let new_value = cmp::min(max, cmp::max(min, *v));
    *v = new_value;
}

struct CompleterWidget {
    candidates: subsequence_match::CandidateSet,
    rpc: Option<client::rpc::client::Context>,
    query: String,
    results: Vec<subsequence_match::QueryResult>,
    selection_index: isize,
}

enum CompleterState {
    Running,
    Canceled,
    Selected(String),
}

impl CompleterWidget {
    fn new(client: &client::Client) -> Self {

        // TODO(sirver): This should use the current work directory of the server, since the server
        // might run on a different machine than the client - and certainly in a different
        // directory.
        let current_dir = env::current_dir().unwrap();

        let rpc = client.call("list_files", &swiboe::plugin_list_files::ListFilesRequest {
            directory: current_dir.to_string_lossy().into_owned(),
        });

        CompleterWidget {
            candidates: subsequence_match::CandidateSet::new(),
            rpc: Some(rpc),
            query: "".into(),
            results: Vec::new(),
            selection_index: 0,
        }
    }

    fn on_key(&mut self, key: rustbox::Key) -> CompleterState {
        match key {
            rustbox::Key::Char(c) => {
                self.query.push(c);
                self.results.clear();
                CompleterState::Running
            },
            rustbox::Key::Backspace => {
                self.query.pop();
                self.results.clear();
                CompleterState::Running
            },
            rustbox::Key::Down => {
                self.selection_index += 1;
                CompleterState::Running
            },
            rustbox::Key::Up => {
                self.selection_index -= 1;
                CompleterState::Running
            },
            rustbox::Key::Esc => {
                self.rpc.take().unwrap().cancel().unwrap();
                CompleterState::Canceled
            },
            rustbox::Key::Enter => {
                self.rpc.take().unwrap().cancel().unwrap();
                if self.results.is_empty() {
                    CompleterState::Canceled
                } else {
                    clamp(0, self.results.len() as isize - 1, &mut self.selection_index);
                    CompleterState::Selected(self.results[self.selection_index as usize].text.clone())
                }
            }
            _ => CompleterState::Running,
        }
    }

    fn draw(&mut self, rustbox: &rustbox::RustBox) {
        while let Some(b) = self.rpc.as_mut().unwrap().try_recv().unwrap() {
            self.results.clear();
            let b: swiboe::plugin_list_files::ListFilesUpdate = serde_json::from_value(b).unwrap();
            for file in &b.files {
                self.candidates.insert(file);
            }
        }

        if self.results.is_empty() {
            let query_to_use: String = self.query.chars().filter(|c| !c.is_whitespace()).collect();
            self.candidates.query(&query_to_use, subsequence_match::MatchCase::No, &mut self.results);
        }
        if !self.results.is_empty() {
            clamp(0, self.results.len() as isize - 1, &mut self.selection_index);
        }

        rustbox.print(0, 0, rustbox::RB_BOLD, Color::Yellow, Color::Default, &self.query);
        let len_string = format!("{}/{} matching ({})", self.results.len(), self.candidates.len(),
            if self.rpc.as_ref().unwrap().done() { "done" } else { "scanning" } );
        rustbox.print(rustbox.width() - len_string.len() - 1, 0, rustbox::RB_BOLD, Color::Blue, Color::Default, &len_string);


        let mut row = 1usize;
        for result in &self.results {
            let mut matching_indices = result.matching_indices.iter().peekable();
            for (col, c) in result.text.chars().enumerate() {
                let matches = match matching_indices.peek() {
                    Some(val) if **val == col => true,
                    _ => false,
                };

                let mut style = if matches {
                    matching_indices.next();
                    rustbox::RB_BOLD
                } else {
                    rustbox::RB_NORMAL
                };
                if row as isize == self.selection_index + 1 {
                    style = style | rustbox::RB_REVERSE;
                }

                rustbox.print_char(col, row, style, Color::Default, Color::Default, c);
            }

            row += 1;
            if row > rustbox.height() {
                break;
            }
        }
    }
}

struct BufferViewWidget {
    view_id: String,
    client: client::ThinClient,
    cursor_id: String,
}

impl BufferViewWidget {
    pub fn new(view_id: String, client: client::ThinClient) -> Self {
        BufferViewWidget {
            view_id: view_id,
            client: client,
            cursor_id: String::new(),
        }
    }

    fn draw(&mut self, buffer_view: &buffer_views::BufferView, rustbox: &rustbox::RustBox) {
        let mut row = 0;
        let top_line_index = buffer_view.top_line_index as usize;
        self.cursor_id = buffer_view.cursor.id().to_string();

        let mut cursor_drawn = false;
        while row < rustbox.height() {
            let line_index = top_line_index + row;
            if let Some(line) = buffer_view.lines.get(line_index) {
                for (col, c) in line.chars().enumerate() {
                    if col >= rustbox.width() {
                        break;
                    }
                    let bg = if buffer_view.cursor.position.line_index == line_index as isize &&
                        buffer_view.cursor.position.column_index as usize == col {
                        cursor_drawn = true;
                        Color::Red
                    } else {
                        Color::Default
                    };
                    rustbox.print_char(col, row, rustbox::RB_NORMAL, Color::Default, bg, c);
                }
            }
            row += 1;
        }

        if !cursor_drawn {
            let row = buffer_view.cursor.position.line_index - top_line_index as isize;
            rustbox.print_char(buffer_view.cursor.position.column_index as usize,
                               row as usize, rustbox::RB_NORMAL,
                               Color::Default, Color::Red, ' ');
        }
    }


    fn on_key(&mut self, key: rustbox::Key) {
        if self.cursor_id.is_empty() {
            return;
        }

        match key {
            rustbox::Key::Up => {
                self.client.call("gui.buffer_view.move_cursor", &buffer_views::MoveCursorRequest {
                    cursor_id: self.cursor_id.clone(),
                    delta: buffer_views::Position { line_index: -1, column_index: 0, },
                });
            },
            rustbox::Key::Down => {
                self.client.call("gui.buffer_view.move_cursor", &buffer_views::MoveCursorRequest {
                    cursor_id: self.cursor_id.clone(),
                    delta: buffer_views::Position { line_index: 1, column_index: 0, },
                });
            }
            rustbox::Key::Left => {
                self.client.call("gui.buffer_view.move_cursor", &buffer_views::MoveCursorRequest {
                    cursor_id: self.cursor_id.clone(),
                    delta: buffer_views::Position { line_index: 0, column_index: -1, },
                });
            },
            rustbox::Key::Right => {
                self.client.call("gui.buffer_view.move_cursor", &buffer_views::MoveCursorRequest {
                    cursor_id: self.cursor_id.clone(),
                    delta: buffer_views::Position { line_index: 0, column_index: 1, },
                });
            },
            _ => (),
        }
    }
}

fn main() {
    let matches = clap::App::new("term_gui")
        .about("Terminal client for Swiboe")
        .version(&crate_version!()[..])
        .arg(clap::Arg::with_name("SOCKET")
             .short("s")
             .long("socket")
             .help("Socket at which the master listens.")
             .required(true)
             .takes_value(true))
        .get_matches();

    let rustbox = match RustBox::init(rustbox::InitOptions {
        input_mode: rustbox::InputMode::Current,
        buffer_stderr: true,
    }) {
        Result::Ok(v) => v,
        Result::Err(e) => panic!("{}", e),
    };


    let path = path::Path::new(matches.value_of("SOCKET").unwrap());
    let client = client::Client::connect(path).unwrap();

    let gui_id: String = Uuid::new_v4().to_hyphenated_string();
    let (gui_commands_tx, gui_commands_rx) = mpsc::channel();
    let buffer_views = gui::buffer_views::BufferViews::new(&gui_id, gui_commands_tx, &client);


    let mut completer: Option<CompleterWidget> = None;
    let mut buffer_view_widget: Option<BufferViewWidget> = None;
    loop {
        match rustbox.peek_event(time::Duration::milliseconds(5), false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                if completer.is_some() {
                    let rv = completer.as_mut().unwrap().on_key(key.unwrap());
                    match rv {
                        CompleterState::Running => (),
                        CompleterState::Canceled => {
                            completer = None;
                        },
                        CompleterState::Selected(result) => {
                            completer = None;

                            let mut rpc = client.call("buffer.open", &swiboe::plugin_buffer::OpenRequest {
                                uri: format!("file://{}", result),
                            });
                            let response: swiboe::plugin_buffer::OpenResponse = rpc.wait_for().unwrap();

                            let mut buffer_views = buffer_views.write().unwrap();
                            let view_id = buffer_views.new_view(response.buffer_index, rustbox.width(), rustbox.height());
                            buffer_view_widget = Some(BufferViewWidget::new(view_id, client.clone()));
                        },
                    }
                } else {
                    match key {
                        Some(Key::Char('q')) => break,
                        Some(Key::Ctrl('t')) => {
                            completer = Some(CompleterWidget::new(&client))
                        },
                        _ => {
                            if let Some(ref mut widget) = buffer_view_widget {
                                widget.on_key(key.unwrap());
                            }
                        }
                    }
                }
            },
            Err(e) => panic!("{}", e),
            _ => { }
        }

        while let Ok(command) = gui_commands_rx.try_recv() {
            match command {
                gui::command::GuiCommand::Quit => break,
                gui::command::GuiCommand::Redraw => {
                },
            }
        }

        rustbox.clear();
        if let Some(ref mut widget) = buffer_view_widget {
            let buffer_views = buffer_views.read().unwrap();
            let buffer_view = buffer_views.get(&widget.view_id).unwrap();
            widget.draw(&buffer_view, &rustbox);
        }
        if let Some(ref mut completer) = completer {
            completer.draw(&rustbox);
        }
        rustbox.present();
    }
}
