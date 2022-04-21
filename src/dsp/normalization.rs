use crate::sndfile::SndFile;
use rayon::prelude::*;
use sndfile::SndFileIO;

pub fn compute_norm(sndfile: &mut SndFile) -> f64 {
    let data: Vec<i32> = sndfile.read_all_to_vec().unwrap();

    let max = data.par_iter().map(|val| { val.abs()}).max().unwrap();

    if max <= 0i32 {
        return f64::EPSILON;
    }
    return max as f64 / i32::MAX as f64;
}