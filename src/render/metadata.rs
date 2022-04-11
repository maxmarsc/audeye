
use sndfile::{MajorFormat, SubtypeFormat, Endian};
use tui::style::{Style, Modifier};
use tui::text::Span;
// use sndfile::TagType;
use tui::{backend::Backend, text::Spans};
use tui::Frame;
use tui::layout::{Rect, Alignment, Constraint, Layout, Direction};
use tui::widgets::{Block, Paragraph, Borders};

use std::fmt::{self, Display};
use std::convert::TryFrom;

extern crate sndfile;
use crate::sndfile::{SndFile, TagType};

use super::Renderer;


fn format_to_string(fmt : MajorFormat) -> String {
    match fmt {
        MajorFormat::WAV => "wav",
        MajorFormat::AIFF => "aiff",
        MajorFormat::AU => "au",
        MajorFormat::RAW => "raw",
        MajorFormat::PAF => "paf",
        MajorFormat::SVX => "svx",
        MajorFormat::NIST => "nist",
        MajorFormat::VOC => "voc",
        MajorFormat::IRCAM => "ircam",
        MajorFormat::W64 => "w64",
        MajorFormat::MAT4 => "mat4",
        MajorFormat::MAT5 => "mat5",
        MajorFormat::PVF => "pvf",
        MajorFormat::XI => "xi",
        MajorFormat::HTK => "htk",
        MajorFormat::SDS => "sds",
        MajorFormat::AVR => "avr",
        MajorFormat::WAVEX => "wavex",
        MajorFormat::SD2 => "sd2",
        MajorFormat::FLAC => "flac",
        MajorFormat::CAF => "caf",
        MajorFormat::WVE => "wve",
        MajorFormat::OGG => "ogg",
        MajorFormat::MPC2K => "mpc2k",
        MajorFormat::RF64 => "rf64",
    }.to_string()
}

fn subtype_to_string(fmt : SubtypeFormat) -> String {
    match fmt {
        SubtypeFormat::PCM_S8 => "PCM 8-bit signed",
        SubtypeFormat::PCM_16 => "PCM 16-bit signed",
        SubtypeFormat::PCM_24 => "PCM 24-bit signed",
        SubtypeFormat::PCM_32 => "PCM 32-bit signed",
        SubtypeFormat::PCM_U8 => "PCM 8-bit unsigned",
        SubtypeFormat::FLOAT => "Single precision floating point (f32)",
        SubtypeFormat::DOUBLE => "Double precision floating point (f64)",
        SubtypeFormat::ULAW => "u-law",
        SubtypeFormat::ALAW => "A-law",
        SubtypeFormat::IMA_ADPCM => "IMA/DVI ADPCM",
        SubtypeFormat::MS_ADPCM => "Microsoft ADPCM",
        SubtypeFormat::GSM610 => "GSM 06.10",
        SubtypeFormat::VOX_ADPCM => "ADPCM",
        SubtypeFormat::G721_32 => "CCITT G.721 (ADPCM 32kbits/s)",
        SubtypeFormat::G723_24 => "CCITT G.723 (ADPCM 24kbits/s)",
        SubtypeFormat::G723_40 => "CCITT G.723 (ADPCM 40kbits/s)",
        SubtypeFormat::DWVW_12 => "DWVW 12-bit",
        SubtypeFormat::DWVW_16 => "DWVW 16-bit",
        SubtypeFormat::DWVW_24 => "DWVW 24-bit",
        SubtypeFormat::DWVW_N => "DWVW N-bit",
        SubtypeFormat::DPCM_8 => "DPCM 8-bit",
        SubtypeFormat::DPCM_16 => "DPCM 16-bit",
        SubtypeFormat::VORBIS => "Vorbis",
        SubtypeFormat::ALAC_16 => "ALAC 16-bit",
        SubtypeFormat::ALAC_20 => "ALAC 20-bit",
        SubtypeFormat::ALAC_24 => "ALAC 24-bit",
        SubtypeFormat::ALAC_32 => "ALAC 32-bit",
    }.to_string()
}

fn channel_layout_to_string(channels: usize) -> String {
    match channels {
        1 => "mono",
        2 => "stereo",
        3 => "ambinosic 2.1",
        5 => "ambisonic 5",
        6 => "ambisonic 5.1",
        7 => "ambisonic 7",
        8 => "ambisonic 7.1",
        _ => panic!("Unsupported channel layout")
    }.to_string()
}

fn endianess_to_string(endianess: Endian) -> String {
    match endianess {
        Endian::Little => "Forced little endian",
        Endian::Big => "Forced big endian",
        Endian::File => "default",
        _ => panic!("Unsupported endianess")
    }.to_string()
}

fn compute_duration_string(samplerate: f64, frames: f64) -> String {
    let duration = frames / samplerate;

    let duration_h = f64::floor(duration / 3600f64);
    let duration_m = f64::floor(duration / 60f64);
    let duration_s = duration % 60f64;

    String::from(format!("{:02.0}:{:02.0}:{:02.2}", duration_h, duration_m, duration_s))
}

struct Metadata {
    // Format data
    samplerate: String,
    channel_layout: String,
    format: String,
    subtype: String,
    endianess: String,
    frames: String,
    duration: String,
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
            duration: compute_duration_string(snd.get_samplerate() as f64, snd.len().unwrap() as f64),
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
    fn draw<B : Backend>(&mut self,  frame: &mut Frame<'_, B>, _: &Vec<(usize, &str)>, area : Rect) {
        let name_style = Style::default().add_modifier(Modifier::BOLD);
        let value_style = Style::default();

        // properties
        let properties = vec![
            ("Format", &self.metadata.format),
            ("Format subtype", &self.metadata.subtype),
            ("Endianess", &self.metadata.endianess),
            ("Samplerate", &self.metadata.samplerate),
            ("Channel layout", &self.metadata.channel_layout),
            ("Frames", &self.metadata.frames),
            ("Duration", &self.metadata.duration)
        ];

        // tags
        let tags = vec![
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

        // Layouts
        let constraints = vec![
            Constraint::Length(u16::try_from(properties.len()).unwrap() + 2u16),
            Constraint::Min(u16::try_from(tags.len()).unwrap() + 2u16),
        ];
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints.as_ref())
            .split(area);

        // Build the spans
        let properties_spans: Vec<Spans> = properties.iter()
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
        let tags_spans: Vec<Spans> = tags.iter()
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

        // build paragraph and render
        let properties_paragraph = Paragraph::new(properties_spans)
            .block(Block::default().title("Properties").borders(Borders::ALL))
            .alignment(Alignment::Left);
        frame.render_widget(properties_paragraph, layout[0]);

        let tags_paragraph = Paragraph::new(tags_spans)
            .block(Block::default().title("Tags").borders(Borders::ALL))
            .alignment(Alignment::Left);
        frame.render_widget(tags_paragraph, layout[1]);

        self.redraw = false
    }

    fn needs_redraw(&mut self) -> bool {
        self.redraw
    }
}