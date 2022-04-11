
use sndfile::{MajorFormat, SubtypeFormat, Endian};
use tui::style::{Style, Modifier};
use tui::text::Span;
// use sndfile::TagType;
use tui::{backend::Backend, text::Spans};
use tui::Frame;
use tui::layout::{Rect, Alignment};
use tui::widgets::{Block, Paragraph};

use std::fmt::{self, Display};

extern crate sndfile;
use crate::sndfile::{SndFile, TagType};

use super::Renderer;


fn format_to_string(fmt : MajorFormat) -> String {
    match fmt {
        WAV => "wav",
        AIFF => "aiff",
        AU => "au",
        RAW => "raw",
        PAF => "paf",
        SVX => "svx",
        NIST => "nist",
        VOC => "voc",
        IRCAM => "ircam",
        W64 => "w64",
        MAT4 => "mat4",
        MAT5 => "mat5",
        PVF => "pvf",
        XI => "xi",
        HTK => "htk",
        SDS => "sds",
        AVR => "avr",
        WAVEX => "wavex",
        SD2 => "sd2",
        FLAC => "flac",
        CAF => "caf",
        WVE => "wve",
        OGG => "ogg",
        MPC2K => "mpc2k",
        RF64 => "rf64",
    }.to_string()
}

fn subtype_to_string(fmt : SubtypeFormat) -> String {
    match fmt {
        PCM_S8 => "PCM 8-bit signed",
        PCM_16 => "PCM 16-bit signed",
        PCM_24 => "PCM 24-bit signed",
        PCM_32 => "PCM 32-bit signed",
        PCM_U8 => "PCM 8-bit unsigned",
        FLOAT => "Single precision floating point (f32)",
        DOUBLE => "Double precision floating point (f64)",
        ULAW => "u-law",
        ALAW => "A-law",
        IMA_ADPCM => "IMA/DVI ADPCM",
        MS_ADPCM => "Microsoft ADPCM",
        GSM610 => "GSM 06.10",
        VOX_ADPCM => "ADPCM",
        G721_32 => "CCITT G.721 (ADPCM 32kbits/s)",
        G723_24 => "CCITT G.723 (ADPCM 24kbits/s)",
        G723_40 => "CCITT G.723 (ADPCM 40kbits/s)",
        DWVW_12 => "DWVW 12-bit",
        DWVW_16 => "DWVW 16-bit",
        DWVW_24 => "DWVW 24-bit",
        DWVW_N => "DWVW N-bit",
        DPCM_8 => "DPCM 8-bit",
        DPCM_16 => "DPCM 16-bit",
        VORBIS => "Vorbis",
        ALAC_16 => "ALAC 16-bit",
        ALAC_20 => "ALAC 20-bit",
        ALAC_24 => "ALAC 24-bit",
        ALAC_32 => "ALAC 32-bit",
    }.to_string()
}


fn channel_layout_to_string(channels: usize) -> String {
    match channels {
        1 => "mono",
        2 => "stereo",
        3 => "ambisonic: 2.1",
        5 => "ambisonic: 5",
        6 => "ambisonic: 5.1",
        7 => "ambisonic: 7",
        8 => "ambisonic: 7.1",
        _ => panic!("Unsupported channel layout")
    }.to_string()
}

fn endianess_to_string(endianess: Endian) -> String {
    match endianess {
        Endian::Little => "Forced little endian",
        Endian::Big => "Forced big endian",
        Endian::File => "Default file endianess",
        _ => panic!("Unsupported endianess")
    }.to_string()
}

struct Metadata {
    // Format data
    samplerate: String,
    channel_layout: String,
    format: String,
    subtype: String,
    endianess: String,
    frames: String,
    // Tags
    title: String,
    copyright: String,
    software: String,
    artist: String,
    comment: String,
    date: String,
    album: String,
    license: String,
    track_number: String,
    genre: String,
}

impl Metadata {
    fn new(path: &std::path::PathBuf) -> Self {
        let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto)
            .from_path(path).unwrap();
        
        let default_msg = "N/A";

        Metadata {
            samplerate: snd.get_samplerate().to_string(),
            channel_layout: channel_layout_to_string(snd.get_channels()),
            format: format_to_string(snd.get_major_format()),
            subtype: subtype_to_string(snd.get_subtype_format()),
            endianess : endianess_to_string(snd.get_endian()),
            frames: snd.len().unwrap().to_string(),
            title: snd.get_tag(TagType::Title).unwrap_or(default_msg.to_string()),
            copyright: snd.get_tag(TagType::Copyright).unwrap_or(default_msg.to_string()),
            artist: snd.get_tag(TagType::Artist).unwrap_or(default_msg.to_string()),
            software: snd.get_tag(TagType::Software).unwrap_or(default_msg.to_string()),
            comment: snd.get_tag(TagType::Comment).unwrap_or(default_msg.to_string()),
            date: snd.get_tag(TagType::Date).unwrap_or(default_msg.to_string()),
            album: snd.get_tag(TagType::Album).unwrap_or(default_msg.to_string()),
            license: snd.get_tag(TagType::License).unwrap_or(default_msg.to_string()),
            track_number: snd.get_tag(TagType::Tracknumber).unwrap_or(default_msg.to_string()),
            genre: snd.get_tag(TagType::Genre).unwrap_or(default_msg.to_string())
        }
    }


}

pub struct MetadataRenderer {
    metadata: Metadata,
    redraw: bool
}

impl MetadataRenderer {
    pub fn new (path: &std::path::PathBuf) -> Self {
        MetadataRenderer{
            metadata: Metadata::new(path),
            redraw: true
        }
    }
}

impl Renderer for MetadataRenderer {
    fn draw<B : Backend>(&mut self, frame: &mut Frame<'_, B>, _: usize, area : Rect, block: Block) {
        let name_style = Style::default().add_modifier(Modifier::BOLD);
        let value_style = Style::default();

        let names_and_values = vec![
            ("Format", &self.metadata.format),
            ("Format subtype", &self.metadata.subtype),
            ("Endianess", &self.metadata.endianess),
            ("Samplerate", &self.metadata.samplerate),
            ("Frames", &self.metadata.frames),
            ("Title", &self.metadata.title),
            ("Copyright", &self.metadata.copyright),
            ("Encoder", &self.metadata.software),
            ("Artist", &self.metadata.artist),
            ("Comment", &self.metadata.comment),
            ("Date", &self.metadata.date),
            ("Album", &self.metadata.album),
            ("License", &self.metadata.license),
            ("Track number", &self.metadata.track_number),
            ("Genre", &self.metadata.genre),
            ("Date", &self.metadata.date),
        ];

        let spans: Vec<Spans> = names_and_values.iter()
            .map(|(name, value)| {
                Spans::from(
                    vec![
                        Span::styled(*name, name_style),
                        Span::raw(" : "),
                        Span::styled(*value, value_style)
                    ]
                )
            })
            .collect();

        let paragraph = Paragraph::new(spans)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);

        self.redraw = false
    }

    fn needs_redraw(&mut self) -> bool {
        self.redraw
    }
}