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
use super::time_window::{TimeWindowBatcher, WindowType, SidePaddingType};
use super::{DspData, DspErr};
use crate::utils::Zoom;

#[inline(always)]
fn db_to_u8(db: f64, threshold: f64) -> u8 {
    if db > 0f64 {
        return u8::MAX;
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
    pub window_type: WindowType,
    pub side_padding_type: SidePaddingType
}

impl DspData<SpectrogramParameters> for Spectrogram{
    fn new(sndfile: SndFile, parameters: SpectrogramParameters, norm: Option<f64>) -> Result<Spectrogram, DspErr> {
        let channels = sndfile.get_channels();
        let mut window_batcher = match TimeWindowBatcher::new(sndfile, 
                parameters.window_size, 
                parameters.overlap_rate, 
                parameters.window_type,
                parameters.side_padding_type) {
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

#[cfg(test)]
mod tests {
    use crate::dsp::{SidePaddingType, WindowType, Spectrogram, SpectrogramParameters, DspData, AsyncDspData, AsyncDspDataState};
    use sndfile;
    use std::path::{Path, PathBuf};
    use crate::Zoom;
    use std::time::Duration;
    use std::thread::sleep;

    fn get_test_files_location() -> PathBuf {
        return Path::new(&env!("CARGO_MANIFEST_DIR").to_string())
            .join("tests")
            .join("files");
    }

    #[test]
    fn build() {
        let overlaps = [0.25f64, 0.5f64, 0.75f64];
        let windows = [512usize, 1024, 2048, 4096];
        let window_types = [WindowType::Hamming, WindowType::Blackman, WindowType::Hanning, WindowType::Uniform];
        let db_thresholds = [-130f64, -80f64, -30f64];
        let side_paddings = [SidePaddingType::Loop, SidePaddingType::SmoothRamp, SidePaddingType::Zeros];

        for overlap in overlaps {
            for window_size in windows {
                for wtype in window_types {
                    for db_th in db_thresholds {
                        for padding_type in side_paddings {
                            for norm in [None, Some(1.1f64)] {
                                let parameters = SpectrogramParameters {
                                    window_size,
                                    overlap_rate: overlap,
                                    window_type: wtype,
                                    db_threshold: db_th,
                                    side_padding_type: padding_type
                                };

                                let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
                                    .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
                                Spectrogram::new(snd, parameters, norm).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn async_build() {
        const OVERLAP: f64 = 0.25f64;
        const WINDOW_SIZE: usize = 4096;
        const DB_THRESHOLD: f64 = -130f64;
        let sleep_interval = Duration::new(1, 0);

        let parameters = SpectrogramParameters {
            window_size: WINDOW_SIZE,
            overlap_rate: OVERLAP,
            window_type: WindowType::Hanning,
            db_threshold: DB_THRESHOLD,
            side_padding_type: SidePaddingType::Zeros
        };
        let path = get_test_files_location().join("rock_1s.wav");

        let mut async_data: AsyncDspData<Spectrogram, SpectrogramParameters> = AsyncDspData::new(&path, parameters, false);
        let mut attempts = 0;

        loop {
            sleep(sleep_interval);
            async_data.update_status();
            let state = async_data.state();

            assert_ne!(state, AsyncDspDataState::Failed);
            assert!(attempts < 90);

            if state == AsyncDspDataState::Finished {
                break;
            }
            attempts += 1;
        }
    }

    #[test]
    fn async_build_normalize() {
        const OVERLAP: f64 = 0.25f64;
        const WINDOW_SIZE: usize = 4096;
        const DB_THRESHOLD: f64 = -130f64;
        let sleep_interval = Duration::new(1, 0);

        let parameters = SpectrogramParameters {
            window_size: WINDOW_SIZE,
            overlap_rate: OVERLAP,
            window_type: WindowType::Hanning,
            db_threshold: DB_THRESHOLD,
            side_padding_type: SidePaddingType::Zeros
        };
        let path = get_test_files_location().join("rock_1s.wav");

        let mut async_data: AsyncDspData<Spectrogram, SpectrogramParameters> = AsyncDspData::new(&path, parameters, true);
        let mut attempts = 0;

        loop {
            sleep(sleep_interval);
            async_data.update_status();
            let state = async_data.state();

            assert_ne!(state, AsyncDspDataState::Failed);
            assert!(attempts < 90);

            if state == AsyncDspDataState::Finished {
                break;
            }
            attempts += 1;
        }
    }

    #[test]
    fn get_data() {
        const OVERLAP: f64 = 0.25f64;
        const WINDOW_SIZE: usize = 4096;
        const DB_THRESHOLD: f64 = -130f64;

        let parameters = SpectrogramParameters {
            window_size: WINDOW_SIZE,
            overlap_rate: OVERLAP,
            window_type: WindowType::Hanning,
            db_threshold: DB_THRESHOLD,
            side_padding_type: SidePaddingType::Zeros
        };

        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let channels = snd.get_channels();
        let mut spectro = Spectrogram::new(snd, parameters, None).unwrap();
        let num_bins = spectro.num_bins();

        let mut zoom = Zoom::new(0.5f64).unwrap();

        // No zoom
        for ch_idx in 0..channels {
            let (no_zoom_data, num_bands) = spectro.data(ch_idx, &mut zoom);

            assert_ne!(no_zoom_data.len(), 0usize);
            assert_eq!(num_bins * num_bands, no_zoom_data.len());
        }

        // Zoom in and move
        for _ in 0..10 {
            zoom.zoom_in();
            zoom.move_right();

            for ch_idx in 0..channels {
                let (no_zoom_data, num_bands) = spectro.data(ch_idx, &mut zoom);
    
                assert_ne!(no_zoom_data.len(), 0usize);
                assert_eq!(num_bins * num_bands, no_zoom_data.len());
            }
        }
    }
}