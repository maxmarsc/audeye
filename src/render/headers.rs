use std::collections::BTreeSet;

use tui::backend::Backend;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::Frame;
// use tui::widgets::canvas::Canvas;
use tui::widgets::{Block, Borders, Paragraph};
// use tui::widgets::canvas::Rectangle;

// use crate::utils::{Zoom};

// pub struct ZoomHead<'a> {
//     zoom: &'a Zoom
// }

// impl<'a> ZoomHead<'a> {
//     pub fn new(zoom: &'a Zoom) -> Self {
//         ZoomHead { zoom }
//     }

//     pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area : Rect) {
//         let canva = Canvas::default()
//             .background_color(Color::Rgb(10, 10, 10))
//             .paint(|ctx| {
//                 ctx.draw(&Rectangle{
//                     x: self.zoom.start(),
//                     y: 0f64,
//                     width: self.zoom.length(),
//                     height: 1f64,
//                     color: Color::Red
//                 })})
//             .x_bounds([0f64, 1f64])
//             .y_bounds([0f64, 1f64]);

//         frame.render_widget(canva, area);
//     }
// }

pub struct ChannelsTabs {
    titles: Vec<String>,
    activated: BTreeSet<usize>,
}

impl<'a> ChannelsTabs {
    pub fn new(count: usize) -> Self {
        let mut set = BTreeSet::new();

        for idx in 0..count {
            set.insert(idx);
        }

        Self {
            titles: Self::get_channels_titles(count),
            activated: set,
        }
    }

    pub fn render<B: Backend>(&self, frame: &mut Frame<'_, B>, area: Rect) {
        // Styles
        let separator = Span::raw(" | ");
        let selected_style = Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Green);
        let not_selected_style = Style::default().fg(Color::Gray);

        // Title block
        let block = Block::default()
            .borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
            .title("Channels")
            .title_alignment(Alignment::Right);

        // Create styled channels names
        let mut span_vec: Vec<Span> = vec![];
        for (activated, name) in self.states() {
            let span = if activated {
                Span::styled(name, selected_style)
            } else {
                Span::styled(name, not_selected_style)
            };

            span_vec.push(span);
            span_vec.push(separator.clone());
        }
        span_vec.pop();
        let spans = vec![Spans::from(span_vec)];

        // Assign to paragraph object
        let paragraph = Paragraph::new(spans)
            .block(block)
            .alignment(Alignment::Right);

        frame.render_widget(paragraph, area);
    }

    pub fn count(&self) -> usize {
        self.titles.len()
    }

    pub fn activated(&'a self) -> Vec<(usize, &'a str)> {
        self.activated
            .iter()
            .map(|idx| (*idx, self.titles[*idx].as_str()))
            .collect()
    }

    fn states(&'a self) -> Vec<(bool, &'a str)> {
        (0..self.count())
            .into_iter()
            .map(|idx| {
                let title = self.titles[idx].as_str();
                match self.activated.get(&idx) {
                    Some(_) => (true, title),
                    None => (false, title),
                }
            })
            .collect()
    }

    pub fn update(&mut self, idx: usize) {
        if idx < self.count() {
            match self.activated.get(&idx) {
                Some(_) => {
                    if self.activated.len() > 1 {
                        self.activated.remove(&idx);
                    }
                }
                None => {
                    self.activated.insert(idx);
                }
            };
        }
    }

    pub fn reset(&mut self) {
        for idx in 0..self.count() {
            self.activated.insert(idx);
        }
    }

    fn get_channels_titles(count: usize) -> Vec<String> {
        match count {
            0usize => panic!(),
            1_usize => vec!["Mono"].into_iter().map(|v| v.to_string()).collect(), // mono
            2_usize => vec!["L", "R"].into_iter().map(|v| v.to_string()).collect(), // stereo
            3_usize => vec!["L", "R", "LFE"]
                .into_iter()
                .map(|v| v.to_string())
                .collect(), // 2.1
            5_usize => vec!["FL", "FR", "C", "BL", "BR"]
                .into_iter()
                .map(|v| v.to_string())
                .collect(), // 5.0
            6_usize => vec!["FL", "FR", "C", "LFE", "BL", "BR"]
                .into_iter()
                .map(|v| v.to_string())
                .collect(), // 5.1
            7_usize => vec!["FL", "FR", "C", "LFE", "SL", "SR", "BC"]
                .into_iter()
                .map(|v| v.to_string())
                .collect(), // 6.1
            8_usize => vec!["FL", "FR", "C", "LFE", "SL", "SR", "BL", "BR"]
                .into_iter()
                .map(|v| v.to_string())
                .collect(), // 7.1
            _ => (0..count)
                .into_iter()
                .map(|idx| format!["Channel {:?}", idx])
                .collect(),
        }
    }
}
