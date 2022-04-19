use termion::{event::Key};

pub const QUIT: Key = Key::Char('q');
pub const PREVIOUS_PANEL: Key = Key::Left;
pub const NEXT_PANEL: Key = Key::Right;
pub const CHANNEL_SELECTOR_1: Key =  Key::Char('1');
pub const CHANNEL_SELECTOR_2: Key =  Key::Char('2');
pub const CHANNEL_SELECTOR_3: Key =  Key::Char('3');
pub const CHANNEL_SELECTOR_4: Key =  Key::Char('4');
pub const CHANNEL_SELECTOR_5: Key =  Key::Char('5');
pub const CHANNEL_SELECTOR_6: Key =  Key::Char('6');
pub const CHANNEL_SELECTOR_7: Key =  Key::Char('7');
pub const CHANNEL_SELECTOR_8: Key =  Key::Char('8');
pub const CHANNEL_SELECTOR_9: Key =  Key::Char('9');
pub const CHANNEL_RESET: Key = Key::Esc;
pub const HELP: Key = Key::Char(' ');
pub const ZOOM_IN: Key = Key::Char('k');
pub const ZOOM_OUT: Key = Key::Char('j');
pub const MOVE_LEFT: Key = Key::Char('h');
pub const MOVE_RIGHT: Key = Key::Char('l');

// pub fn binding_iterat

pub fn key_to_string(key: &Key) -> String {
    match key {
        Key::Left => String::from("Left arrow"),
        Key::Right => String::from("Right arrow"),
        Key::Up => String::from("Up arrow"),
        Key::Down => String::from("Down arrow"),
        Key::Home => String::from("Home"),
        Key::F(value) => {
            match value {
                1 => "F1",
                2 => "F2",
                3 => "F3",
                4 => "F4",
                5 => "F5",
                6 => "F6",
                7 => "F7",
                8 => "F8",
                9 => "F9",
                10 => "F10",
                11 => "F11",
                12 => "F12",
                _ => panic!()
            }.to_string()
        },
        Key::Char(value) => {
            match *value {
                ' ' => String::from("Space"),
                val => format!("{}", val)
            }
        },
        Key::Alt(value) => format!("alt-{}", value),
        Key::Ctrl(value) => format!("ctl-{}", value),
        Key::Esc => String::from("Esc"),
        _ => panic!()
    }
}