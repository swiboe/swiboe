use ::client;
use ::rpc;
use serde::json;
use std::convert;
use std::fs::{self, DirEntry};
use std::io;
use std::mem;
use std::path::Path;
use std::path;
use std::string;
use std::thread;
use time;

// NOCOM(#sirver): rewrite
// one possible implementation of fs::walk_dir only visiting files
fn visit_dirs(dir: &Path, cb: &mut FnMut(&DirEntry)) -> io::Result<()> {
    if try!(fs::metadata(dir)).is_dir() {
        for entry in try!(fs::read_dir(dir)) {
            let entry = try!(entry);
            let filetype = try!(entry.file_type());
            if filetype.is_dir() && !filetype.is_symlink() {
                try!(visit_dirs(&entry.path(), cb));
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
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

impl client::RemoteProcedure for ListFiles {
    fn call(&mut self, mut sender: client::RpcServerContext, args: json::Value) {
        let request: ListFilesRequest = try_rpc!(sender, json::from_value(args));

        thread::spawn(move || {
            let mut files = Vec::new();
            let mut last = time::SteadyTime::now();
            visit_dirs(Path::new(&request.directory), &mut |entry| {
                files.push(entry.path().to_string_lossy().into_owned());
                let now = time::SteadyTime::now();
                if now - last > time::Duration::milliseconds(50) {
                    last = now;
                    sender.update(&ListFilesUpdate {
                        files: mem::replace(&mut files, Vec::new())
                    });
                }
            });

            let response = ListFilesResponse;
            sender.finish(rpc::Result::success(response));
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
