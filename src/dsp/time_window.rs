extern crate sndfile;
use crate::sndfile::SndFile;

use std::{convert::TryFrom, io::SeekFrom, sync::mpsc::channel, fmt::Display, cmp::min};
use apodize::{hanning_iter, blackman_iter, hamming_iter};
use realfft::num_traits::ops::saturating;
use sndfile::SndFileIO;

use super::DspErr;

use crate::utils::deinterleave_vec;

use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    Blackman,
    Hanning,
    Hamming,
    Uniform
}

#[derive(Debug, Clone, Copy)]
pub struct WindowTypeParseError;

impl Display for WindowTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid window type")
    }
}

const HANNING: &str = "hanning";
const HAMMING: &str = "hamming";
const BLACKMAN: &str = "blackman";
const UNIFORM: &str = "uniform";

impl WindowType {
    fn build_window(&self, size: usize) -> Vec<f64> {
        match self {
            Self::Blackman => blackman_iter(size).collect(),
            Self::Hamming => hamming_iter(size).collect(),
            Self::Hanning => hanning_iter(size).collect(),
            Self::Uniform => vec![1f64; size]
        }
    }

    pub fn correction_factor(&self) -> f64 {
        match self {
            Self::Blackman => 2.80f64,
            Self::Hamming => 1.85f64,
            Self::Hanning => 2f64,
            Self::Uniform => 1f64
        }
    }

    pub fn parse(name: &str) -> Result<Self, WindowTypeParseError> {
        if name == HANNING {
            return Ok(Self::Hanning);
        } else if name == HAMMING {
            return Ok(Self::Hamming);
        } else if name == BLACKMAN {
            return Ok(Self::Blackman);
        } else if name == UNIFORM {
            return Ok(Self::Uniform);
        } else {
            return Err(WindowTypeParseError);
        }
    }

    pub fn possible_values() -> &'static [&'static str] {
        return &[HAMMING, HANNING, BLACKMAN, UNIFORM];
    }

    pub fn default() -> &'static str {
        return HANNING;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SidePaddingType {
    Zeros,
    SmoothRamp,
    Loop
}

struct SidePadding {
    padding_type: SidePaddingType,
    padding_left: Vec<Vec<f64>>,
    padding_right: Vec<Vec<f64>>,
    window_size: usize,
    left_side_offset: usize
}

impl SidePadding {
    fn new(padding_type: SidePaddingType, sndfile: &mut SndFile, tband_size: usize, window_size: usize) -> Self {
        let left_side_offset = (window_size - tband_size) / 2;
        let channels = sndfile.get_channels();

        let mut padding_left = vec![vec![0f64;left_side_offset]; channels];
        let mut padding_right  = vec![vec![0f64;window_size]; channels];

        match padding_type {
            SidePaddingType::Loop => {
                let frames = sndfile.len().unwrap();
                let mut interleaved_data = vec![0f64; channels * window_size];

                // Read the beginning of the file
                sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
                sndfile.read_to_slice(interleaved_data.as_mut_slice()).unwrap();
                deinterleave_vec(channels, interleaved_data.as_slice(), padding_right.as_mut_slice());
                
                // Read the end of the file
                let idx_offset = frames - left_side_offset as u64;
                sndfile.seek(SeekFrom::Start(idx_offset)).unwrap();
                sndfile.read_to_slice(interleaved_data.as_mut_slice()).unwrap();
                deinterleave_vec(channels, &interleaved_data[..left_side_offset*channels], padding_left.as_mut_slice());
            },
            _ => ()
        };

        Self{
            padding_type,
            padding_left: padding_left,
            padding_right: padding_right,
            window_size,
            left_side_offset
        }
    }

