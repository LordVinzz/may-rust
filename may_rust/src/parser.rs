use crate::ast::Ast;
use crate::parser::lexer::Lexer;
use crate::parser::lexer::token::Token;

pub mod lexer;

pub struct Parser {
    pub lexer: Lexer,
    pub token: Token,
}

impl Parser {
    pub fn new(file: &str) -> Self {
        Self {
            lexer: Lexer::new(file),
            token: Token::EOF,
        }
    }

    pub fn next_token(&mut self) {
        self.token = self.lexer.next_token()
    }

    fn accept(&mut self, t: &Token) -> bool {
        if t == &self.token {
            self.next_token();
            return true;
        }
        return false;
    }

    fn expect(&mut self, t: Token, caller: &str) -> bool {
        if self.accept(&t) {
            return true;
        }
        panic!(
            "Token inatendu : {:?}, attendait {:?}. Erreur de syntaxe {:?}.",
            self.token, t, caller
        );
    }

    fn ident(&mut self) -> String {
        match &self.token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.next_token();
                name
            }
            _ => panic!(
                "Token inatendu : {:?}, attendait un identifiant.",
                self.token
            ),
        }
    }

    fn path(&mut self) -> Vec<String> {
        let mut path = vec![self.ident()];

        while self.accept(&Token::Dot) {
            path.push(self.ident());
        }

        path
    }

    fn generic(&mut self) -> Option<String> {
        if self.accept(&Token::Lbracket) {
            let generic = self.ident();
            self.expect(Token::Rbracket, "generic");
            Some(generic)
        } else {
            None
        }
    }

    fn part(&mut self) -> Vec<Ast> {
        let mut parts = Vec::new();

        while self.accept(&Token::Part) {
            let name = self.ident();
            self.expect(Token::Colon, "part1");
            let type_name = self.ident();
            let generic = self.generic();

            self.expect(Token::Lbrace, "part2");
            let mut binds = Vec::new();

            while self.accept(&Token::Bind) {
                let name = self.ident();
                self.expect(Token::To, "part3");
                let mut target = vec![self.ident()];

                if self.accept(&Token::Dot) {
                    target.push(self.ident());
                }

                binds.push(Ast::Bind { name, target });
            }

            self.expect(Token::Rbrace, "part4");

            parts.push(Ast::Part {
                name,
                type_name,
                generic,
                body: Box::new(Ast::SEQ(binds)),
            });
        }

        parts
    }

    fn provides(&mut self) -> Vec<Ast> {
        let mut nodes = Vec::new();

        if self.accept(&Token::Provides) {
            loop {
                let name = self.ident();
                self.expect(Token::Colon, "provides1");
                let type_name = self.ident();
                let source = if self.accept(&Token::Equals) {
                    let left = self.ident();
                    self.expect(Token::Dot, "provides2");
                    let right = self.ident();
                    Some(vec![left, right])
                } else {
                    None
                };

                nodes.push(Ast::Provides {
                    name,
                    type_name,
                    source,
                });

                if !self.accept(&Token::Provides) {
                    break;
                }
            }
            nodes.extend(self.part());
        } else {
            panic!("Missing provides")
        }

        nodes
    }

    fn requires(&mut self) -> Vec<Ast> {
        let mut nodes = Vec::new();

        while self.accept(&Token::Requires) {
            let name = self.ident();
            self.expect(Token::Colon, "requires");
            let type_name = self.ident();
            nodes.push(Ast::Requires { name, type_name });
        }

        nodes.extend(self.provides());

        nodes
    }

    fn component(&mut self) -> Ast {
        self.expect(Token::Component, "component1");
        let name = self.ident();
        let specializes = if self.accept(&Token::Specializes) {
            Some(self.ident())
        } else {
            None
        };
        let generic = self.generic();

        self.expect(Token::Lbrace, "component2");
        let body = Ast::SEQ(self.requires());
        self.expect(Token::Rbrace, "component3");

        Ast::Component {
            name,
            specializes,
            generic,
            body: Box::new(body),
        }
    }

    pub fn namespace(&mut self) -> Ast {
        let mut nodes = Vec::new();

        while self.accept(&Token::Import) {
            nodes.push(Ast::Import { path: self.path() });
        }

        self.expect(Token::Namespace, "namespace1");
        let path = self.path();
        self.expect(Token::Lbrace, "namespace2");
        let body = self.component();
        self.expect(Token::Rbrace, "namespace3");

        nodes.push(Ast::Namespace {
            path,
            body: Box::new(body),
        });

        Ast::SEQ(nodes)
    }
}
