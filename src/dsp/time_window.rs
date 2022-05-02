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
    max_padding_right: usize,
    max_padding_left: usize
}

impl SidePadding {
    fn new(padding_type: SidePaddingType, sndfile: &mut SndFile, max_padding_left: usize, max_padding_right: usize) -> Self {
        let channels = sndfile.get_channels();

        let mut padding_left = vec![vec![0f64;max_padding_left]; channels];
        let mut padding_right  = vec![vec![0f64;max_padding_right]; channels];

        match padding_type {
            SidePaddingType::Loop => {
                let frames = sndfile.len().unwrap();
                let mut interleaved_data = vec![0f64; channels * max_padding_right];

                // Read the beginning of the file
                sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
                sndfile.read_to_slice(interleaved_data.as_mut_slice()).unwrap();
                deinterleave_vec(channels, interleaved_data.as_slice(), padding_right.as_mut_slice());
                
                // Read the end of the file
                let idx_offset = frames - max_padding_left as u64;
                sndfile.seek(SeekFrom::Start(idx_offset)).unwrap();
                sndfile.read_to_slice(interleaved_data.as_mut_slice()).unwrap();
                deinterleave_vec(channels, &interleaved_data[..max_padding_left*channels], padding_left.as_mut_slice());
            },
            _ => ()
        };

        Self{
            padding_type,
            padding_left: padding_left,
            padding_right: padding_right,
            max_padding_right,
            max_padding_left
        }
    }

    pub fn pad_left(&mut self, content: &mut [f64], next_sample: f64, channel: usize) {
        if content.len() > self.padding_left[0].len() {
            panic!();
        }
        let pad_slice = match self.padding_type {
            SidePaddingType::SmoothRamp => {
                let ramp_size = min(content.len(), self.max_padding_left / 64);
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
                let start = self.max_padding_left - content.len();
                &self.padding_left[channel][start..]
            }
        };

        pad_slice.iter().zip(content.iter_mut())
            .for_each(|(pad_sample, content_sample)| {
                *content_sample = *pad_sample; 
        });
    }

