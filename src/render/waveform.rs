use crate::render::renderer::Renderer;
use core::panic;
extern crate sndfile;
use crate::sndfile::SndFile;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Receiver, Sender};
use tui::backend::{Backend};
use tui::layout::Rect;
use tui::symbols::Marker;
use tui::widgets::canvas::{Canvas, Line, Context};
use tui::{
    widgets::{Chart, Dataset, GraphType, Block, Borders, Axis},
    symbols,
    style::{Style, Color, Modifier},
    text::Span,
    Frame
};

use crate::dsp::waveform::Waveform;

fn draw_outlined_shape(ctx: &mut Context, n_int: &Vec<i32>, p_int: &Vec<i32>) {
    let mut previous_idx = 0usize;
    let (mut prev_n, mut prev_p) = (0f64, 0f64);
    for (idx, (n,p)) in n_int.iter().zip(p_int.iter()).enumerate() {
        if idx != 0 {
            // draw positive line
            ctx.draw(&Line{
                x1: previous_idx as f64,
                y1: prev_p,
                x2: idx as f64,
                y2: *p as f64,
                color: Color::White
            });

            // draw negative line
            ctx.draw(&Line{
                x1: previous_idx as f64,
                y1: prev_n,
                x2: idx as f64,
                y2: *n as f64,
                color: Color::White
            });
        }
        previous_idx = idx;
        prev_n = *n as f64;
        prev_p = *p as f64;
    }
}

fn draw_filled_shape(ctx: &mut Context, n_int: &Vec<i32>, p_int: &Vec<i32>) {
    for (idx, (n,p)) in n_int.iter().zip(p_int.iter()).enumerate() {
        ctx.draw(&Line{
            x1: idx as f64,
            x2: idx as f64,
            y1: *n as f64,
            y2: *p as f64,
            color: Color::White
        });
    }
}


pub struct WaveformRenderer {
    rendered : bool,
    pub channels: usize,
    data : Option<Waveform>,
    // rendering_buffers: Vec<Vec<
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<Waveform>>
}

impl WaveformRenderer {
    pub fn new(path: &std::path::PathBuf) -> WaveformRenderer {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }
        
        let channels = snd.get_channels();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let handle = async_compute(snd, rendered_tx);
            
        WaveformRenderer {
            rendered: false,
            channels,
            data: None,
            rendered_rx,
            process_handle: Some(handle),
        }
    }

    fn load_results(&mut self) {
        if self.rendered {
            let opt_handle = self.process_handle.take();
            match opt_handle {
                Some(handle) => {
                    self.data = Some(handle.join().expect("Waveform rendering failed"));
                },
                None => panic!("Waveform rendering handle is None")
            }
        }
    }
}

impl Renderer for WaveformRenderer {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block) {
        // Check for end of rendering
        if !self.rendered {
            match self.rendered_rx.try_recv() {
                Ok(true) => {
                    self.rendered = true;
                    self.load_results();
                },
                _ => ()
            }
        }

        if !self.rendered || channel >= self.channels { return; }

        // Prepare
        let data_ref = self.data.as_ref().unwrap();
        let canva_width_int = area.width as usize - 2;
        let estimated_witdh_res = canva_width_int * 2;      // Braille res is 2 per char

        // Compute local min & max for each block
        let (mut n_int, mut p_int) = (vec![0i32; estimated_witdh_res],vec![0i32; estimated_witdh_res]);
        data_ref.compute_min_max(channel, p_int.as_mut_slice(), n_int.as_mut_slice());

        // Pick drawing method
        let drawing_method = draw_filled_shape;
    
        // Draw the canva
        let canva = Canvas::default()
            .block(block)
            .paint(|ctx| { drawing_method(ctx, &n_int, &p_int); })
            .marker(Marker::Braille)
            .x_bounds([-1., estimated_witdh_res as f64 + 1f64])
            .y_bounds([i32::MIN as f64, i32::MAX as f64]);
        
        frame.render_widget(canva, area)
    }
}


fn async_compute(snd: SndFile, rendered_tx: Sender<bool>) -> JoinHandle<Waveform> {

    thread::spawn(move || {
        let data = Waveform::new(snd);
    
        // Send rendered signal
        let _ = rendered_tx.send(true);
        
        data
    })
}

