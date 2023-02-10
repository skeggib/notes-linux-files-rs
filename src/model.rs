use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{remove_dir_all, create_dir, File};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::{fmt};
use string_join::Join;

#[derive(Serialize, Deserialize, Clone)]
pub struct Model {
    pub notes: HashMap<String, Note>,
}

pub enum Comm {
    ServerKind(SingleConnectionServer),
    ClientKind(Client),
}

pub struct SingleConnectionServer {
    listener: TcpListener,
    stream: TcpStream,
}

impl SingleConnectionServer {
    /// Binds the provided address and awaits exactly one connection
    pub fn new(address: &str) -> Result<SingleConnectionServer, String> {
        println!("binding {}", address);
        let listener = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(error) => return Err(format!("cannot bind {} -> {}", address, error)),
        };
        println!("waiting for client...");
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) => return Err(format!("cannot accept incoming connection -> {}", error)),
        };
        println!("connected");
        Ok(SingleConnectionServer { listener, stream })
    }

    pub fn as_writer(self: &Self) -> &TcpStream {
        &self.stream
    }
}

pub struct Client {
    pub stream: TcpStream,
}

impl Client {
    pub fn new(address: &str) -> Result<Client, String> {
        println!("connecting to 127.0.0.1:55000...");
        let stream = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(error) => return Err(format!("Could not connect to {} -> {}", address, error)),
        };
        println!("connected");
        Ok(Client { stream })
    }
}

impl Model {
    pub fn new() -> Model {
        Model {
            notes: HashMap::from([
                (
                    "note_1.txt".to_string(),
                    Note {
                        title: "Example note 1".to_string(),
                        body: "Some text".to_string(),
                    },
                ),
                (
                    "note_2.txt".to_string(),
                    Note {
                        title: "Example note 2".to_string(),
                        body: "Some text\nwith multiple lines".to_string(),
                    },
                ),
            ]),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub title: String,
    pub body: String,
}

impl Note {
    pub fn new() -> Note {
        return Note {
            title: "".to_string(),
            body: "".to_string(),
        };
    }
}

impl fmt::Display for Model {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "=============== Model ===============\n\n{}\n\n=====================================\n",
            "\n\n---------------------\n\n".join(
                self.notes
                    .iter()
                    .map(|element| format!("{:?}\n\n{}", element.0, element.1)),
            )
        )
    }
}

impl fmt::Display for Note {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}\n\n{}", self.title, self.body)
    }
}
