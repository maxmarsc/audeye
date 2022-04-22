pub mod filled_rectangle;
mod zoom;
pub mod bindings;

pub use zoom::*;

pub mod event;

use sndfile::SndFile;

use num_traits::{Num, NumAssign};

// use core::slice::SlicePattern;
use std::collections::BTreeSet;

use rand::distributions::{Distribution, Uniform};
use rand::rngs::ThreadRng;
use tui::widgets::ListState;

// #[repr(transparent)]
// pub struct Sample<T : NumAssign>(T);

// #[repr(C)]
// pub struct Frame<T : NumAssign, const C: usize> {
//     samples: [Sample<T>; C]
// }

// fn deinterleaved<T: NumAssign, const C: usize>(src: &[Frame<T, C>], dst )
pub fn deinterleave_vec<T : NumAssign + Copy>(channels: usize, src: &[T], dst: &mut[Vec<T>]) {
    let mut vec_slices: Vec<& mut[T]> = dst.iter_mut().map(|vec| vec.as_mut_slice()).collect();
    deinterleave(channels, src, vec_slices.as_mut_slice());
}

pub fn deinterleave<T : NumAssign + Copy>(channels: usize, src: &[T], dst: &mut [&mut [T]]) {
    src.chunks_exact(channels)
        .enumerate()
        .for_each(|(frame_idx, samples)| {
            for (channel, value) in samples.iter().enumerate() {
                dst[channel][frame_idx] = *value;
            }
        });
}

#[derive(Clone)]
pub struct RandomSignal {
    distribution: Uniform<u64>,
    rng: ThreadRng,
}

impl RandomSignal {
    pub fn new(lower: u64, upper: u64) -> RandomSignal {
        RandomSignal {
            distribution: Uniform::new(lower, upper),
            rng: rand::thread_rng(),
        }
    }
}

impl Iterator for RandomSignal {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        Some(self.distribution.sample(&mut self.rng))
    }
}

#[derive(Clone)]
pub struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    scale: f64,
}

impl SinSignal {
    pub fn new(interval: f64, period: f64, scale: f64) -> SinSignal {
        SinSignal {
            x: 0.0,
            interval,
            period,
            scale,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
        self.x += self.interval;
        Some(point)
    }
}

pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
