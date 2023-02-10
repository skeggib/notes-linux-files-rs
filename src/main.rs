pub mod model;
pub mod app;

use model::{Client, Comm, SingleConnectionServer};
use model::{Model, Note};
use app::Application;

use serde::Deserialize;
use std::fs::{OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::{process::exit};
use std::{thread};

fn main() {
    let workspace_path_string = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let instance_kind_string = std::env::args()
        .nth(2)
        .expect("Argument 2 needs to be either 'server' or 'client'");

    let mut workspace_path = PathBuf::new();
    workspace_path.push(workspace_path_string);

    // create server or client
    let address = "127.0.0.1:55000";
    let comm = match instance_kind_string.as_str() {
        "server" => match SingleConnectionServer::new(address) {
            Ok(server) => Comm::ServerKind(server),
            Err(error) => {
                eprintln!("cannot create server -> {}", error);
                exit(1)
            }
        },
        "client" => match Client::new(address) {
            Ok(client) => Comm::ClientKind(client),
            Err(error) => {
                eprintln!("cannot create client -> {}", error);
                exit(1)
            }
        },
        _ => {
            eprintln!("invalid instance kind '{}'", instance_kind_string);
            exit(1)
        }
    };

    // at this point, client and server are connected

    // create model
    let mut model = match comm {
        Comm::ServerKind(ref server) => {
            let model = Model::new();
            println!("send model to client");
            match serde_json::to_writer(server.as_writer(), &model) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("cannot write model to stream -> {}", error);
                    exit(1)
                }
            };
            model
        }
        Comm::ClientKind(ref client) => {
            println!("receive model from server");
            let mut de = serde_json::Deserializer::from_reader(&client.stream);
            match Model::deserialize(&mut de) {
                Ok(model) => model,
                Err(error) => {
                    eprintln!("cannot read model from stream -> {}", error);
                    exit(1)
                }
            }
        }
    };

    let app = Application {
        comm,
        model,
        workspace_path
    };

    println!("initialize workspace");
    match app.destroy_workspace() {
        Ok(_) => {}
        Err(error) => {
            eprintln!("cannot destroy workspace -> {}", error);
            exit(1)
        }
    };
    match app.init_workspace() {
        Ok(_) => {}
        Err(error) => {
            eprintln!("cannot init workspace -> {}", error);
            exit(1)
        }
    };

    println!("watch workspace...");
    match comm {
        Comm::ServerKind(ref server) => {
            match app.watch_workspace() {
                Ok(watch_receiver) => loop {
                    match watch_receiver.recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                            match serde_json::to_writer(server.as_writer(), &model) {
                                Ok(_) => {}
                                Err(error) => {
                                    eprintln!("cannot write model to stream -> {}", error);
                                    exit(1)
                                }
                            };
                        }
                        Err(_) => {}
                    };
                },
                Err(error) => {
                    eprintln!("cannot watch workspace -> {}", error);
                    exit(1)
                }
            };
        }
        Comm::ClientKind(client) => {
            let (stream_sender, stream_receiver) = channel();
            thread::spawn(move || loop {
                let mut de = serde_json::Deserializer::from_reader(&client.stream);
                match Model::deserialize(&mut de) {
                    Ok(model) => match stream_sender.send(model) {
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("cannot send model -> {}", error);
                        }
                    },
                    Err(error) => {
                        eprintln!("cannot read model from stream -> {}", error);
                    }
                }
            });
            match app.watch_workspace() {
                Ok(watch_receiver) => loop {
                    match watch_receiver.try_recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                        }
                        Err(_) => {}
                    }
                    match stream_receiver.try_recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                            update_workspace(&workspace_path, &model);
                        }
                        Err(_) => {}
                    }
                },
                Err(error) => {
                    eprintln!("cannot watch workspace -> {}", error);
                    exit(1)
                }
            }
        }
    }
}

fn update_workspace(workspace_path: &Path, model: &Model) -> () {
    model
        .notes
        .iter()
        .for_each(|(path, note)| update_node(workspace_path.join(path).as_path(), note));
}

fn update_node(path: &Path, note: &Note) -> () {
    if path.exists() {
        match OpenOptions::new().write(true).open(path) {
            Ok(mut file) => match file.set_len(0) {
                Ok(()) => match writeln!(file, "{}\n\n{}", note.title, note.body) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!("could not write file '{:?}': {}", path, error);
                    }
                },
                Err(error) => {
                    eprintln!("could not clear file '{:?}': {}", path, error);
                }
            },
            Err(error) => {
                eprintln!("cannot update existing note -> {}", error)
            }
        }
    } else {
    }
}
