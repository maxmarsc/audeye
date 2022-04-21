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
use super::time_window::{TimeWindowBatcher, WindowType};
use super::{DspData, DspErr};
use crate::utils::Zoom;

#[inline(always)]
fn db_to_u8(db: f64, threshold: f64) -> u8 {
    if db > 0f64 {
        panic!();
    }
    if db < threshold {
        0u8
    } else {
        ((db - threshold) * u8::MAX as f64 / - threshold) as u8
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

pub struct SpectrogramParameters {
    pub window_size: usize,
    pub overlap_rate: f64,
    pub db_threshold: f64,
    pub window_type: WindowType
}

impl DspData<SpectrogramParameters> for Spectrogram{
    fn new(sndfile: SndFile, parameters: SpectrogramParameters, norm: Option<f64>) -> Result<Spectrogram, DspErr> {
        let channels = sndfile.get_channels();
        let mut window_batcher = match TimeWindowBatcher::new(sndfile, parameters.window_size, 
                parameters.overlap_rate, parameters.window_type) {
            Ok(batcher) => batcher,
            Err(err) => return Err(err)
        };
        if parameters.db_threshold > 0f64 {
            return Err(DspErr::new("dB threshold should be a negative value"));
        }
        let num_bins = parameters.window_size / 2;
        let num_bands = window_batcher.get_num_bands();

        // Allocate the memory for the u8 spectrograms
        let mut spectrograms_u8 = vec![vec![0u8; num_bands * (num_bins)]; channels];

        // Plan the fft
        let mut planner = RealFftPlanner::<f64>::new();
        let r2c = planner.plan_fft_forward(parameters.window_size);
        let mut spectrum = r2c.make_output_vec();
        let mut scratch  = r2c.make_scratch_vec();

        // Compute the Spectrogram
        let mut batch_idx = 0 as usize;
        let fft_len = parameters.window_size as f64 / 2f64;
        let correction_factor = parameters.window_type.correction_factor();


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
                match norm {
                    Some(fnorm) => {
                        let fnorm_inv = 1f64 / fnorm;
                        spectrum[1..num_bins + 1].iter()
                            .enumerate()
                            .for_each(|(fidx, value) | {
                                let bin_amp = (value * correction_factor * fnorm_inv / fft_len).norm_sqr();
                                let db_bin_amp = 10f64 * f64::log10(bin_amp + f64::EPSILON);
                                u8_spectrogram_slice[fidx] = db_to_u8(db_bin_amp, parameters.db_threshold);
                            });
                    },
                    None => {
                        spectrum[1..num_bins + 1].iter()
                            .enumerate()
                            .for_each(|(fidx, value) | {
                                let bin_amp = (value * correction_factor / fft_len).norm_sqr();
                                let db_bin_amp = 10f64 * f64::log10(bin_amp + f64::EPSILON);
                                u8_spectrogram_slice[fidx] = db_to_u8(db_bin_amp, parameters.db_threshold);
                            });
                    }
                }
            }


            batch_idx += 1;

        }

        Ok(Spectrogram{
            num_bands,
            num_bins,
            frames: spectrograms_u8
        })
    }
}

impl Spectrogram {
    pub fn data(&mut self, channel :usize, zoom: &Zoom) -> (&mut [u8], usize) {
        let start = (self.num_bands as f64 * zoom.start()) as usize;
        let end = (self.num_bands as f64 * (zoom.start() + zoom.length())) as usize;

        (&mut self.frames[channel][start * self.num_bins..end * self.num_bins], end - start)
    }

    pub fn num_bins(&self) -> usize {
        self.num_bins
    }

    pub fn num_bands(&self) -> usize {
        self.num_bands
    }
}
