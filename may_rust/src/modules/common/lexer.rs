#[derive(Clone)]
pub struct CharReader {
    chars: Vec<char>,
    index: usize,
}

impl CharReader {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
        }
    }

    pub fn reset(&self) -> Self {
        Self {
            chars: self.chars.clone(),
            index: 0,
        }
    }

    pub fn current_char(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    pub fn position(&self) -> usize {
        self.index
    }

    pub fn next_char(&mut self) {
        self.index += 1;
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    pub fn read_identifier(&mut self) -> String {
        let mut identifier = String::new();

        while let Some(c) = self.current_char() {
            match c {
                'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                    identifier.push(c);
                    self.next_char();
                }
                _ => break,
            }
        }

        identifier
    }
}