    fn pad_left(&mut self, content: &mut [f64], next_sample: f64, channel: usize) {
        let pad_slice = match self.padding_type {
            SidePaddingType::SmoothRamp => {
                let ramp_size = min(content.len(), self.left_side_offset / 64);
                let mut crt_val = 0f64;
                let step = next_sample / ramp_size as f64;

                let start_idx = content.len() - ramp_size;

                // Fill the start with zeros
                self.padding_left[channel][..start_idx].iter_mut().for_each(|val| *val = 0f64);

                // Fill the rest
                self.padding_left[channel][start_idx..content.len()].iter_mut()
                    .for_each(|pad_sample| {
                        *pad_sample = crt_val;
                        crt_val += step;
                });

                &self.padding_left[channel][..content.len()]
            },
            _ => {
                let start = self.left_side_offset - content.len();
                &self.padding_left[channel][start..]
            }
        };

        pad_slice.iter().zip(content.iter_mut())
            .for_each(|(pad_sample, content_sample)| {
                *content_sample = *pad_sample; 
        });
    }

    fn pad_right(&mut self, content: &mut [f64], prev_sample: f64, channel: usize) {
        let pad_slice = match self.padding_type {
            SidePaddingType::SmoothRamp => {
                let ramp_size = min(content.len(), self.left_side_offset / 64);
                let mut crt_val = 0f64;
                let step = prev_sample / ramp_size as f64;

                // Fill the start with the ramp
                self.padding_right[channel][..ramp_size].iter_mut()
                    .for_each(|pad_sample| {
                        *pad_sample = crt_val;
                        crt_val += step;
                });

                // Fill the rest with zeros
                self.padding_right[channel][ramp_size..].iter_mut().for_each(|val| *val = 0f64);

                &self.padding_right[channel][..content.len()]
            },
            _ => {
                // let start = self.left_side_offset - content.len();
                &self.padding_right[channel][..content.len()]
            }
        };

        pad_slice.iter().zip(content.iter_mut())
            .for_each(|(pad_sample, content_sample)| {
                *content_sample = *pad_sample; 
        });
    }

    fn left_side_offset(&self) -> usize {
        self.left_side_offset
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SidePaddingTypeParseError;

impl Display for SidePaddingTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid side padding type")
    }
}

const ZEROS: &str = "zeros";
const RAMP: &str = "ramp";
const LOOP: &str = "loop";
pub const PADDING_HELP_TEXT: &str = 
    "How to fill the missing samples for the firsts and lasts sample windows
    \tzeros : fill with zeros samples
    \tramp : small linear ramp to match the last/next sample
    \tloop : loop the end to the beginning and vice-versa\n";

impl SidePaddingType {
    pub fn parse(name: &str) -> Result<Self, SidePaddingTypeParseError> {
        if name == ZEROS {
            return Ok(Self::Zeros);
        } else if name == RAMP {
            return Ok(Self::SmoothRamp);
        } else if name == LOOP {
            return Ok(Self::Loop);
        } else {
            return Err(SidePaddingTypeParseError);
        }
    }

    pub fn possible_values() -> &'static [&'static str] {
        return &[ZEROS, RAMP, LOOP];
    }

    pub fn default() -> &'static str {
        return ZEROS;
    }
}

pub struct TimeWindowBatcher {
    sndfile: SndFile,
    frames: u64,
    tband_size: usize,
    window_size: usize,
    crt_band_idx: usize,
    num_bands: usize,
    batch: Vec<Vec<f64>>,
    window: Vec<f64>,
    tmp_interleaved_block: Vec<f64>,
    side_padding: SidePadding
}

