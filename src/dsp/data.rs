use core::fmt;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

extern crate sndfile;
use crate::sndfile::SndFile;

pub struct DspErr {
    msg: String
}

impl DspErr {
    pub fn new(msg: &str) -> Self {
        Self{
            msg: msg.to_string()
        }
    }
}

impl fmt::Display for DspErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

pub trait DspData<P> {
    fn new(file: SndFile, parameter: P) -> Result<Self, DspErr> where Self: Sized;
}


pub struct AsyncDspData<T : DspData<P> + Send + 'static, P : Send + 'static> {
    rendered : bool,
    pub data: Option<T>,
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<T>>,
    phantom : PhantomData<P>
}

impl<T : DspData<P> + Send + 'static, P : Send + 'static> AsyncDspData<T, P> {
    pub fn update_status(&mut self) -> bool {
        if !self.rendered {
            match self.rendered_rx.try_recv() {
                Ok(true) => {
                    // Rendered properly
                    self.rendered = true;
                    self.load_results();
                    return true;
                },
                Ok(false) => {
                    // Failed to render, try to join to catch error
                    let opt_handle = self.process_handle.take();
                    match opt_handle {
                        Some(handle) => {
                            match handle.join() {
                                Ok(_) => panic!("Async rendering sent failed signal but succeeded"),
                                Err(err) => panic!("{:?}", err)
                            }
                        },
                        None => panic!("Async rendering handle is None")
                    }
                }
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

    pub fn new(path: &std::path::PathBuf, parameters: P) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }

        let (rendered_tx, rendered_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            let res = T::new(snd, parameters);

            match res {
                Ok(data) => {
                    let _ = rendered_tx.send(true);
                    return data;
                },
                Err(dsp_err) => {
                    let _ = rendered_tx.send(false);
                    panic!("{}", dsp_err);
                }
            }
        });

        Self {
            rendered: false,
            data: None,
            rendered_rx,
            process_handle: Some(join_handle),
            phantom: PhantomData::default()
        }
    }
}