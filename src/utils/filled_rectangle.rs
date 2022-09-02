use tui::{
    style::Color,
    widgets::canvas::{Painter, Shape},
};

/// Shape to draw a rectangle from a `Rect` with the given color
#[derive(Debug, Clone)]
pub struct FilledRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Color,
}

impl Shape for FilledRectangle {
    fn draw(&self, painter: &mut Painter) {
        let (b_x, b_y) = match painter.get_point(self.x, self.y) {
            Some(c) => c,
            None => return,
        };
        let (e_x, e_y) = match painter.get_point(self.x + self.width, self.y - self.height) {
            Some(c) => c,
            None => return,
        };

        for x in b_x..e_x {
            for y in b_y..e_y {
                painter.paint(x, y, self.color);
            }
        }
    }
}
