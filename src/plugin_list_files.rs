use ::client;
use ::rpc;
use serde_json;
use std::convert;
use std::fs::{self, DirEntry};
use std::io;
use std::mem;
use std::path::Path;
use std::path;
use std::thread;
use time;

enum Continue {
    Yes,
    No
}

// NOCOM(#sirver): rewrite
// one possible implementation of fs::walk_dir only visiting files
fn visit_dirs(dir: &Path, cb: &mut FnMut(&DirEntry) -> Continue) -> io::Result<Continue> {
    if try!(fs::metadata(dir)).is_dir() {
        for entry in try!(fs::read_dir(dir)) {
            let entry = try!(entry);
            let filetype = try!(entry.file_type());
            if filetype.is_dir() && !filetype.is_symlink() {
                match try!(visit_dirs(&entry.path(), cb)) {
                    Continue::Yes => (),
                    Continue::No => return Ok(Continue::No),
                }
            } else {
                match cb(&entry) {
                    Continue::Yes => (),
                    Continue::No => return Ok(Continue::No),
                }
            }
        }
    }
    Ok(Continue::Yes)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesUpdate {
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesRequest {
    pub directory: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesResponse;

struct ListFiles;

impl client::rpc::server::Rpc for ListFiles {
    fn call(&mut self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: ListFilesRequest = try_rpc!(context, serde_json::from_value(args));

        thread::spawn(move || {
            let mut files = Vec::new();
            let mut last = time::SteadyTime::now();
            visit_dirs(Path::new(&request.directory), &mut |entry| {
                if context.cancelled() {
                    return Continue::No;
                }

                files.push(entry.path().to_string_lossy().into_owned());
                let now = time::SteadyTime::now();
                if now - last > time::Duration::milliseconds(50) {
                    last = now;
                    if context.update(&ListFilesUpdate {
                        files: mem::replace(&mut files, Vec::new())
                    }).is_err() {
                        return Continue::No;
                    };
                }
                Continue::Yes
            }).unwrap();

            // Ignore errors: we might have been cancelled.
            let _ = context.update(&ListFilesUpdate {
                files: mem::replace(&mut files, Vec::new())
            });
            let response = ListFilesResponse;
            let _ = context.finish(rpc::Result::success(response));
        });

    }
}

pub struct ListFilesPlugin<'a> {
    client: client::Client<'a>,
}

impl<'a> ListFilesPlugin<'a> {
    pub fn new(socket_name: &path::Path) -> Self {
        let client = client::Client::connect(socket_name);

        let plugin = ListFilesPlugin {
            client: client,
        };

        let list_files = Box::new(ListFiles);
        plugin.client.new_rpc("list_files", list_files);

        plugin
    }
}
