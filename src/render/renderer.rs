use tui::widgets::Widget;


pub trait Renderer<'a, T : Widget> {
    fn get_representation(&'a mut self, channel: usize) -> Option<T>;
}

