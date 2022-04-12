const ZOOM_FACTOR: f64 = 0.9;

#[derive(Debug, Clone)]
pub struct ZoomError;
pub struct Zoom {
    start: f64,
    length: f64,
    min: f64
}

impl Zoom {
    pub fn new(max_zoom: f64) -> Result<Self, ZoomError> {
        if max_zoom <= 0f64 || max_zoom > 1f64 {
            return Err(ZoomError)
        }
        Ok(Zoom { start: 0f64, length: 1f64, min:  max_zoom})
    }

    pub fn start(&self) -> f64{
        self.start
    } 

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn update_zoom_max(&mut self, max: f64) {
        self.min = max;

        // Already above the limit, nothing to change
        if self.length >= max {
            return;
        }

        // Under the new limit, need to update the current state
        let mut center = self.start + self.length / 2f64;
        self.length = max;

        // Compute if the new center is appropriate or not anymore
        if center + (self.length / 2f64) > 1f64 {
            center = 1f64 - (self.length / 2f64);
        } else if center - (self.length / 2f64) < 0f64{
            center = self.length / 2f64;
        }

        self.start = center - self.length / 2f64;

    }

    pub fn zoom_in(&mut self) {
        let center = self.start + self.length / 2f64;

        self.length = if self.length * ZOOM_FACTOR <= self.min {
            self.min
        } else {
            self.length * ZOOM_FACTOR
        };

        self.start = center - self.length / 2f64;
    }

    pub fn zoom_out(&mut self) {
        let mut center = self.start + self.length / 2f64;

        self.length = if self.length / ZOOM_FACTOR > 1f64 {
            1f64
        } else {
            self.length / ZOOM_FACTOR
        };

        // Compute if the new center is appropriate or not anymore
        if center + (self.length / 2f64) > 1f64 {
            center = 1f64 - (self.length / 2f64);
        } else if center - (self.length / 2f64) < 0f64{
            center = self.length / 2f64;
        }

        self.start = center - self.length / 2f64;
    }

    pub fn move_left(&mut self) {
        let offset = self.length / 10f64;

        self.start = if self.start - offset < 0f64 {
            0f64
        } else {
            self.start - offset
        }
    }

    pub fn move_right(&mut self) {
        let offset = self.length / 10f64;

        self.start = if self.start + self.length + offset > 1f64 {
            1f64 - self.length
        } else {
            self.start + offset
        }
    }
}