use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

use image::imageops::vertical_gradient;
use tui::backend::Backend;
use tui::widgets::{Block, Paragraph, Borders};
use tui::Frame;
use tui::text::{Span, Spans};
use tui::layout::{Rect, Alignment, Constraint, Layout, Direction};

use std::convert::TryFrom;

use super::ChannelsTabs;


pub struct RenderingInfo<'a> {
    pub activated_channels: Vec<(usize, &'a str)>
}

pub trait Renderer {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, info: &RenderingInfo, area : Rect);

    fn needs_redraw(&mut self) -> bool;
}

pub trait ChannelRenderer : Renderer {
    fn draw_single_channel<B: Backend>(&mut self, frame: &mut Frame<'_, B>, channel: usize, area: Rect, block: Block);

    fn needs_redraw(&mut self) -> bool;
}

impl<T : ChannelRenderer> Renderer for T {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, info: &RenderingInfo, area : Rect) {
        let layout = compute_channels_layout(area, info.activated_channels.len());
    
        for (activated_idx, (ch_idx, title)) in info.activated_channels.iter().enumerate() {
            let block = Block::default().title(*title).borders(Borders::ALL);
            self.draw_single_channel(frame, *ch_idx, layout[activated_idx], block);
        }
    }

    fn needs_redraw(&mut self) -> bool {
        ChannelRenderer::needs_redraw(self)
    }
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

fn compute_channels_layout(area: Rect, num_channels: usize) -> Vec<Rect> {
    let constraints = vec![Constraint::Ratio(1u32, u32::try_from(num_channels).unwrap()); num_channels];
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.as_ref())
        .split(area)
}