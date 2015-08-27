use std::collections::{HashSet};

#[derive(Debug,Eq,PartialEq)]
pub struct Chord {
   keys: HashSet<Key>
}

impl Chord {
    pub fn with(c: Key) -> Self {
        let chord = Chord {
            keys: HashSet::new(),
        };
        chord.and(c)
    }

    pub fn and(mut self, c: Key) -> Self {
        self.keys.insert(c);
        self
    }
}

pub type Arpeggio = Vec<Chord>;


pub struct Mapping {
    mapping: Arpeggio,
    function: Box<FnMut()>,
}

impl Mapping {
    pub fn new(lhs: Arpeggio, function: Box<FnMut()>) -> Self {
        Mapping {
            mapping: lhs,
            function: function,
        }
    }
}

struct KeyEvent {
    delta_t: f64,
    key: Key,
}

pub struct KeymapHandler {
    keymaps: Vec<Mapping>,
    current_key_events: Vec<KeyEvent>,
}

impl KeymapHandler {
    pub fn new() -> Self {
        KeymapHandler {
            keymaps: Vec::new(),
            current_key_events: Vec::new(),
        }
    }

    pub fn insert(&mut self, mapping: Mapping) {
        self.keymaps.push(mapping);
    }

    pub fn key_down(&mut self, delta_t: f64, key: Key) {
        self.current_key_events.push(KeyEvent {
            delta_t: delta_t,
            key: key,
        });
        self.check_if_current_key_match();
    }

    // NOCOM(#sirver): this should be triggered after a while and the
    // best prefix
    pub fn timeout(&mut self) {
        self.check_if_current_key_match();
        self.current_key_events.clear();
    }

    pub fn check_if_current_key_match(&mut self) {
        let mut arpeggio: Arpeggio = Vec::new();
        for key_event in &self.current_key_events {
            // NOCOM(#sirver): make configurable
            if key_event.delta_t < 50e-3 && !arpeggio.is_empty() {
                let last = arpeggio.last_mut().unwrap();
                last.keys.insert(key_event.key);
            } else {
                arpeggio.push(Chord::with(key_event.key));
            }
        }

        // NOCOM(#sirver): this should actually check the prefix only.
        let mut possible_keys: Vec<_> = self.keymaps
            .iter_mut()
            .filter(|keymap| { keymap.mapping == arpeggio })
            .collect();

        if possible_keys.len() == 1 {
            let mut mapping = possible_keys.last_mut().unwrap();
            (mapping.function)();
        }
    }
}

#[derive(Debug,PartialEq,Eq,Hash,Clone,Copy)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Ctrl,
    Char(char),
}

// NOCOM(#sirver): This needs to support Chords too, like "<Ctrl>ta" for pressing Ctrl, t and a
// too. That requires a simple parser.
impl Key {
    fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        match &lower as &str {
            "<up>" => return Some(Key::Up),
            "<down>" => return Some(Key::Down),
            "<left>" => return Some(Key::Left),
            "<right>" => return Some(Key::Right),
            "<ctrl>" => return Some(Key::Ctrl),
            _ => (),
        };

        let chars: Vec<_> = lower.chars().collect();
        if chars.len() == 1 {
            return Some(Key::Char(*chars.first().unwrap()));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell;
    use std::rc;

    #[test]
    fn test_simple_coord() {
        let mut keymap_handler = KeymapHandler::new();

        let mut arpeggio = Vec::new();
        arpeggio.push(Chord::with(Key::Up).and(Key::Down));

        let v = rc::Rc::new(cell::Cell::new(false));
        let v_clone = v.clone();
        keymap_handler.insert(Mapping::new(
            arpeggio, Box::new(move || {
                v_clone.set(true);
            })
        ));
        keymap_handler.key_down(1000., Key::Down);
        keymap_handler.key_down(20e-3, Key::Up);

        assert_eq!(v.get(), true);
    }

    #[test]
    fn test_simple_arpeggio() {
        let mut keymap_handler = KeymapHandler::new();

        let mut arpeggio = Vec::new();
        arpeggio.push(Chord::with(Key::Char(',')));
        arpeggio.push(Chord::with(Key::Char('g')));
        arpeggio.push(Chord::with(Key::Char('f')));

        let v = rc::Rc::new(cell::Cell::new(false));
        let v_clone = v.clone();
        keymap_handler.insert(Mapping::new(
            arpeggio, Box::new(move || {
                v_clone.set(true);
            })
        ));
        keymap_handler.key_down(1000., Key::Char(','));
        keymap_handler.key_down(80e-3, Key::Char('g'));
        keymap_handler.key_down(80e-3, Key::Char('f'));

        assert_eq!(v.get(), true);
    }

    #[test]
    fn test_arpeggio_with_chords() {
        let mut keymap_handler = KeymapHandler::new();

        let mut arpeggio = Vec::new();
        arpeggio.push(Chord::with(Key::Char('g')).and(Key::Ctrl));
        arpeggio.push(Chord::with(Key::Char(',')));
        arpeggio.push(Chord::with(Key::Char('f')));

        let v = rc::Rc::new(cell::Cell::new(false));
        let v_clone = v.clone();
        keymap_handler.insert(Mapping::new(
            arpeggio, Box::new(move || {
                v_clone.set(true);
            })
        ));
        // NOCOM(#sirver): timining parameters can be simplified to immediate or quick.
        keymap_handler.key_down(1000., Key::Char('g'));
        keymap_handler.key_down(80e-3, Key::Ctrl);
        keymap_handler.key_down(80e-3, Key::Char(','));
        keymap_handler.key_down(80e-3, Key::Char('f'));
        assert_eq!(v.get(), false);

        keymap_handler.timeout();
        keymap_handler.key_down(1000., Key::Char('g'));
        keymap_handler.key_down(40e-3, Key::Ctrl);
        keymap_handler.key_down(80e-3, Key::Char(','));
        keymap_handler.key_down(80e-3, Key::Char('f'));
        assert_eq!(v.get(), true);
    }

    #[test]
    fn test_char_from_string() {
        assert_eq!(Some(Key::Up), Key::from_str("<UP>"));
        assert_eq!(Some(Key::Ctrl), Key::from_str("<CTrl>"));
        assert_eq!(Some(Key::Char('ö')), Key::from_str("ö"));
    }

}
