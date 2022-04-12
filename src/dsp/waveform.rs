extern crate sndfile;
use sndfile::SndFileIO;

use crate::sndfile::SndFile;
use std::io::SeekFrom;

use std::convert::{TryFrom, TryInto};

use rayon::prelude::*;

use super::DspData;
use crate::utils::Zoom;

const THRESHOLD: i32 = 0;

#[inline(always)]
fn get_min_max(frames: &[i32]) -> (i32, i32) {
    let mut min = 0i32;
    let mut max = 0i32;

    for elm in frames {
        if *elm > max {
            max = *elm;
        } else if *elm < min {
            min = *elm;
        }
    };

    (min, max)
}

pub struct Waveform {
    frames: Vec<Vec<i32>>
}

impl DspData for Waveform {
    fn new(mut sndfile: SndFile) -> Waveform {
        // Compute block size
        let frames = sndfile.len().expect("Unable to retrieve number of frames");
        sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
        let block_size = 4096usize;
        let block_count = if frames % block_size as u64 == 0 {
            usize::try_from(frames / block_size as u64).unwrap()
        } else {
            usize::try_from(frames / block_size as u64 + 1).unwrap()
        };
        let channels = sndfile.get_channels();

        // Create data vectors
        let mut data = Waveform {
            frames: vec![vec![0i32;usize::try_from(frames).unwrap()]; channels]
        };
        let mut block_data : Vec<i32> = vec![0; block_size*channels];

        // Find min and max for each block
        for block_idx in 0..block_count {

            // Read block from file
            let mut nb_frames: usize = 0;
            let read = sndfile.read_to_slice(block_data.as_mut_slice());
            match read {
                Ok(frames) => {
                    if frames == 0 { panic!("0 frames read")}
                    nb_frames = frames;
                },
                Err(err) => panic!("{:?}", err)
            }

            // Load into frames vector
            let frame_offset = block_idx * block_size;
            {
                let interleaved_slice = &block_data[..nb_frames * channels];
                // let mono_slices = data.frames.iter_mut().map(|channel| {
                //     channel.as_mut_slice()[frame_idx..frame_idx + interleaved_slice.len()]
                // }).collect();
                interleaved_slice.chunks_exact(channels)
                    .enumerate()
                    .for_each(|(frame_idx, samples)| {
                        for (channel, value) in samples.iter().enumerate() {
                            data.frames[channel][frame_offset + frame_idx] = *value;
                        }
                    });
            }

        };

        data
    }
}

impl Waveform {
    pub fn compute_min_max(&self, channel: usize, block_count: usize, zoom: &Zoom) -> (Vec<i32>, Vec<i32>) {
        // Alloc vectors
        let mut p = vec![0i32; block_count];
        let mut n = vec![0i32; block_count];

        // Compute block size and count
        let total_frames = self.frames[0].len();
        let start = (total_frames as f64 * zoom.start()) as usize;
        let end = (total_frames as f64 * (zoom.start() + zoom.length())) as usize;
        let rendered_frames = end - start;
        let block_size = if rendered_frames % block_count == 0 {
            rendered_frames / block_count
        } else {
            rendered_frames / block_count + 1
        };

        let samples_chunks = self.frames[channel][start..end].par_chunks_exact(block_size);
        let remains = samples_chunks.remainder();

        n[..block_count].par_iter_mut()
            .zip(p[..block_count].par_iter_mut())
            .zip(samples_chunks.into_par_iter())
            .for_each(|((n, p), chunk)| {
                (*n, *p) = get_min_max(chunk);
            });
        

        if remains.len() > 0 {
            // Consume the end
            (n[block_count - 1], p[block_count - 1]) = get_min_max(remains);
        }

        (n, p)
    }
}