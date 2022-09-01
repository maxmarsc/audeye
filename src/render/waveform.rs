use super::{draw_text_info, renderer::ChannelRenderer};
use core::panic;
extern crate sndfile;
use crate::utils::Zoom;
use tui::backend::{Backend};
use tui::layout::Rect;
use tui::symbols::Marker;
use tui::widgets::canvas::{Canvas, Line, Context};
use tui::{
    widgets::{Block},
    style::{Color},
    Frame
};
use std::convert::TryFrom;

use crate::dsp::{Waveform, AsyncDspData, WaveformParameters, AsyncDspDataState, WaveformPoint};

// fn draw_outlined_shape(ctx: &mut Context, n_int: &Vec<i32>, p_int: &Vec<i32>) {
//     let mut previous_idx = 0usize;
//     let (mut prev_n, mut prev_p) = (0f64, 0f64);
//     for (idx, (n,p)) in n_int.iter().zip(p_int.iter()).enumerate() {
//         if idx != 0 {
//             // draw positive line
//             ctx.draw(&Line{
//                 x1: previous_idx as f64,
//                 y1: prev_p,
//                 x2: idx as f64,
//                 y2: *p as f64,
//                 color: Color::White
//             });

//             // draw negative line
//             ctx.draw(&Line{
//                 x1: previous_idx as f64,
//                 y1: prev_n,
//                 x2: idx as f64,
//                 y2: *n as f64,
//                 color: Color::White
//             });
//         }
//         previous_idx = idx;
//         prev_n = *n as f64;
//         prev_p = *p as f64;
//     }
// }

// fn draw_filled_shape(ctx: &mut Context, n_int: &Vec<i32>, p_int: &Vec<i32>) {
//     for (idx, (n,p)) in n_int.iter().zip(p_int.iter()).enumerate() {
//         ctx.draw(&Line{
//             x1: idx as f64,
//             x2: idx as f64,
//             y1: *n as f64,
//             y2: *p as f64,
//             color: Color::White
//         });
//     }
// }

fn draw_shape(ctx: &mut Context, points: &[WaveformPoint<i32>]) {
    let mut prev_peak_up = 0f64;
    let mut prev_peak_down = 0f64;

    for (idx, points) in points.iter().enumerate() {
        // Draw inner RMS shape
        ctx.draw(&Line{
            x1: idx as f64,
            x2: idx as f64,
            y1: -(points.rms as f64),
            y2: points.rms as f64,
            color: Color::White
        });

        if idx != 0usize {
            // Draw top and low peaks
            ctx.draw(&Line{
                x1: idx as f64 - 1f64,
                x2: idx as f64,
                y1: prev_peak_up,
                y2: points.peak_max as f64,
                color: Color::White
            });

            ctx.draw(&Line{
                x1: idx as f64 - 1f64,
                x2: idx as f64,
                y1: prev_peak_down,
                y2: points.peak_min as f64,
                color: Color::White
            });
        }

        prev_peak_down = points.peak_min as f64;
        prev_peak_up = points.peak_max as f64;
    }
}

pub struct WaveformRenderer {
    channels: usize,
    async_renderer: AsyncDspData<Waveform, WaveformParameters>,
    max_width_res: usize
}

impl WaveformRenderer {
    pub fn new(path: &std::path::PathBuf, normalize: bool) -> WaveformRenderer {
        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).expect("Could not open wave file");
        
        let channels = snd.get_channels();
        let max_res = usize::try_from(snd.len().unwrap()).unwrap();
            
        WaveformRenderer {
            channels,
            async_renderer: AsyncDspData::new(path, WaveformParameters::default(), normalize),
            max_width_res: max_res
        }
    }
}

impl ChannelRenderer for WaveformRenderer {
    fn draw_single_channel<B: Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area: Rect, block: Block, zoom: &Zoom) {
        match self.async_renderer.state() {
            AsyncDspDataState::Normalizing => {
                draw_text_info(frame, area, block, "Normalizing...");
                return;
            },
            AsyncDspDataState::Created | AsyncDspDataState::Processing => {
                draw_text_info(frame, area, block, "Loading...");
                return;
            },
            AsyncDspDataState::Failed => {
                // Should crash soon
                draw_text_info(frame, area, block, "Error");
                return;
            },
            _ => {}
        }

        if channel >= self.channels { panic!(); }

        // Prepare
        let data_ref = self.async_renderer.data().unwrap();
        let canva_width_int = area.width as usize - 2;
        let estimated_witdh_res = canva_width_int * 2;      // Braille res is 2 per char

        // Compute local min & max for each block
        let points = data_ref.compute_points(channel, estimated_witdh_res, zoom);
    
        // Draw the canva
        let canva = Canvas::default()
            .block(block)
            .paint(|ctx| { draw_shape(ctx, &points) })
            .marker(Marker::Braille)
            .x_bounds([-1., estimated_witdh_res as f64 + 1f64])
            .y_bounds([i32::MIN as f64, i32::MAX as f64]);
        
        frame.render_widget(canva, area)
    }

    fn needs_redraw(&mut self) -> bool {
        self.async_renderer.update_status()
    }

    fn max_width_resolution(&self) -> usize {
        self.max_width_res
    }
}


