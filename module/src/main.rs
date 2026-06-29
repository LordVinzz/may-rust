use crate::parser::Parser;
use std::error::Error;
use std::fs::read_to_string;

pub mod ast;
pub mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let path = "./src/Start.java";

    let source = read_to_string(path)?;

    let mut parser = Parser::new(&source);

    parser.next_token();
    let ast = parser.class();

    println!("Syntaxe valide");
    println!("{:#?}", ast);
    
    Ok(())
}
