use tui::backend::Backend;
use tui::widgets::Block;
use tui::Frame;
use tui::layout::Rect;


pub trait Renderer {
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, channel: usize, area : Rect, block: Block<'_>);

}