impl TimeWindowBatcher {
    pub fn new(mut sndfile: SndFile, window_size: usize, overlap: f64, window_type: WindowType, side_padding: SidePaddingType) -> Result<TimeWindowBatcher, DspErr> {
        if 0f64 >= overlap || overlap >= 1f64 {
            return Err(DspErr::new("Overlap values should be contained within ]0:1["))
        }

        let frames = sndfile.len().unwrap();
        let channels = sndfile.get_channels();
        let tband_size = usize::try_from((window_size as f64 * (1. - overlap)) as i32).unwrap();
        sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
        let num_bands = if frames % tband_size as u64 == 0 {
            usize::try_from(frames / tband_size as u64).unwrap()
        } else {
            usize::try_from(frames / tband_size as u64 + 1).unwrap()
        };

        let side_padding = SidePadding::new(side_padding, &mut sndfile, tband_size, window_size);

        Ok(TimeWindowBatcher{
            sndfile,
            frames,
            tband_size,
            window_size,
            crt_band_idx: 0,
            num_bands,
            batch: vec![vec![0f64; window_size]; channels],
            window: window_type.build_window(window_size),
            tmp_interleaved_block: vec![0f64; window_size * channels],
            side_padding
        })
    }

    pub fn get_num_bands(&self) -> usize {
        self.num_bands
    }

    pub fn get_next_batch(&mut self) -> Option<Vec<&mut [f64]>> {
        // We reached the end
        if self.crt_band_idx >= self.num_bands {
            return None;
        }

        // Compute the first sample to seek
        let new_seek_idx = self.crt_band_idx as u64 * self.tband_size as u64;
        self.sndfile.seek(SeekFrom::Start(new_seek_idx as u64)).unwrap_or_else(
                |_| panic!("Failed to seek frame {}", new_seek_idx));

        // The offset left and right of the window lobe
        let side_offset = (self.window_size - self.tband_size) / 2;

        let left_padding_idx = if new_seek_idx < side_offset as u64 {
            // Beginning of the file, need a zero offset at the start
            usize::try_from(side_offset as u64 - new_seek_idx).unwrap()
        } else {
            0 as usize
        };

        let right_padding_idx = if new_seek_idx + self.window_size as u64 > self.frames {
            // End of the file, need a zero offset at the end
            usize::try_from(self.frames - new_seek_idx).unwrap()
        } else {
            self.window_size
        };

        let channels = self.batch.len();

        // Read interleaved data
        let interleaved_write_slice = &mut self.tmp_interleaved_block[left_padding_idx*channels..right_padding_idx*channels];
        match self.sndfile.read_to_slice(interleaved_write_slice) {
            Ok(frames) => {
                if frames != right_padding_idx - left_padding_idx {
                    panic!("Only read {} frames over {}", frames, right_padding_idx - left_padding_idx);
                }
            },
            Err(_) => { panic!("Failed to read"); }
        }
        
        // Write the padding zeros - TODO: vectorize ?
        for (channel, ch_vec) in self.batch.iter_mut().enumerate() {
            // Left padding
            let next_sample = ch_vec[left_padding_idx];
            self.side_padding.pad_left(&mut ch_vec[..left_padding_idx], next_sample, channel);

            // Right padding
            let prev_sample = ch_vec[right_padding_idx - 1];
            self.side_padding.pad_right(&mut ch_vec[right_padding_idx..], prev_sample, channel);
        }

        {
            // Write deinterleaved data into batch vector
            let batch_mut_slice = self.batch.as_mut_slice();
            interleaved_write_slice.chunks(channels)
                .enumerate()
                .for_each(|(frame_idx, samples)| {
                    for (channel, value) in samples.iter().enumerate() {
                        batch_mut_slice[channel][left_padding_idx + frame_idx] = *value;
                    }
            });
        }

        // Apply window to the batch
        for ch_vec in &mut self.batch {

            // Faster ? see : https://www.nickwilcox.com/blog/autovec/
            let window_slice = self.window.as_slice();
            let ch_vec_slice = &mut ch_vec[0..window_slice.len()];

            for i in 0..ch_vec_slice.len() {
                ch_vec_slice[i] *= window_slice[i];
            }
        }

        // build return type
        let ret : Vec<&mut [f64]> = self.batch.iter_mut()
            .map(|v| v.as_mut_slice()).collect();

        // Update index
        self.crt_band_idx += 1;

        Some(ret)
    }

}