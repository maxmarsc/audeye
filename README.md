# Audeye

<p align="center">
    <b> 🔊 💻 👁️Audio content visualization tool</b>
</p>

![Demo](.github/images/audeye_0_1_0.gif)

Audeye is a terminal tool to visualize audio content, written in Rust

## Features
 - wav / aif / flac / ogg-vorbis and many more (see : [libsndfile format compatibility v1.0.31](https://libsndfile.github.io/libsndfile/formats.html))
 - mono / stereo / 5.1 / 7.1 ... (up to 9 channels)
 - Waveform peak & RMS visualizer
 - Spectrogram visualizer
 - Signal normalization
 - Zoom and move inside both visualizers
 - Metadata display

## Bindings
 - `space` : display bindings
 - `left arrow` / `right arrow` : navigate through panels
 - `j` / `k` : zoom out / in
 - `h` / `l` : move left / right
 - [`0`-`9`] : activate / deactivate display of the corresponding channel
 - `Esc` : reset channel layout

## CLI arguments
 - `-n` : normalize the audio signal before displaying it (not channel aware)
 - `--fft-window-size`
 - `--fft-window-type` : `hanning` / `hamming` / `blackman` / `uniform`
 - `--fft-overlap`
 - `--fft-db-threshold` : minimum energy level to consider (in dB)
 - `--fft-padding-type` : `zeros` / `loop` / `ramp`

### Paddings types
The padding type determine how to fill the sides of each FFT window when at the 
very edges of the audio content
 - Zeros : fill with zeros
 - Ramp : fill with zeros and a small amplitude ramp to match the last/next sample
 - Loop : fill with the end/beginning of the audio file


# Installation
TBD

# Build
1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Then run `cargo run <AUDIO_FILE_PATH>`

## Development
Please consider audeye is still in early development, feedbacks are very welcome

### Contributing
If you wanna contribute, either make a PR (for small changes/adds) or contact me
on twitter / discord if you wanna get involved more deeply
 - [Twitter](https://twitter.com/Groumpf_)
 - [Discord](https://discordapp.com/users/Groumpf#2353)

### Milestone
 - [x] Waveform view
 - [x] Spectogram view
 - [x] Channels view navigation
 - [x] Channel naming (stereo, 2.1, 5.1, 7.1 ...)
 - [x] Zoom in/out
 - [x] Metadata view
 - [x] RMS and Peak in waveform view
 - [x] Option : normalize
 - [x] Option : FFT windows size and overlap
 - [x] Option :  FFT dB threshold
 - [x] Option : FFT window type
 - [x] Option : FFT side smoothing
 - [x] Unit tests
 - [ ] Optionnal labels on graphs
 - [ ] Option : FFT logarithmic scale
 - [ ] Option : Waveform envelope ?
 - [ ] More audio format support
