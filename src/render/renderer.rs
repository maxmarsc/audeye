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

pub trait Renderer {
    // fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block<'_>);
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, activated_channels: &Vec<(usize, &str)>, area : Rect);

    // fn render_channel<B : Backend>(&mut self, _: &mut Frame<'_, B>, _: usize, _ : Rect, _: Block<'_>) {
    //     // default blank implementation
    // }

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

fn compute_channels_layout(area: Rect, num_channels: usize) -> Vec<Rect> {
    let constraints = vec![Constraint::Ratio(1u32, u32::try_from(num_channels).unwrap()); num_channels];
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.as_ref())
        .split(area)
}

pub fn draw_activated_channels<B : Backend>(
        frame: &mut Frame<'_, B>,
        area: Rect,
        activated_channels: &Vec<(usize, &str)>,
        single_channel_draw_fn: &mut dyn FnMut(&mut Frame<'_, B>, usize, Rect, Block)) {
    let layout = compute_channels_layout(area, activated_channels.len());
    
    for (activated_idx, (ch_idx, title)) in activated_channels.iter().enumerate() {
        let block = Block::default().title(*title).borders(Borders::ALL);
        single_channel_draw_fn(frame, *ch_idx, layout[activated_idx], block);
    }
}