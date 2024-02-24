use std::convert::TryFrom;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::text::Spans;
use tui::widgets::{Block, Borders, Paragraph};
use tui::Frame;

use crate::utils::Zoom;

pub struct RenderingInfo<'a> {
    pub activated_channels: Vec<(usize, &'a str)>,
    pub zoom: &'a Zoom,
}

pub trait Renderer {
    fn draw<B: Backend>(&mut self, frame: &mut Frame<'_, B>, info: &RenderingInfo, area: Rect);

    fn needs_redraw(&mut self) -> bool;
    fn max_width_resolution(&self) -> usize {
        usize::MAX
    }
}

pub trait ChannelRenderer: Renderer {
    fn draw_single_channel<B: Backend>(
        &mut self,
        frame: &mut Frame<'_, B>,
        channel: usize,
        area: Rect,
        block: Block,
        zoom: &Zoom,
    );

    fn needs_redraw(&mut self) -> bool;
    fn max_width_resolution(&self) -> usize;
}

impl<T: ChannelRenderer> Renderer for T {
    fn draw<B: Backend>(&mut self, frame: &mut Frame<'_, B>, info: &RenderingInfo, area: Rect) {
        let layout = compute_channels_layout(area, info.activated_channels.len());

        for (activated_idx, (ch_idx, title)) in info.activated_channels.iter().enumerate() {
            let block = Block::default().title(*title).borders(Borders::ALL);
            self.draw_single_channel(frame, *ch_idx, layout[activated_idx], block, info.zoom);
        }
    }

    fn needs_redraw(&mut self) -> bool {
        ChannelRenderer::needs_redraw(self)
    }

    fn max_width_resolution(&self) -> usize {
        ChannelRenderer::max_width_resolution(self)
    }
}

pub fn draw_text_info<B: Backend>(
    frame: &mut Frame<'_, B>,
    area: Rect,
    block: Block<'_>,
    text: &str,
) {
    let num_lines_to_center: usize = if area.height % 2 == 0 {
        usize::try_from(area.height).unwrap() / 2 - 1
    } else {
        usize::try_from(area.height).unwrap() / 2
    };

    let mut span_vec = vec![Spans::from(""); num_lines_to_center];
    span_vec[num_lines_to_center - 1] = Spans::from(text);

    let paragraph = Paragraph::new(span_vec)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn compute_channels_layout(area: Rect, num_channels: usize) -> Vec<Rect> {
    let constraints =
        vec![Constraint::Ratio(1u32, u32::try_from(num_channels).unwrap()); num_channels];
    Layout::default()
        .direction(Direction::Vertical)
        .constraints::<&[Constraint]>(constraints.as_ref())
        .split(area)
}
