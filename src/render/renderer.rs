use std::sync::mpsc::{Receiver, self};
use std::thread::{JoinHandle, self};

use tui::backend::Backend;
use tui::widgets::{Block, Paragraph};
use tui::Frame;
use tui::text::{Span, Spans};
use tui::layout::{Rect, Alignment};

use std::convert::TryFrom;

pub trait Renderer {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block<'_>);

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

// pub render_channel()