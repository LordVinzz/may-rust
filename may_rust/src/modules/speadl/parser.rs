use super::ast::{Ast, ProvidedServiceImplementation, ServiceReference, Specializes};
use super::lexer::Lexer;
use super::token::{SpeadlTokenExtension, Token};
use crate::modules::common::token::CommonToken;
use std::fmt;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub struct Parser {
    pub lexer: Lexer,
    pub token: Token,
    source_dir: Option<PathBuf>,
    imports: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<super::lexer::LexError> for ParseError {
    fn from(error: super::lexer::LexError) -> Self {
        Self::new(error.to_string())
    }
}

impl Parser {
    pub fn new(file: &str) -> Self {
        Self {
            lexer: Lexer::new(file),
            token: Token::Common(CommonToken::EOF),
            source_dir: None,
            imports: Vec::new(),
        }
    }

    pub fn new_with_path(file: &str, path: &Path) -> Self {
        let mut parser = Self::new(file);
        parser.source_dir = path.parent().map(Path::to_path_buf);
        parser
    }

    pub fn next_token(&mut self) -> Result<(), ParseError> {
        self.token = self.lexer.next_token()?;
        Ok(())
    }

    fn accept(&mut self, t: &Token) -> Result<bool, ParseError> {
        if t == &self.token {
            self.next_token()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect(&mut self, expected: Token, context: &str) -> Result<(), ParseError> {
        if self.accept(&expected)? {
            return Ok(());
        }

        Err(ParseError::new(format!(
            "syntax error {context}: found {:?}, expected {:?}",
            self.token, expected
        )))
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        match &self.token {
            Token::Common(CommonToken::Identifier(name)) => {
                let name = name.clone();
                self.next_token()?;
                Ok(name)
            }
            _ => Err(ParseError::new(format!(
                "syntax error: found {:?}, expected an identifier",
                self.token
            ))),
        }
    }

    fn path(&mut self) -> Result<Vec<String>, ParseError> {
        let mut path = vec![self.ident()?];

        while self.accept(&Token::Common(CommonToken::Dot))? {
            path.push(self.ident()?);
        }

        Ok(path)
    }

    fn generic(&mut self) -> Result<Option<String>, ParseError> {
        if self.accept(&Token::Common(CommonToken::Lbracket))? {
            let generic = self.ident()?;
            self.expect(
                Token::Common(CommonToken::Rbracket),
                "after generic parameter name",
            )?;
            Ok(Some(generic))
        } else {
            Ok(None)
        }
    }

    fn part(&mut self) -> Result<Ast, ParseError> {
        self.expect(
            Token::Extended(SpeadlTokenExtension::Part),
            "before part name",
        )?;
        let name = self.ident()?;
        self.expect(Token::Common(CommonToken::Colon), "after part name")?;
        let type_name = self.ident()?;
        let generic = self.generic()?;
        let mut binds = Vec::new();

        if self.accept(&Token::Common(CommonToken::Lbrace))? {
            while self.accept(&Token::Extended(SpeadlTokenExtension::Bind))? {
                let name = self.ident()?;
                self.expect(Token::Extended(SpeadlTokenExtension::To), "after bind name")?;
                let mut target = vec![self.ident()?];

                if self.accept(&Token::Common(CommonToken::Dot))? {
                    target.push(self.ident()?);
                }

                binds.push(Ast::Bind { name, target });
            }

            self.expect(Token::Common(CommonToken::Rbrace), "after part body")?;
        }

        Ok(Ast::Part {
            name,
            type_name,
            generic,
            body: Box::new(Ast::SEQ(binds)),
        })
    }

    fn provides(&mut self) -> Result<Ast, ParseError> {
        self.expect(
            Token::Extended(SpeadlTokenExtension::Provides),
            "before provided service name",
        )?;
        let name = self.ident()?;
        self.expect(
            Token::Common(CommonToken::Colon),
            "after provided service name",
        )?;
        let type_name = self.ident()?;
        let implementation = if self.accept(&Token::Common(CommonToken::Equals))? {
            let part_name = self.ident()?;
            self.expect(
                Token::Common(CommonToken::Dot),
                "between delegated part and service name",
            )?;
            let service_name = self.ident()?;
            ProvidedServiceImplementation::Delegated(ServiceReference {
                part_name,
                service_name,
            })
        } else {
            ProvidedServiceImplementation::Local
        };

        Ok(Ast::Provides {
            name,
            type_name,
            implementation,
        })
    }

    fn requires(&mut self) -> Result<Ast, ParseError> {
        self.expect(
            Token::Extended(SpeadlTokenExtension::Requires),
            "before required service name",
        )?;
        let name = self.ident()?;
        self.expect(
            Token::Common(CommonToken::Colon),
            "after required service name",
        )?;
        let type_name = self.ident()?;

        Ok(Ast::Requires { name, type_name })
    }

    fn component_body(&mut self) -> Result<Vec<Ast>, ParseError> {
        let mut nodes = Vec::new();

        loop {
            let node = match &self.token {
                Token::Extended(SpeadlTokenExtension::Requires) => self.requires()?,
                Token::Extended(SpeadlTokenExtension::Provides) => self.provides()?,
                Token::Extended(SpeadlTokenExtension::Part) => self.part()?,
                Token::Common(CommonToken::Rbrace) => break,
                _ => {
                    return Err(ParseError::new(format!(
                        "syntax error in component body: found {:?}, expected `requires`, `provides`, `part`, or `}}`",
                        self.token
                    )));
                }
            };
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn specializes(&mut self) -> Result<Option<Specializes>, ParseError> {
        if !self.accept(&Token::Extended(SpeadlTokenExtension::Specializes))? {
            return Ok(None);
        }

        let parent = self.ident()?;
        let argument = self.generic()?;
        Ok(Some(Specializes {
            parent: parent.clone(),
            argument,
            parent_file: self.search_import(parent),
        }))
    }

    fn component(&mut self) -> Result<Ast, ParseError> {
        self.expect(
            Token::Extended(SpeadlTokenExtension::Component),
            "before component name",
        )?;
        let name = self.ident()?;
        let generic = self.generic()?;
        let specializes = self.specializes()?;

        self.expect(Token::Common(CommonToken::Lbrace), "before component body")?;
        let body = Ast::SEQ(self.component_body()?);
        self.expect(Token::Common(CommonToken::Rbrace), "after component body")?;

        Ok(Ast::Component {
            name,
            specializes,
            generic,
            body: Box::new(body),
        })
    }

    pub fn namespace(&mut self) -> Result<Ast, ParseError> {
        let mut nodes = Vec::new();

        while self.accept(&Token::Extended(SpeadlTokenExtension::Import))? {
            let path = self.path()?;
            self.imports.push(path.clone());
            nodes.push(Ast::Import { path, ast: None });
        }

        self.expect(
            Token::Extended(SpeadlTokenExtension::Namespace),
            "before namespace path",
        )?;
        let path = self.path()?;
        self.expect(Token::Common(CommonToken::Lbrace), "before namespace body")?;
        let body = self.component()?;
        self.expect(Token::Common(CommonToken::Rbrace), "after namespace body")?;
        self.expect(Token::Common(CommonToken::EOF), "after namespace")?;

        attach_specialized_parent_to_imports(&mut nodes, &body);

        nodes.push(Ast::Namespace {
            path,
            body: Box::new(body),
        });

        Ok(Ast::SEQ(nodes))
    }

    pub fn search_import(&self, import: String) -> Option<Box<Ast>> {
        let imported_path = self
            .imports
            .iter()
            .find(|path| path.last() == Some(&import))?;
        let source_path = self.resolve_import_path(imported_path)?;
        let source = read_to_string(&source_path).ok()?;
        let mut parser = Parser::new_with_path(&source, &source_path);

        parser.next_token().ok()?;
        let ast = parser.namespace().ok()?;

        Some(Box::new(ast))
    }

    fn resolve_import_path(&self, import: &[String]) -> Option<PathBuf> {
        let file_name = format!("{}.speadl", import.last()?);
        let source_dir = self.source_dir.as_ref()?;

        let same_dir = source_dir.join(&file_name);
        if same_dir.is_file() {
            return Some(same_dir);
        }

        for ancestor in source_dir.ancestors() {
            let path = import
                .iter()
                .fold(ancestor.to_path_buf(), |mut path, part| {
                    path.push(part);
                    path
                });
            let path = path.with_extension("speadl");

            if path.is_file() {
                return Some(path);
            }
        }

        None
    }
}

fn attach_specialized_parent_to_imports(imports: &mut [Ast], component: &Ast) {
    let Ast::Component {
        specializes: Some(specializes),
        ..
    } = component
    else {
        return;
    };

    let Some(parent_file) = &specializes.parent_file else {
        return;
    };

    for import in imports {
        if let Ast::Import { path, ast } = import
            && path.last() == Some(&specializes.parent)
        {
            *ast = Some(parent_file.clone());
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse(source: &str) -> Ast {
        let mut parser = Parser::new(source);
        parser.next_token().expect("test source should lex");
        parser.namespace().expect("test source should parse")
    }

    fn component_from(ast: &Ast) -> &Ast {
        let Ast::SEQ(nodes) = ast else {
            panic!("expected a top-level sequence");
        };
        let Some(Ast::Namespace { body, .. }) = nodes.last() else {
            panic!("expected a namespace");
        };
        body
    }

    #[test]
    fn parses_official_generic_then_specializes_header_order() {
        let ast = parse(
            "import demo.Parent
             namespace demo.child {
                 component Child[T] specializes Parent[String] {}
             }",
        );

        let Ast::Component {
            name,
            specializes,
            generic,
            body,
        } = component_from(&ast)
        else {
            panic!("expected a component");
        };

        assert_eq!(name, "Child");
        assert_eq!(generic.as_deref(), Some("T"));
        assert_eq!(
            specializes
                .as_ref()
                .map(|specializes| specializes.parent.as_str()),
            Some("Parent")
        );
        assert_eq!(
            specializes
                .as_ref()
                .and_then(|specializes| specializes.argument.as_deref()),
            Some("String")
        );
        assert_eq!(body.as_ref(), &Ast::SEQ(Vec::new()));
    }

    #[test]
    fn brackets_after_parent_are_the_official_parent_argument() {
        let ast = parse(
            "namespace demo {
                 component Child specializes Parent[T] {}
             }",
        );

        let Ast::Component {
            specializes,
            generic,
            ..
        } = component_from(&ast)
        else {
            panic!("expected a component");
        };

        assert_eq!(generic, &None);
        assert_eq!(
            specializes
                .as_ref()
                .map(|specializes| specializes.parent.as_str()),
            Some("Parent")
        );
        assert_eq!(
            specializes
                .as_ref()
                .and_then(|specializes| specializes.argument.as_deref()),
            Some("T")
        );
    }

    #[test]
    fn accepts_a_component_parameter_without_a_parent_argument() {
        let ast = parse(
            "namespace demo {
                 component Child[T] specializes Parent {}
             }",
        );

        let Ast::Component {
            specializes,
            generic,
            ..
        } = component_from(&ast)
        else {
            panic!("expected a component");
        };

        assert_eq!(generic.as_deref(), Some("T"));
        assert_eq!(
            specializes
                .as_ref()
                .and_then(|specializes| specializes.argument.as_deref()),
            None
        );
    }

    #[test]
    fn returns_a_descriptive_error_for_invalid_syntax() {
        let mut parser = Parser::new("namespace demo { component Broken { requires } }");
        parser.next_token().expect("fixture should lex");

        let error = parser.namespace().expect_err("missing port name must fail");

        assert!(error.to_string().contains("expected an identifier"));
    }

    #[test]
    fn parses_component_declarations_in_source_order() {
        let ast = parse(
            "namespace demo {
                 component Mixed {
                     part worker: Worker
                     provides delegated: Api = worker.api
                     requires input: Input
                     provides local: Api
                     part client: Client {
                         bind dependency to worker.api
                     }
                     requires output: Output
                 }
             }",
        );

        let Ast::Component { body, .. } = component_from(&ast) else {
            panic!("expected a component");
        };
        let Ast::SEQ(nodes) = body.as_ref() else {
            panic!("expected a component body sequence");
        };

        assert!(matches!(nodes[0], Ast::Part { ref name, .. } if name == "worker"));
        assert!(matches!(
            nodes[1],
            Ast::Provides {
                ref name,
                implementation: ProvidedServiceImplementation::Delegated(
                    ServiceReference {
                        ref part_name,
                        ref service_name,
                    }
                ),
                ..
            } if name == "delegated" && part_name == "worker" && service_name == "api"
        ));
        assert!(matches!(nodes[2], Ast::Requires { ref name, .. } if name == "input"));
        assert!(matches!(
            nodes[3],
            Ast::Provides {
                ref name,
                implementation: ProvidedServiceImplementation::Local,
                ..
            } if name == "local"
        ));
        assert!(matches!(nodes[4], Ast::Part { ref name, .. } if name == "client"));
        assert!(matches!(nodes[5], Ast::Requires { ref name, .. } if name == "output"));

        let Ast::Part { body, .. } = &nodes[4] else {
            unreachable!();
        };
        assert_eq!(
            body.as_ref(),
            &Ast::SEQ(vec![Ast::Bind {
                name: "dependency".to_string(),
                target: vec!["worker".to_string(), "api".to_string()],
            }])
        );
    }

    #[test]
    fn accepts_zero_provided_ports_and_a_part_without_a_body() {
        let ast = parse(
            "namespace demo {
                 component Consumer {
                     requires input: Input
                     part worker: Worker[T]
                 }
             }",
        );

        let Ast::Component { body, .. } = component_from(&ast) else {
            panic!("expected a component");
        };
        assert_eq!(
            body.as_ref(),
            &Ast::SEQ(vec![
                Ast::Requires {
                    name: "input".to_string(),
                    type_name: "Input".to_string(),
                },
                Ast::Part {
                    name: "worker".to_string(),
                    type_name: "Worker".to_string(),
                    generic: Some("T".to_string()),
                    body: Box::new(Ast::SEQ(Vec::new())),
                },
            ])
        );
    }

    #[test]
    fn resolves_imported_parent_with_official_header_order() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after the Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "may-rust-speadl-parser-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("temporary directory should be created");

        let parent_path = directory.join("Parent.speadl");
        fs::write(
            &parent_path,
            "namespace demo { component Parent[T] { provides api: T } }",
        )
        .expect("parent source should be written");

        let child_path = directory.join("Child.speadl");
        let mut parser = Parser::new_with_path(
            "import demo.Parent
             namespace demo {
                 component Child[T] specializes Parent[Api] {
                     requires input: Input
                 }
             }",
            &child_path,
        );
        parser.next_token().expect("fixture should lex");
        let ast = parser.namespace().expect("fixture should parse");

        let Ast::SEQ(nodes) = &ast else {
            panic!("expected a top-level sequence");
        };
        let Ast::Import {
            ast: Some(imported_parent),
            ..
        } = &nodes[0]
        else {
            panic!("expected the imported parent AST to be attached");
        };
        assert!(matches!(
            component_from(imported_parent),
            Ast::Component { name, .. } if name == "Parent"
        ));

        let Ast::Component {
            specializes: Some(specializes),
            ..
        } = component_from(&ast)
        else {
            panic!("expected a specialized component");
        };
        assert!(specializes.parent_file.is_some());
        assert_eq!(specializes.argument.as_deref(), Some("Api"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    #[should_panic(expected = "after namespace")]
    fn rejects_tokens_after_namespace() {
        parse("namespace demo { component Child {} } trailing");
    }
}
