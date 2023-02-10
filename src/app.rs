use std::fs::{create_dir, read, remove_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use notify::Watcher;
use string_join::Join;

use crate::model::{Comm, Model, Note};

pub struct Application {
    pub comm: Comm,
    pub model: Model,
    workspace_path: PathBuf,
}

impl Application {
    pub fn destroy_workspace(self: &Self) -> Result<(), String> {
        if !self.workspace_path.exists() {
            Ok(())
        } else {
            match remove_dir_all(&self.workspace_path) {
                Ok(_) => Ok(()),
                Err(error) => Err(format!(
                    "cannot delete workspace '{:?}': {}",
                    self.workspace_path, error
                )),
            }
        }
    }

    pub fn init_workspace(self: &Self) -> Result<(), String> {
        let path = Path::new(&self.workspace_path);
        match create_dir(path) {
            Ok(_) => {}
            Err(error) => {
                return Err(format!(
                    "could not create directory '{:?}': {}",
                    self.workspace_path, error
                ))
            }
        };
        for (filename, note) in &self.model.notes {
            let file_path = path.join(filename);
            match File::create(file_path.clone()) {
                Ok(mut file) => match writeln!(file, "{}\n\n{}", note.title, note.body) {
                    Ok(_) => (),
                    Err(error) => {
                        return Err(format!("could not write file '{:?}': {}", file_path, error))
                    }
                },
                Err(error) => {
                    return Err(format!(
                        "could not create file '{:?}': {}",
                        file_path, error
                    ))
                }
            }
        }
        Ok(())
    }

    fn route_notify_event(self: &mut Self, event: notify::Event) -> () {
        match event.kind {
            notify::EventKind::Access(_) => (),
            notify::EventKind::Any => todo!(),
            notify::EventKind::Create(_) => event
                .paths
                .iter()
                .for_each(|path| self.on_create_file(path)),
            notify::EventKind::Modify(kind) => match kind {
                notify::event::ModifyKind::Any => (),
                notify::event::ModifyKind::Data(_) => event
                    .paths
                    .iter()
                    .for_each(|path| self.on_modify_file_data(path)),
                notify::event::ModifyKind::Metadata(_) => (),
                notify::event::ModifyKind::Name(_) => todo!(),
                notify::event::ModifyKind::Other => (),
            },
            notify::EventKind::Other => todo!(),
            notify::EventKind::Remove(_) => todo!(),
        }
    }

    fn on_create_file(self: &mut Self, path: &PathBuf) -> () {
        self.model
            .notes
            .entry(
                path.strip_prefix(&self.workspace_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )
            .or_insert(Note::new());
    }

    fn on_modify_file_data(self: &mut Self, path: &PathBuf) -> () {
        match read_note(&path) {
            Ok(note) => {
                *self
                    .model
                    .notes
                    .entry(
                        path.strip_prefix(&self.workspace_path)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    )
                    .or_insert(Note::new()) = note
            }
            Err(error) => {
                eprintln!("could not read note '{:?}': {}", path, error)
            }
        }
    }

    pub fn watch_workspace(self: &mut Self) -> Result<Receiver<Model>, String> {
        fn event_handler(
            res: Result<notify::Event, notify::Error>,
            app: &mut Application,
        ) -> () {
            match res {
                Ok(event) => app.route_notify_event(event),
                Err(error) => eprintln!("{}", error),
            }
        }

        println!("{}", &self.model);

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(watcher) => watcher,
            Err(error) => return Err(format!("cannot create watcher -> {}", error)),
        };

        let (sender, receiver) = channel();

        let m = self.model.clone();
        let w = self.workspace_path.to_owned();
        thread::spawn(move || {
            match watcher.watch(&w, notify::RecursiveMode::NonRecursive) {
                Ok(_) => rx.iter().for_each(|res| event_handler(res, self)),
                Err(error) => eprintln!("cannot watch '{:?}' -> {}", &w, error),
            };
        });

        Ok(receiver)
    }
}

fn read_note(path: &Path) -> Result<Note, String> {
    match read(&path) {
        Ok(buf) => match str::from_utf8(&buf) {
            Ok(text) => {
                let lines: Vec<&str> = text.split("\n").collect();
                if lines.len() > 1 {
                    Ok(Note {
                        title: lines[0].to_string(),
                        body: "\n".join(lines.iter().skip(1).skip_while(|line| line.is_empty())),
                    })
                } else {
                    Ok(Note {
                        title: "".to_string(),
                        body: text.to_string(),
                    })
                }
            }
            Err(error) => Err(format!("could not read '{:?}' -> {}", path, error)),
        },
        Err(error) => Err(format!("could not read '{:?}' -> {}", path, error)),
    }
}
