use super::token::{SpeadlTokenExtension, Token};
use crate::modules::common::lexer::CharReader;
use crate::modules::common::token::{CommonToken, Token as SharedToken};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    position: usize,
    character: char,
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "lexical error at position {}: invalid character `{}` in SpeADL input",
            self.position + 1,
            self.character
        )
    }
}

impl std::error::Error for LexError {}

pub struct Lexer {
    reader: CharReader,
}

impl Lexer {
    pub fn new(file: &str) -> Self {
        Self {
            reader: CharReader::new(file),
        }
    }

    pub fn from(lexer: &Lexer) -> Self {
        Self {
            reader: lexer.reader.reset(),
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.reader.skip_whitespace();

        match self.reader.current_char() {
            Some('.') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Dot))
            }
            Some(':') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Colon))
            }
            Some('=') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Equals))
            }
            Some('{') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Lbrace))
            }
            Some('}') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Rbrace))
            }
            Some('[') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Lbracket))
            }
            Some(']') => {
                self.reader.next_char();
                Ok(SharedToken::Common(CommonToken::Rbracket))
            }

            Some('a'..='z') | Some('A'..='Z') | Some('_') | Some('0'..='9') => {
                let ident = self.reader.read_identifier();

                Ok(match ident.as_str() {
                    "import" => SharedToken::Extended(SpeadlTokenExtension::Import),
                    "namespace" => SharedToken::Extended(SpeadlTokenExtension::Namespace),
                    "component" => SharedToken::Extended(SpeadlTokenExtension::Component),
                    "specializes" => SharedToken::Extended(SpeadlTokenExtension::Specializes),
                    "provides" => SharedToken::Extended(SpeadlTokenExtension::Provides),
                    "requires" => SharedToken::Extended(SpeadlTokenExtension::Requires),
                    "part" => SharedToken::Extended(SpeadlTokenExtension::Part),
                    "bind" => SharedToken::Extended(SpeadlTokenExtension::Bind),
                    "to" => SharedToken::Extended(SpeadlTokenExtension::To),
                    _ => SharedToken::Common(CommonToken::Identifier(ident)),
                })
            }

            Some(character) => Err(LexError {
                position: self.reader.position(),
                character,
            }),

            None => Ok(SharedToken::Common(CommonToken::EOF)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_invalid_characters_with_their_position() {
        let error = Lexer::new("namespace demo @").next_token();
        assert!(error.is_ok(), "the first token should be valid");

        let mut lexer = Lexer::new("@");
        let error = lexer.next_token().expect_err("@ must be rejected");
        assert_eq!(
            error.to_string(),
            "lexical error at position 1: invalid character `@` in SpeADL input"
        );
    }
}
