use crate::render::renderer::Renderer;
use core::panic;
extern crate sndfile;
use crate::sndfile::SndFile;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Receiver, Sender};
use tui::backend::{Backend};
use tui::layout::Rect;
use tui::symbols::Marker;
use tui::widgets::canvas::{Canvas, Line};
use tui::{
    widgets::{Chart, Dataset, GraphType, Block, Borders, Axis},
    symbols,
    style::{Style, Color, Modifier},
    text::Span,
    Frame
};

use crate::dsp::waveform::Waveform;

pub struct WaveformRenderer {
    rendered : bool,
    block_count : usize,
    pub channels: usize,
    data : Option<Waveform>,
    // rendering_buffers: Vec<Vec<
    rendered_rx: Receiver<bool>,
    process_handle: Option<JoinHandle<Waveform>>,
}

impl WaveformRenderer {
    pub fn new(block_count: usize, path: &std::path::PathBuf) -> Self {
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        if !snd.is_seekable() {
            panic!("Input file is not seekable");
        }
        
        let channels = snd.get_channels();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let handle = async_compute(snd, block_count, rendered_tx);
            
        WaveformRenderer {
            rendered: false,
            block_count,
            channels,
            data: None,
            rendered_rx,
            process_handle: Some(handle)
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

    fn render<'a>(&'a self, channel: usize) -> Chart<'a> {
        let data_ref = match self.data.as_ref() {
            Some(data_ref) => data_ref,
            None => panic!()
        };

        let datasets = vec![
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::White))
                .graph_type(GraphType::Line)
                .data(data_ref.p_data(channel)),
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::White))
                .graph_type(GraphType::Line)
                .data(data_ref.n_data(channel)),
            // Dataset::default()
            //     .marker(symbols::Marker::Braille)
            //     .style(Style::default().fg(Color::LightRed))
            //     .graph_type(GraphType::Line)
            //     .data(zero_data.as_ref()),
        ];

        Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!["Channel {:?}", channel],
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
            ).x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, self.block_count as f64]),
            ).y_axis(
                Axis::default()
                    .title("Amplitude")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([-1.0, 1.0])
            )
    }
}

impl Renderer for WaveformRenderer {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area : Rect) {
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

        // if self.rendered && channel < self.channels {
        //     frame.render_widget(self.render(channel), area);
        // }
        if !self.rendered || channel >= self.channels { return; }
        let data_ref = self.data.as_ref().unwrap();
        let canva_width_int = area.width as usize - 2;
        
        let estimated_witdh_res = canva_width_int * 3;
        let (mut n_int, mut p_int) = (vec![0i32; estimated_witdh_res],vec![0i32; estimated_witdh_res]);
        data_ref.compute_min_max(channel, p_int.as_mut_slice(), n_int.as_mut_slice());

        let canva = Canvas::default()
            .block(Block::default().title(format!["Channel {:?}", channel]).borders(Borders::ALL))
            .paint(|ctx| {
                for (idx, (n,p)) in n_int.iter().zip(p_int.iter()).enumerate() {
                    ctx.draw(&Line{
                        x1: idx as f64,
                        x2: idx as f64,
                        y1: *n as f64,
                        y2: *p as f64,
                        color: Color::White
                    })
                }
            })
            .marker(Marker::Braille)
            .x_bounds([-1., estimated_witdh_res as f64 + 1f64])
            .y_bounds([i32::MIN as f64, i32::MAX as f64]);
        
            frame.render_widget(canva, area)

    }
}


fn async_compute(snd: SndFile, block_count: usize, rendered_tx: Sender<bool>) -> JoinHandle<Waveform> {

    thread::spawn(move || {
        let data = Waveform::new(snd, block_count);
    
        // Send rendered signal
        let _ = rendered_tx.send(true);
        
        data
    })
}

