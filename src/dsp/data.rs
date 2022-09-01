use core::fmt;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

extern crate sndfile;
use crate::sndfile::SndFile;

use super::normalization::compute_norm;

#[derive(Debug)]
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
    fn new(file: SndFile, parameter: P, normalize: Option<f64>) -> Result<Self, DspErr> where Self: Sized;
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncDspDataState {
    Created,
    Normalizing,
    Processing,
    Failed,
    Finished
}

pub struct AsyncDspData<T : DspData<P> + Send + 'static, P : Send + 'static> {
    state: AsyncDspDataState,
    pub data: Option<T>,
    rendered_rx: Receiver<AsyncDspDataState>,
    process_handle: Option<JoinHandle<T>>,
    phantom : PhantomData<P>
}

impl<T : DspData<P> + Send + 'static, P : Send + 'static> AsyncDspData<T, P> {
    pub fn update_status(&mut self) -> bool {
        let mut update_needed = false;

        loop {
            match self.rendered_rx.try_recv() {
                Ok(AsyncDspDataState::Finished) => {
                    // Rendered properly
                    self.load_results();
                    self.state = AsyncDspDataState::Finished;
                    update_needed = true;
                    break;
                },
                Ok(AsyncDspDataState::Failed) => {
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
                },
                Ok(new_state) => {
                    self.state = new_state;
                    update_needed = true;
                }
                _ => { break; }
            }
        };

        update_needed
    }

    pub fn state(&mut self) -> AsyncDspDataState {
        self.state
    }

    pub fn data(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    fn load_results(&mut self) {
        let opt_handle = self.process_handle.take();
        match opt_handle {
            Some(handle) => {
                self.data = Some(handle.join().expect("Async rendering failed"));
            },
            None => panic!("Async rendering handle is None")
        }
    }

    pub fn new(path: &std::path::PathBuf, parameters: P, normalize: bool) -> Self {
        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }

        let (rendered_tx, rendered_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            // First, compute the norm if needed
            let norm = if normalize {
                let _ = rendered_tx.send(AsyncDspDataState::Normalizing);
                Some(compute_norm(&mut snd))
            } else {
                None
            };

            // Start the processing
            let _ = rendered_tx.send(AsyncDspDataState::Processing);
            let res = T::new(snd, parameters, norm);

            // Check the processing result
            match res {
                Ok(data) => {
                    // Success, we update the state and return the data
                    let _ = rendered_tx.send(AsyncDspDataState::Finished);
                    data
                },
                Err(dsp_err) => {
                    // Failure, we stop the program and display the error
                    let _ = rendered_tx.send(AsyncDspDataState::Failed);
                    panic!("{}", dsp_err);
                }
            }
        });

        Self {
            state: AsyncDspDataState::Created,
            data: None,
            rendered_rx,
            process_handle: Some(join_handle),
            phantom: PhantomData::default()
        }
    }
}
