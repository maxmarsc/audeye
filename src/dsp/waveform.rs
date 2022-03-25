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
    p: Vec<Vec<(f64, f64)>>,
    n: Vec<Vec<(f64, f64)>>,
    frames: Vec<Vec<i32>>
}

impl Waveform {
    pub fn new(mut sndfile: SndFile, block_count: usize) -> Waveform {
        // Compute block size
        let frames = sndfile.len().expect("Unable to retrieve number of frames");
        sndfile.seek(SeekFrom::Start(0)).expect("Failed to seek 0");
        let block_size = usize::try_from(frames / block_count as u64)
            .expect("Block size is too big, file is probably too long");
        let channels = sndfile.get_channels();

        // Create data vectors
        let mut data = Waveform {
            p: vec![],
            n: vec![],
            frames: vec![vec![0i32;usize::try_from(frames).unwrap()]; channels]
        };
        data.reserve(channels, block_count);
        let mut block_data : Vec<i32> = vec![0; block_size*channels];

        // Find min and max for each block
        for block_idx in 0..block_count {
            // // Check for termination signal
            // match kill_rx.try_recv() {
            //     Ok(_) | Err(TryRecvError::Disconnected) => {
            //         return data;
            //     }
            //     Err(TryRecvError::Empty) => {}
            // }

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
                let interleaved_slice = block_data.as_slice();
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

    
            // Compute min & max
            let mut mins = vec![0 as i32; channels];
            let mut maxs = vec![0 as i32; channels];
            for frame_idx in 0..nb_frames {
                // match kill_rx.try_recv() {
                //     Ok(_) | Err(TryRecvError::Disconnected) => {
                //         println!("Terminating.");
                //         break;
                //     }
                //     Err(TryRecvError::Empty) => {}
                // }

                for ch_idx in 0..channels {
                    let value = block_data[frame_idx * channels + ch_idx];
                    if value < mins[ch_idx] {
                        mins[ch_idx] = value
                    } else if value > maxs[ch_idx] {
                        maxs[ch_idx] = value;
                    }
                }
            }

            // // Check for termination signal
            // match kill_rx.try_recv() {
            //     Ok(_) | Err(TryRecvError::Disconnected) => {
            //         return data;
            //     }
            //     Err(TryRecvError::Empty) => {}
            // }


            for ch_idx in 0..channels {
                if mins[ch_idx] < - THRESHOLD {
                    data.n[ch_idx].push((block_idx as f64, mins[ch_idx] as f64 / i32::MAX as f64));
                }
                if maxs[ch_idx] > THRESHOLD {
                    data.p[ch_idx].push((block_idx as f64, maxs[ch_idx] as f64 / i32::MAX as f64));
                }
            }
        };

        data
    }

    pub fn p_data(&self, channel: usize) -> &[(f64,f64)] {
        self.p[channel].as_slice()
    }

    pub fn n_data(&self, channel: usize) -> &[(f64,f64)] {
        self.n[channel].as_slice()
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

    fn reserve(&mut self, channels: usize, block_count: usize) {
        self.p.clear();
        self.n.clear();
        for ch_idx in 0..channels {
            self.p.push(vec![]);
            self.n.push(vec![]);
            self.p[ch_idx].reserve_exact(block_count);
            self.n[ch_idx].reserve_exact(block_count);
        }
    }
}