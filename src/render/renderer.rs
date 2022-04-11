use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

use tui::backend::Backend;
use tui::widgets::{Block, Paragraph};
use tui::Frame;
use tui::text::{Span, Spans};
use tui::layout::{Rect, Alignment};

use std::convert::TryFrom;

use crate::dsp::{DspData};

// pub struct AsyncRendererData<T : DspData> {
//     rendered : bool,
//     pub data: Option<T>,
//     rendered_rx: Receiver<bool>,
//     process_handle: Option<JoinHandle<T>>
// }

// impl<T : DspData + Send + 'static> AsyncRendererData<T> {
//     pub fn update_status(&mut self) -> bool {
//         if !self.rendered {
//             match self.rendered_rx.try_recv() {
//                 Ok(true) => {
//                     self.rendered = true;
//                     self.load_results();
//                     return true;
//                 },
//                 _ => { return false; }
//             }
//         }
//         return false;
//     }

//     pub fn rendered(&mut self) -> bool {
//         self.rendered
//     }

//     pub fn data(&mut self) -> Option<&mut T> {
//         self.data.as_mut()
//     }

//     fn load_results(&mut self) {
//         if self.rendered {
//             let opt_handle = self.process_handle.take();
//             match opt_handle {
//                 Some(handle) => {
//                     self.data = Some(handle.join().expect("Async rendering failed"));
//                 },
//                 None => panic!("Async rendering handle is None")
//             }
//         }
//     }

//     pub fn new(path: &std::path::PathBuf) -> Self {
//         let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
//             .from_path(path).expect("Could not open wave file");
//         if !snd.is_seekable() {
//             panic!("Input file is not seekable");
//         }

//         let (rendered_tx, rendered_rx) = mpsc::channel();
//         let join_handle = thread::spawn(move || {
//             let data = T::new(snd);
//             // Send rendered signal
//             let _ = rendered_tx.send(true);
//             data
//         });

//         Self {
//             rendered: false,
//             data: None,
//             rendered_rx,
//             process_handle: Some(join_handle)
//         }
//     }
// }

pub trait Renderer {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block<'_>);

    fn needs_redraw(&mut self) -> bool;
}

pub fn draw_loading<B : Backend>(frame: &mut Frame<'_, B>, area : Rect, block: Block<'_>) {
    let num_lines_to_center: usize = if area.height % 2 == 0 {
        usize::try_from(area.height).unwrap() / 2 - 1
    } else {
        usize::try_from(area.height).unwrap() / 2
    };

    let mut span_vec = vec![Spans::from(""); num_lines_to_center];
    span_vec[num_lines_to_center - 1] = Spans::from("Loading...");

    let paragraph = Paragraph::new(span_vec)
        .block(block)
        .alignment(Alignment::Center);
    
    frame.render_widget(paragraph, area);
}
