use tui::backend::Backend;
use tui::widgets::Widget;
use tui::Frame;
use tui::layout::Rect;


pub trait Renderer {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, channel: usize, area : Rect);
}


