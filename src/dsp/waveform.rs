extern crate sndfile;
use sndfile::SndFileIO;

use crate::sndfile::SndFile;
use std::io::SeekFrom;

use std::convert::{TryFrom, TryInto};

use rayon::prelude::*;

const THRESHOLD: i32 = 0;//1024 * 128;


// #[repr(C)]
// struct StereoSample {
//     l: i32,
//     r: i32,
// }

// #[repr(transparent)]
// struct MonoSample(i32);

// fn interleaved_to_splitted(src: &[StereoSample], left: &mut [MonoSample], right: &mut[MonoSample]) {

// }
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
    // p: Vec<Vec<(f64, f64)>>,
    // n: Vec<Vec<(f64, f64)>>,
    frames: Vec<Vec<i32>>
}

impl Waveform {
    pub fn new(mut sndfile: SndFile) -> Waveform {
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

    pub fn compute_min_max(&self, channel: usize, p: &mut [i32], n: &mut [i32]) {
        if p.len() != n.len() {
            panic!()
        }

        // Compute block size and count
        let block_count = p.len();
        let frames = self.frames[0].len();
        let mut exact_count = false;
        let block_size = if frames % block_count == 0 {
            frames / block_count
        } else {
            exact_count = true;
            frames / block_count + 1
        };

        let samples_chunks = self.frames[channel].par_chunks_exact(block_size);
        let remains = samples_chunks.remainder();

        n[..block_count].par_iter_mut()
            .zip(p[..block_count].par_iter_mut())
            .zip(samples_chunks.into_par_iter())
            .for_each(|((n, p), chunk)| {
                (*n, *p) = get_min_max(chunk);
            });
        

        if ! exact_count {
            // Consume the end
            (n[block_count - 1], p[block_count - 1]) = get_min_max(remains);
        }


    }
}