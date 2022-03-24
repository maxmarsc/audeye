extern crate sndfile;
use crate::sndfile::SndFile;

use std::{convert::TryFrom, io::SeekFrom, sync::mpsc::channel};
use apodize::hanning_iter;
use sndfile::SndFileIO;

use rayon::prelude::*;

pub struct Batcher {
    sndfile: SndFile,
    frames: u64,
    tband_size: usize,
    window_size: usize,
    crt_band_idx: usize,
    num_bands: usize,
    batch: Vec<Vec<f64>>,
    window: Vec<f64>,
    tmp_interleaved_block: Vec<f64>
}

impl Batcher {
    pub fn new(mut sndfile: SndFile, window_size: usize, overlap: f64) -> Batcher {
        let frames = sndfile.len().unwrap();
        let channels = sndfile.get_channels();
        let tband_size = usize::try_from((window_size as f64 * (1. - overlap)) as i32).unwrap();
        sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
        let num_bands = if frames % tband_size as u64 == 0 {
            usize::try_from(frames / tband_size as u64).unwrap()
        } else {
            usize::try_from(frames / tband_size as u64 + 1).unwrap()
        };

        Batcher{
            sndfile,
            frames,
            tband_size,
            window_size,
            crt_band_idx: 0,
            num_bands,
            batch: vec![vec![0f64; window_size]; channels],
            window: hanning_iter(window_size).collect(),
            tmp_interleaved_block: vec![0f64; window_size * channels]
        }
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
        for ch_vec in &mut self.batch {
            ch_vec[..left_padding_idx].iter_mut().for_each(|v| *v = 0f64);
            ch_vec[right_padding_idx..].iter_mut().for_each(|v| *v = 0f64);
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