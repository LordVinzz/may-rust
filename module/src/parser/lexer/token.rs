#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Identifier(String),

    Semicolon,
    Lbrace,
    Rbrace,
    Lparentheses,
    Rparentheses,

    Package,
    Public,
    Class,

    EOF,
}