    pub fn pad_right(&mut self, content: &mut [f64], prev_sample: f64, channel: usize) {
        if content.len() > self.padding_right[0].len() {
            panic!();
        }
        let pad_slice = match self.padding_type {
            SidePaddingType::SmoothRamp => {
                let ramp_size = min(content.len(), self.max_padding_left / 64);
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

    pub fn left_side_offset(&self) -> usize {
        self.max_padding_left
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

        let max_padding_left = (window_size - tband_size) / 2;
        let side_padding = SidePadding::new(side_padding, &mut sndfile, max_padding_left, window_size);

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


#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

const WINDOW_SIZES: &[usize] = &[256, 512, 1024, 2048, 4096, 8192, 333, 10000, 984];

fn get_test_files_location() -> PathBuf {
    return Path::new(&env!("CARGO_MANIFEST_DIR").to_string())
        .join("tests")
        .join("files");
}

mod window_type {
    use crate::dsp::WindowType;

    use super::WINDOW_SIZES;


    #[test]
    fn parse_default() {
        let _ = 

        WindowType::parse(WindowType::default()).unwrap();
    }

    #[test]
    fn parse_all() {
        for value in WindowType::possible_values() {
            WindowType::parse(value).unwrap();
        }
    }

    #[test]
    fn build_window() {
        for value in WindowType::possible_values() {
            let wtype = WindowType::parse(value).unwrap();

            for wsize in WINDOW_SIZES {
                let window = wtype.build_window(*wsize);
                assert_eq!(window.len(), *wsize);

                for sample in window {
                    assert!(sample <= 1f64);
                    assert!(sample >= 0f64);
                }
            }
        }
    }
}

mod side_padding {
    use sndfile::SndFile;

    use crate::dsp::SidePaddingType;
    use crate::dsp::time_window::SidePadding;

    use super::get_test_files_location;

    #[test]
    fn default() {
        const PADDING_LEFT: usize = 1024;
        const WINDOW_SIZE: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();

        let mut data = vec![0f64; WINDOW_SIZE];
    
        let default_type = SidePaddingType::parse(SidePaddingType::default()).unwrap();
        let mut padder = SidePadding::new(default_type, &mut snd, PADDING_LEFT, WINDOW_SIZE);

        for i in 0..snd.get_channels() {
            padder.pad_left(&mut data[..PADDING_LEFT / 2], 1f64, i);
            padder.pad_right(&mut data[..WINDOW_SIZE / 2], 1f64, i);
        }
    }

    #[test]
    fn possible_values() {
        const PADDING_LEFT: usize = 1024;
        const WINDOW_SIZE: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; WINDOW_SIZE];

        for padding_type_str in SidePaddingType::possible_values() {
            let padding_type = SidePaddingType::parse(padding_type_str).unwrap();
            let mut padder = SidePadding::new(padding_type, &mut snd, PADDING_LEFT, WINDOW_SIZE);
    
            for i in 0..snd.get_channels() {
                padder.pad_left(&mut data[..PADDING_LEFT / 2], 1f64, i);
                padder.pad_right(&mut data[..WINDOW_SIZE / 2], 1f64, i);
            }
        }
    }

    #[test]
    #[should_panic]
    fn padding_left_oversize_zeros() {
        const PADDING_LEFT: usize = 1024;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_LEFT * 2];

        let mut padder = SidePadding::new(SidePaddingType::Zeros, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_left(&mut data[..PADDING_LEFT * 2], 1f64, 0);
    }

    #[test]
    #[should_panic]
    fn padding_left_oversize_loop() {
        const PADDING_LEFT: usize = 1024;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_LEFT * 2];

        let mut padder = SidePadding::new(SidePaddingType::Loop, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_left(&mut data[..PADDING_LEFT * 2], 1f64, 0);
    }

    #[test]
    #[should_panic]
    fn padding_left_oversize_ramp() {
        const PADDING_LEFT: usize = 1024;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_LEFT * 2];

        let mut padder = SidePadding::new(SidePaddingType::SmoothRamp, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_left(&mut data[..PADDING_LEFT * 2], 1f64, 0);
    }

    #[test]
    #[should_panic]
    fn padding_right_oversize_zeros() {
        const PADDING_LEFT: usize = 4096;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_RIGHT * 2];

        let mut padder = SidePadding::new(SidePaddingType::Zeros, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_right(&mut data[..PADDING_RIGHT * 2], 1f64, 0);
    }

    #[test]
    #[should_panic]
    fn padding_right_oversize_loop() {
        const PADDING_LEFT: usize = 4096;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_RIGHT * 2];

        let mut padder = SidePadding::new(SidePaddingType::Loop, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_right(&mut data[..PADDING_RIGHT * 2], 1f64, 0);
    }

    #[test]
    #[should_panic]
    fn padding_right_oversize_ramp() {
        const PADDING_LEFT: usize = 4096;
        const PADDING_RIGHT: usize = 4096;

        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
        let mut data = vec![0f64; PADDING_RIGHT * 2];

        let mut padder = SidePadding::new(SidePaddingType::SmoothRamp, &mut snd, PADDING_LEFT, PADDING_RIGHT);
        padder.pad_right(&mut data[..PADDING_RIGHT * 2], 1f64, 0);
    }
}

mod time_window {
    use crate::dsp::{SidePaddingType, WindowType, time_window::TimeWindowBatcher};
    use super::get_test_files_location;
    use std::convert::TryFrom;
    
    #[test]
    fn build() {
        let valid_window_size = &[128usize, 256, 512, 1024, 2048, 4096, 8192, 666, 1333];
        let valid_overlaps = &[0.1f64, 0.25f64, 0.5f64, 0.75f64, 0.95f64];
        
        for window_size in valid_window_size {
            for overlap in valid_overlaps {
                let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
                    .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
                TimeWindowBatcher::new(snd, *window_size, *overlap, WindowType::Hanning, SidePaddingType::Zeros).unwrap();
            }
        }
    }

    #[test]
    #[should_panic]
    fn negative_overlap() {
        const WINDOW_SIZE: usize = 2048;
        const OVERLAP: f64 = -1f64;
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();

        TimeWindowBatcher::new(snd, WINDOW_SIZE, OVERLAP, WindowType::Hanning, SidePaddingType::Zeros).unwrap();
    }

    #[test]
    #[should_panic]
    fn overlap_greater_than_one() {
        const WINDOW_SIZE: usize = 2048;
        const OVERLAP: f64 = 1.5f64;
        let snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();

        TimeWindowBatcher::new(snd, WINDOW_SIZE, OVERLAP, WindowType::Hanning, SidePaddingType::Zeros).unwrap();
    }

    #[test]
    fn check_content() {
        let valid_window_size = &[128usize, 256, 512, 1024, 2048, 4096, 8192, 666, 1333];
        let valid_overlaps = &[0.1f64, 0.25f64, 0.5f64, 0.75f64, 0.95f64];
        
        for window_size in valid_window_size {
            for overlap in valid_overlaps {
                let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
                    .from_path(get_test_files_location().join("rock_1s.wav")).unwrap();
                let frames = snd.len().unwrap();
                let channels = snd.get_channels();

                let band_size = (*window_size as f64 * (1f64 - *overlap)) as i32;
                let expected_num_batch = usize::try_from(frames / band_size as u64).unwrap();
                
                let mut batcher = TimeWindowBatcher::new(snd, *window_size, *overlap, WindowType::Hanning, SidePaddingType::Zeros).unwrap();
                let num_batch = batcher.get_num_bands();

                assert!(expected_num_batch == num_batch 
                        || expected_num_batch + 1 == num_batch);
                
                let mut count = 0usize;

                loop {
                    let batch_opt = batcher.get_next_batch();
                    if batch_opt.is_none() {
                        break;
                    }
                    let batch = batch_opt.unwrap();
                    
                    assert_eq!(batch.len(), channels);
                    
                    for chan in batch {
                        assert_eq!(chan.len(), *window_size);
                    }
    
                    count += 1;
                }

                assert_eq!(count, num_batch);
            }
        }
    }
}

}
