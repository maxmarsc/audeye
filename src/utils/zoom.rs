const ZOOM_FACTOR: f64 = 0.9;

#[derive(Debug, Clone)]
pub struct ZoomError;
pub struct Zoom {
    start: f64,
    length: f64,
    min: f64
}

impl Zoom {
    /// Builds a new zoom tracker. Default to no zoom at all
    /// # Arguments
    /// 
    /// * 'max_zoom' - The maximum zoom value allowed for the current
    /// file. 1 is no zoom, 0 is infinite zoom, 0.5 is half the content 
    /// showed at screen
    /// >1 zoom is allowed and will be casted as 1.0
    pub fn new(max_zoom: f64) -> Result<Self, ZoomError> {
        let mut min = max_zoom;
        if max_zoom <= 0f64 {
            return Err(ZoomError)
        } else if min > 1f64 {
            min = 1f64;
        }
        Ok(Zoom { start: 0f64, length: 1f64, min})
    }

    /// Get the relative starting point of the current zoom window btw 0 and 1
    pub fn start(&self) -> f64{
        self.start
    } 

    /// Get the relative length of the current zoom window btw 0 and 1
    pub fn length(&self) -> f64 {
        self.length
    }

    /// Set a new limit for the zoom value
    pub fn update_zoom_max(&mut self, max: f64) {
        self.min = if max <= 1f64 {
            max
        } else {
            1f64
        };

        // Already above the limit, nothing to change
        if self.length >= self.min {
            return;
        }

        // Under the new limit, need to update the current state
        let mut center = self.start + self.length / 2f64;
        self.length = self.min;

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

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;
    use rand::{Rng, prelude::ThreadRng};
    
    const VALID_MAX_VALUES: &[f64] = &[0.5f64, 1.0f64, 3.0f64, 0.000000001f64,
        0.99999999f64];
    const INVALID_MAX_VALUES: &[f64] = &[0f64, -1.0f64, -3.0f64, -0.000000001f64,
        -0.99999999f64];

    const MAX_TOP_VALUE: f64 = f64::MAX;
    const MAX_BTM_VALUE: f64 = f64::EPSILON;

    fn valid_values(rand_count: usize) -> Vec<f64> {
        let mut rng = rand::thread_rng();

        let mut vec = vec![(rng.gen::<f64>() + MAX_BTM_VALUE) * MAX_TOP_VALUE; rand_count];
        vec.extend_from_slice(VALID_MAX_VALUES);
        vec
    }

    #[test]
    fn inits() {
        for max in valid_values(1000) {
            Zoom::new(max).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn bad_init() {
        for max in INVALID_MAX_VALUES {
            Zoom::new(*max).unwrap();
        }
    }

    #[test]
    fn check_init() {
        for max in valid_values(1000) {
            let z = Zoom::new(max).unwrap();
            assert_eq!(z.length(), 1f64);
            assert_eq!(z.start(), 0f64)
        }
    }

    #[test]
    fn check_update_zoom_max() {
        for max in valid_values(1000) {
            let mut z = Zoom::new(max).unwrap();

            for new_max in valid_values(50) {
                z.update_zoom_max(new_max);
                assert!(z.start() >= 0f64);
                assert!(z.start() < 1f64);
                assert!(z.length() <= 1f64);
                assert!(z.length() > 0f64);
            }
        }
    }

    #[test]
    fn check_zoom_in() {
        for max in valid_values(1000) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                z.zoom_in();
                assert!(z.start() >= 0f64);
                assert!(z.start() < 1f64);
                assert!(z.length() <= 1f64);
                assert!(z.length() > 0f64);
            }
         }
    }

    #[test]
    fn check_zoom_out() {
        for max in valid_values(1000) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                z.zoom_out();
                assert!(z.start() >= 0f64);
                assert!(z.start() < 1f64);
                assert!(z.length() <= 1f64);
                assert!(z.length() > 0f64);
            }
        }
    }

    #[test]
    fn check_fuzz_zoom_io() {
        let mut rng = rand::thread_rng();

        for max in valid_values(100) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                match rng.gen::<u32>() % 2 {
                    0 => z.zoom_in(),
                    1 => z.zoom_out(),
                    _ => unreachable!()
                }
            }

            assert!(z.start() >= 0f64);
            assert!(z.start() < 1f64);
            assert!(z.length() <= 1f64);
            assert!(z.length() > 0f64);
        }
    }

    #[test]
    fn check_move_left() {
        for max in valid_values(1000) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                z.move_left();
                assert!(z.start() >= 0f64);
                assert!(z.start() < 1f64);
                assert!(z.length() <= 1f64);
                assert!(z.length() > 0f64);
            }
        }
    }

    #[test]
    fn check_move_right() {
        for max in valid_values(1000) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                z.move_right();
                assert!(z.start() >= 0f64);
                assert!(z.start() < 1f64);
                assert!(z.length() <= 1f64);
                assert!(z.length() > 0f64);
            }
        }
    }

    #[test]
    fn check_fuzz_move() {
        let mut rng = rand::thread_rng();

        for max in valid_values(100) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                match rng.gen::<u32>() % 2 {
                    0 => z.move_left(),
                    1 => z.move_right(),
                    _ => unreachable!()
                }
            }

            assert!(z.start() >= 0f64);
            assert!(z.start() < 1f64);
            assert!(z.length() <= 1f64);
            assert!(z.length() > 0f64);
        }
    }

    #[test]
    fn check_fuzz_all() {
        let mut rng = rand::thread_rng();

        for max in valid_values(100) {
            let mut z = Zoom::new(max).unwrap();

            for _ in 0..1000 {
                match rng.gen::<u32>() % 4 {
                    0 => z.move_left(),
                    1 => z.move_right(),
                    2 => z.zoom_in(),
                    3 => z.zoom_out(),
                    _ => unreachable!()
                }
            }

            assert!(z.start() >= 0f64);
            assert!(z.start() < 1f64);
            assert!(z.length() <= 1f64);
            assert!(z.length() > 0f64);
        }
    }
}