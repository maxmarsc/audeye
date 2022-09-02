
use super::{Renderer, RenderingInfo};
use crate::utils::{bindings};
use tui::{backend::Backend, Frame, layout::{Alignment, Rect}, style::{Style, Modifier}, text::{Spans, Span}, widgets::{Paragraph, Block, Borders}};


pub struct HelperPopup {
    visible: bool,
    repaint: bool,
}

impl Default for HelperPopup {
    fn default() -> Self {
        Self{
            visible: false,
            repaint: true
        }
    }
}

impl Renderer for HelperPopup {
    fn needs_redraw(&mut self) -> bool {
        self.repaint
    }

    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, _: &RenderingInfo, area : Rect) {
        let name_style = Style::default().add_modifier(Modifier::BOLD);
        let value_style = Style::default();


        let bindings_categories = vec![
            vec![
                ("Quit", bindings::QUIT),
                ("Previous panel", bindings::PREVIOUS_PANEL),
                ("Next panel", bindings::NEXT_PANEL),
                ("Help menu", bindings::HELP),
            ],
            vec![
                ("Zoom in", bindings::ZOOM_IN),
                ("Zoom out", bindings::ZOOM_OUT),
                ("Move left", bindings::MOVE_LEFT),
                ("Move right", bindings::MOVE_RIGHT)
            ],
            vec![
                ("Reset channel selection", bindings::CHANNEL_RESET),
                ("Enable/disable channel 1", bindings::CHANNEL_SELECTOR_1),
                ("Enable/disable channel 2", bindings::CHANNEL_SELECTOR_2),
                ("Enable/disable channel 3", bindings::CHANNEL_SELECTOR_3),
                ("Enable/disable channel 4", bindings::CHANNEL_SELECTOR_4),
                ("Enable/disable channel 5", bindings::CHANNEL_SELECTOR_5),
                ("Enable/disable channel 6", bindings::CHANNEL_SELECTOR_6),
                ("Enable/disable channel 7", bindings::CHANNEL_SELECTOR_7),
                ("Enable/disable channel 8", bindings::CHANNEL_SELECTOR_8),
                ("Enable/disable channel 9", bindings::CHANNEL_SELECTOR_9),
            ]
        ];

        let spans : Vec<Spans> = bindings_categories.iter()
            .map(|cat| {
                cat.iter().map(|(name, value)| {
                    Spans::from(vec![
                        Span::styled(*name, name_style),
                        Span::raw(" : "),
                        Span::styled(bindings::key_to_string(value), value_style)
                    ])})
                .collect()})
            .flat_map(|mut spans: Vec<Spans>| {
                spans.extend(vec![Spans::from("")]);
                spans
            })
            .collect();

        let paragraph = Paragraph::new(spans)
            .block(Block::default().title("Bindings").borders(Borders::ALL))
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);

        self.repaint = false;
    }

}

impl HelperPopup {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, enable: bool) {
        if self.visible != enable {
            self.repaint = true;
        }
        self.visible = enable;
    }
}