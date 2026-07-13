use crate::parser::lexer::token::Token;

pub mod token;

pub struct Lexer {
    file: Vec<char>,
    ind: usize,
}

impl Lexer {
    pub fn new(file: &str) -> Self {
        Self {
            file: file.chars().collect(),
            ind: 0,
        }
    }

    pub fn from(lexer: &Lexer) -> Self {
        Self {
            file: lexer.file.clone(),
            ind: 0,
        }
    }

    fn current_char(&self) -> Option<char> {
        self.file.get(self.ind).copied()
    }

    fn next_char(&mut self) {
        self.ind += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut id = String::new();

        while let Some(c) = self.current_char() {
            match c {
                'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                    id.push(c);
                    self.next_char();
                }
                _ => break,
            }
        }

        id
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.current_char() {
            Some('.') => {
                self.next_char();
                Token::Dot
            }
            Some(':') => {
                self.next_char();
                Token::Colon
            }
            Some('=') => {
                self.next_char();
                Token::Equals
            }
            Some('{') => {
                self.next_char();
                Token::Lbrace
            }
            Some('}') => {
                self.next_char();
                Token::Rbrace
            }
            Some('[') => {
                self.next_char();
                Token::Lbracket
            }
            Some(']') => {
                self.next_char();
                Token::Rbracket
            }

            Some('a'..='z') | Some('A'..='Z') | Some('_') | Some('0'..='9') => {
                let ident = self.read_identifier();

                match ident.as_str() {
                    "import" => Token::Import,
                    "namespace" => Token::Namespace,
                    "component" => Token::Component,
                    "specializes" => Token::Specializes,
                    "provides" => Token::Provides,
                    "requires" => Token::Requires,
                    "part" => Token::Part,
                    "bind" => Token::Bind,
                    "to" => Token::To,
                    _ => Token::Identifier(ident),
                }
            }

            Some(c) => {
                panic!("Caractère invalide: {}", c);
            }

            None => Token::EOF,
        }
    }
}
