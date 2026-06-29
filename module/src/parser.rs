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

    fn access(&mut self) -> Option<String> {
        if self.accept(&Token::Public) {
            Some("public".to_string())
        } else {
            None
        }
    }

    fn function(&mut self) -> Ast {
        let access = self.access();

        let type_name = self.ident();
        let name = self.ident();

        self.expect(Token::Lparentheses, "function1");
        self.expect(Token::Rparentheses, "function2");
        self.expect(Token::Lbrace, "function3");
        self.expect(Token::Rbrace, "function4");

        Ast::Function {
            name,
            type_name,
            access,
            body: None,
        }
    }

    pub fn class(&mut self) -> Ast {
        let mut nodes = Vec::new();

        while self.accept(&Token::Package) {
            nodes.push(Ast::Package { name : self.ident() });
            self.expect(Token::Semicolon, "class1");
        }

        let access = self.access();

        self.expect(Token::Class, "class2");
        let name = self.ident();
        self.expect(Token::Lbrace, "class3");
        let body = self.function();
        self.expect(Token::Rbrace, "class4");

        nodes.push(Ast::Class {
            name,
            access,
            body: Box::new(body),
        });

        Ast::SEQ(nodes)
    }
}
