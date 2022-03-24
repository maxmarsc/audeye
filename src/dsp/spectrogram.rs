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

const HANN_FACTOR: f64 = 2f64; // 2²
const DB_MIN_THRESHOLD: f64 = -156f64; // -120dB

#[inline(always)]
fn scale_norm_to_u8(value: f64, max: f64, minref: &mut f64, maxref: &mut f64) -> u8 {
    if value > *maxref {
        *maxref = value;
    }
    if value <= *minref {
        *minref = value;
    }
    if value <= max && value >= 0f64 {
        let test = u8::try_from((value / max * u8::MAX as f64) as i32).unwrap();
        return test;
    } else {
        return u8::MAX;
    }
    // if value >= 2000f64 {
    //     return u8::MAX;
    // }
    // return u8::MAX;
}

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
pub struct SpectralData {
    // Ordered by [channel]
    pub num_frames: NonZeroU32,
    pub frame_size: NonZeroU32,
    pub frames: Vec<Vec<u8>>
}

impl Default for SpectralData {
    fn default() -> Self {
        SpectralData{
            num_frames: NonZeroU32::new(1).unwrap(),
            frame_size: NonZeroU32::new(1).unwrap(),
            frames: vec![]
        }
    }
}

impl SpectralData{
    pub fn new(mut sndfile: SndFile, twindow_size: usize, overlap_size: f64) -> SpectralData {
        let channels = sndfile.get_channels();
        let mut window_batcher = Batcher::new(sndfile, twindow_size, overlap_size);
        let fband_size = twindow_size / 2;
        let num_band = window_batcher.get_num_bands();

        // Allocate the memory for the u8 spectrograms
        let mut spectrograms_u8 = vec![vec![0u8; num_band * (fband_size)]; channels];

        // Plan the fft
        let mut planner = RealFftPlanner::<f64>::new();
        let r2c = planner.plan_fft_forward(twindow_size);
        let mut spectrum = r2c.make_output_vec();
        let mut scratch  = r2c.make_scratch_vec();

        // Compute the Spectrogram
        let mut batch_idx = 0 as usize;
        let fft_len = twindow_size as f64 / 2f64;
        // let
        let mut max = 0f64;
        let mut min = 0f64;
        loop {
            let mut batchs = match window_batcher.get_next_batch()  {
                Some(batchs) => batchs,
                None => break
            };

            // // Perfect sine
            // let phi = 2f64 * std::f64::consts::PI / twindow_size as f64 * 4f64;
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

                let u8_spectrogram_slice = &mut spectrograms_u8[ch_idx][batch_idx*(fband_size)..(batch_idx + 1)*(fband_size)];

                // Compute the magnitude and reduce it to u8
                spectrum[1..fband_size + 1].iter()
                    .enumerate()
                    .for_each(|(fidx, value) | {
                        let bin_amp = (value * HANN_FACTOR / fft_len).norm_sqr();
                        let db_bin_amp = 10f64 * f64::log10(bin_amp + f64::EPSILON);
                        u8_spectrogram_slice[fidx] = db_to_u8(db_bin_amp);
                    });
                // let u8_max: usize = u8::MAX as usize;
                // let modulo = batch_idx / 4;
                // let color_idx = u8::try_from(modulo % u8_max).unwrap();
                // let color_idx = u8::try_into((batch_idx / 4) % u8_max).unwrap();
                // let color_idx = u8::try_from(batch_idx  % (num_band / 4)).unwrap();
                // for elm in u8_spectrogram_slice {
                //     *elm = color_idx;
                // }
            }


            batch_idx += 1;

        }

        // let db_max = 10f64 * f64::log10(max);
        // let db_min = 10f64 * f64::log10(min + f64::EPSILON);

        SpectralData{
            num_frames: NonZeroU32::new(num_band.try_into().unwrap()).unwrap(),
            frame_size: NonZeroU32::new((fband_size).try_into().unwrap()).unwrap(),
            frames: spectrograms_u8
        }
    }
}
