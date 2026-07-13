use crate::ast::{Ast, ProvidedServiceImplementation, ServiceReference, Specializes};
use crate::parser::lexer::Lexer;
use crate::parser::lexer::token::Token;
use std::fs::read_to_string;

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

    pub fn from(lexer: Lexer) -> Self {
        Self {
            lexer: lexer,
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

    fn expect(&mut self, expected: Token, context: &str) {
        if self.accept(&expected) {
            return;
        }

        panic!(
            "Syntax error {context}: found {:?}, expected {:?}.",
            self.token, expected
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
            self.expect(Token::Rbracket, "after generic parameter name");
            Some(generic)
        } else {
            None
        }
    }

    fn part(&mut self) -> Vec<Ast> {
        let mut parts = Vec::new();

        while self.accept(&Token::Part) {
            let name = self.ident();
            self.expect(Token::Colon, "after part name");
            let type_name = self.ident();
            let generic = self.generic();

            self.expect(Token::Lbrace, "before part body");
            let mut binds = Vec::new();

            while self.accept(&Token::Bind) {
                let name = self.ident();
                self.expect(Token::To, "after bind name");
                let mut target = vec![self.ident()];

                if self.accept(&Token::Dot) {
                    target.push(self.ident());
                }

                binds.push(Ast::Bind { name, target });
            }

            self.expect(Token::Rbrace, "after part body");

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
                self.expect(Token::Colon, "after provided service name");
                let type_name = self.ident();
                let implementation = if self.accept(&Token::Equals) {
                    let part_name = self.ident();
                    self.expect(Token::Dot, "between delegated part and service name");
                    let service_name = self.ident();
                    ProvidedServiceImplementation::Delegated(ServiceReference {
                        part_name,
                        service_name,
                    })
                } else {
                    ProvidedServiceImplementation::Local
                };

                nodes.push(Ast::Provides {
                    name,
                    type_name,
                    implementation,
                });

                if !self.accept(&Token::Provides) {
                    break;
                }
            }
            nodes.extend(self.part());
        } else {
            panic!(
                "Syntax error in component body: expected at least one `provides name: Type` declaration, found {:?}.",
                self.token
            )
        }

        nodes
    }

    fn requires(&mut self) -> Vec<Ast> {
        let mut nodes = Vec::new();

        while self.accept(&Token::Requires) {
            let name = self.ident();
            self.expect(Token::Colon, "after required service name");
            let type_name = self.ident();
            nodes.push(Ast::Requires { name, type_name });
        }

        nodes.extend(self.provides());

        nodes
    }

    fn component(&mut self) -> Ast {
        self.expect(Token::Component, "before component name");
        let name = self.ident();
        let specializes = if self.accept(&Token::Specializes) {
            let spe_lexer = Lexer::from(&self.lexer);
            let mut spe_parser = Parser::from(spe_lexer);
            spe_parser.next_token();

            let parent = self.ident();
            Some(Specializes{
                parent: parent.clone(),
                parent_file: spe_parser.search_import(parent),
            })
        } else {
            None
        };
        let generic = self.generic();

        self.expect(Token::Lbrace, "before component body");
        let body = Ast::SEQ(self.requires());
        self.expect(Token::Rbrace, "after component body");

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

        self.expect(Token::Namespace, "before namespace path");
        let path = self.path();
        self.expect(Token::Lbrace, "before namespace body");
        let body = self.component();
        self.expect(Token::Rbrace, "after namespace body");

        nodes.push(Ast::Namespace {
            path,
            body: Box::new(body),
        });

        Ast::SEQ(nodes)
    }

    pub fn search_import(&mut self, import: String) -> Option<Box<Ast>>{
        while self.accept(&Token::Import) {
            let file = self.path();

            if file.iter().position(|x| x==&import) != None {
                //To do : get the path to the parent file
                let mut path = String::from("../examples/speadl/");
                path.push_str(&import);
                path.push_str(".speadl");

                let source = read_to_string(path).ok()?;

                let mut parser = Parser::new(&source);

                parser.next_token();
                let ast = parser.namespace();
                
                return Some(Box::new(ast));
            }
        }
        
        None
    }
}
