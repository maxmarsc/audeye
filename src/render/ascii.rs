pub struct AsciiArtConverter {
    ascii_chars: [u8; 10],
}

impl Default for AsciiArtConverter {
    fn default() -> Self {
        // let test = "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft\\/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. ";
        // " .:-=+*#%@"
        AsciiArtConverter {
            ascii_chars: [
                ' ' as u8, '.' as u8, ':' as u8, '-' as u8, '=' as u8, '+' as u8, '*' as u8,
                '#' as u8, '%' as u8, '@' as u8,
            ],
        }
    }
}

impl AsciiArtConverter {
    #[inline(always)]
    pub fn convert_u8_to_ascii(&self, value: u8) -> u8 {
        let idx = value / (u8::MAX / 10);
        return self.ascii_chars[idx as usize];
    }
}
