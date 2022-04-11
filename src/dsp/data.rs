use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

extern crate sndfile;
use crate::sndfile::SndFile;

pub trait DspData {
    fn new(file: SndFile) -> Self;
}

pub struct AsyncDspData<T : DspData + Send + 'static> {
    rendered : bool,
    pub data: Option<T>,
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<T>>
}

impl<T : DspData + Send + 'static> AsyncDspData<T> {
    pub fn update_status(&mut self) -> bool {
        if !self.rendered {
            match self.rendered_rx.try_recv() {
                Ok(true) => {
                    self.rendered = true;
                    self.load_results();
                    return true;
                },
                _ => { return false; }
            }
        }
        return false;
    }

    pub fn rendered(&mut self) -> bool {
        self.rendered
    }

    pub fn data(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    fn load_results(&mut self) {
        if self.rendered {
            let opt_handle = self.process_handle.take();
            match opt_handle {
                Some(handle) => {
                    self.data = Some(handle.join().expect("Async rendering failed"));
                },
                None => panic!("Async rendering handle is None")
            }
        }
    }

    pub fn new(path: &std::path::PathBuf) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }

        let (rendered_tx, rendered_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            let data = T::new(snd);
            // Send rendered signal
            let _ = rendered_tx.send(true);
            data
        });

        Self {
            rendered: false,
            data: None,
            rendered_rx,
            process_handle: Some(join_handle)
        }
    }
}