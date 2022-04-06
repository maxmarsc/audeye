extern crate sndfile;
use crate::sndfile::SndFile;

// use core::num::dec2flt::number;
use std::convert::{TryFrom, TryInto};
// use std::intrinsics::log10f64;
use std::num::NonZeroU32;

use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

use rayon::prelude::*;

// use super::compute_spectrogram;
use super::time_window::Batcher;
use super::DspData;

const HANN_FACTOR: f64 = 2f64; // 2²
const DB_MIN_THRESHOLD: f64 = -130f64; // -120dB
const WINDOW_SIZE: usize = 4096;
const OVERLAP_RATE: f64 = 0.75;

#[inline(always)]
fn db_to_u8(db: f64) -> u8 {
    if db > 0f64 {
        panic!();
    }
    if db < DB_MIN_THRESHOLD {
        0u8
    } else {
        ((db - DB_MIN_THRESHOLD) * u8::MAX as f64 / - DB_MIN_THRESHOLD) as u8
    }
}



/// Ordered vertically and by channel. Each channel vector contains contiguous
/// frequency bins
pub struct Spectrogram {
    num_bands: usize,
    num_bins: usize,
    // Ordered by [channel]
    frames: Vec<Vec<u8>>
}


impl DspData for Spectrogram{
    fn new(sndfile: SndFile) -> Spectrogram {
        let channels = sndfile.get_channels();
        let mut window_batcher = Batcher::new(sndfile, WINDOW_SIZE, OVERLAP_RATE);
        let num_bins = WINDOW_SIZE / 2;
        let num_bands = window_batcher.get_num_bands();

        // Allocate the memory for the u8 spectrograms
        let mut spectrograms_u8 = vec![vec![0u8; num_bands * (num_bins)]; channels];

        // Plan the fft
        let mut planner = RealFftPlanner::<f64>::new();
        let r2c = planner.plan_fft_forward(WINDOW_SIZE);
        let mut spectrum = r2c.make_output_vec();
        let mut scratch  = r2c.make_scratch_vec();

        // Compute the Spectrogram
        let mut batch_idx = 0 as usize;
        let fft_len = WINDOW_SIZE as f64 / 2f64;


        loop {
            let mut batchs = match window_batcher.get_next_batch()  {
                Some(batchs) => batchs,
                None => break
            };

            // // Perfect sine
            // let phi = 2f64 * std::f64::consts::PI / WINDOW_SIZE as f64 * 4f64;
            // for channel in batchs.iter_mut() {
            //     channel[0] = 1f64;
            //     for (idx,value) in channel[1..].iter_mut().enumerate() {
            //         *value = f64::sin(phi * idx as f64);
            //     }
            // }

            // // Impulse
            // for channel in batchs.iter_mut() {
            //     channel[0] = 1f64;
            //     for value in channel[1..].iter_mut() {
            //         *value = 1f64;
            //     }
            // }


            // Iterate over each channel
            for (ch_idx, mono_batch) in batchs.iter_mut().enumerate() {
                // Process the FFT
                r2c.process_with_scratch(mono_batch, &mut spectrum, &mut scratch).unwrap();

                let u8_spectrogram_slice = &mut spectrograms_u8[ch_idx][batch_idx*(num_bins)..(batch_idx + 1)*(num_bins)];

                // Compute the magnitude and reduce it to u8
                spectrum[1..num_bins + 1].iter()
                    .enumerate()
                    .for_each(|(fidx, value) | {
                        let bin_amp = (value * HANN_FACTOR / fft_len).norm_sqr();
                        let db_bin_amp = 10f64 * f64::log10(bin_amp + f64::EPSILON);
                        u8_spectrogram_slice[fidx] = db_to_u8(db_bin_amp);
                    });
            }


            batch_idx += 1;

        }

        Spectrogram{
            num_bands,
            num_bins,
            frames: spectrograms_u8
        }
    }
}

impl Spectrogram {
    pub fn data(&mut self, channel :usize) -> &mut [u8] {
        self.frames[channel].as_mut_slice()
    }

    pub fn num_bins(&self) -> usize {
        self.num_bins
    }

    pub fn num_bands(&self) -> usize {
        self.num_bands
    }
}
