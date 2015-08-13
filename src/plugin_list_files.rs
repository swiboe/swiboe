use serde::json;
use std::collections::HashMap;
use std::convert;
use std::fs::{self, DirEntry};
use std::io;
use std::ops;
use std::path::Path;
use std::path;
use std::string;
use std::thread;
use super::client;
use super::ipc;
use uuid::Uuid;

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
pub struct ListFilesResult {
    context: String,
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesRequest {
    pub directory: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesResponse {
    pub context: String,
}

struct ListFiles;

impl client::RemoteProcedure for ListFiles {
    fn call(&mut self, mut sender: client::RpcSender, args: json::Value) {
        let request: ListFilesRequest = try_rpc!(sender, json::from_value(args));

        let context = Uuid::new_v4().to_hyphenated_string();

        let context_clone = context.clone();
        thread::spawn(move || {
            // let mut files = Vec::new();
            let mut nfiles = 0;
            visit_dirs(Path::new(&request.directory), &mut |entry| {
                println!("#sirver entry: {:#?}", &entry.path());
                nfiles += 1;
                // files.push(entry.path());
            });
            println!("#sirver nfiles: {:#?}", nfiles);
        });

        let response = ListFilesResponse {
            context: context,
        };
        sender.finish(ipc::RpcResult::success(response));
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
