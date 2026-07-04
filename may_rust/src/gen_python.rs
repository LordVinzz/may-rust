use crate::ast::Ast;
use std::fs::{self, create_dir_all};

//deal with specializes
//no analog to generic in py

pub struct GenPython {
    g_ast: Ast,
    path: Vec<String>,
    ls_import: Vec<Vec<String>>,
    name_component: String,
    ls_requires: Vec<(String, String)>,
    ls_provides: Vec<(String, String, Option<Vec<String>>)>,
    ls_part: Vec<(String, String, Vec<String>)>,
}

impl GenPython{

    pub fn new(ast : Ast) -> Self {
        Self {
            g_ast: ast,
            path: Vec::new(),
            ls_import: Vec::new(),
            name_component: String::new(),
            ls_requires: Vec::new(),
            ls_provides: Vec::new(),
            ls_part: Vec::new(),
        }
    }
        
    pub fn generate(&mut self){
        match self.g_ast.clone() {
            Ast::SEQ(v) => {self.namespace(&v, 0);}
            _ => {}
        }
    }

    fn namespace(&mut self, v: &Vec<Ast>, i: usize){
        match v[i].clone(){
            Ast::Import { path } => {
                self.ls_import.push(path);
                self.namespace(v, i+1);
            }
            Ast::Namespace { path, body } => {
                self.path = path;
                self.component(body);
            }
            _ => {}
        }
    }

    fn component(&mut self, b: Box<Ast>){
        match *b {
            Ast::Component { name, specializes, generic, body } => {
                self.name_component = name;
                self.service(body);
            }
            _ => {}
        }
    }

    fn service(&mut self, b: Box<Ast>){
        match *b {
            Ast::SEQ(v) => { self.vec_service(&v, 0); }
            _ => {}
        }
    }

    fn vec_service(&mut self, v: &Vec<Ast>, i: usize){
        if i<v.len(){
            match v[i].clone() {
                Ast::Requires { name, type_name } => {
                    self.ls_requires.push((name, type_name));
                    self.vec_service(v, i+1);
                }
                Ast::Provides { name, type_name, source } => {
                    self.ls_provides.push((name, type_name, source));
                    self.vec_service(v, i+1);
                }
                Ast::Part { name, type_name, generic, body } => {
                    match *body {
                        Ast::SEQ(v) => {
                            if v.len()!=0 {
                                match v[0].clone() {
                                    Ast::Bind { name, target } => {
                                        self.ls_part.push((name, type_name, target));
                                    }
                                    _ => {
                                        self.ls_part.push((name, type_name, Vec::new()));
                                    }
                                }
                            } else {
                                self.ls_part.push((name, type_name, Vec::new()));
                            }
                            
                        }
                        _ => {}
                    }
                    self.vec_service(v, i+1);
                }
                _ => {}
            }
        }
        else{
           self.write_file();
        }
    }

    fn write_file(&mut self){
        
        //Create folder
        let mut f_path: String = String::from("./src");
        let mut i = 0;
        while i < self.path.len() {
            f_path.push('/');
            f_path += &self.path[i];
            i+=1;
        }

        create_dir_all(f_path.clone());


        //Create file path
        f_path.push('/');
        f_path += &self.name_component;
        f_path.push_str(".py");


        let mut wr = String::new();
        //Add imports
        i = 0;
        while i < self.ls_import.len() {
            wr.push_str("from ");
            wr += &self.ls_import[i][0];
            let mut j = 1;
            while j < self.ls_import[i].len() {
                wr.push('.');
                wr += &self.ls_import[i][j];
                j+=1;
            }
            wr.push_str(" import *");
            wr.push('\n');
            i+=1;
        }


        //Create class
        wr.push_str("\nclass ");
        wr += &self.name_component;
        wr.push_str(" :\n");


        //Create init
        wr.push_str("\tdef __init__(self");
        let mut body = String::new();

        i = 0;
        while i < self.ls_requires.len() {
            wr.push_str(", ");
            wr += &self.ls_requires[i].0;
            wr.push_str(" : ");
            wr += &self.ls_requires[i].1;

            body.push_str("\t\tself.");
            body += &self.ls_requires[i].0;
            body.push_str(" = ");
            body += &self.ls_requires[i].0;
            body.push('\n');

            i+=1
        }

        i = 0;
        while i < self.ls_part.len() {
            body.push_str("\t\tself.");
            body += &self.ls_part[i].0;
            body.push_str(" = ");
            body += &self.ls_part[i].1;
            body.push('(');
            
            let mut j = 0;
            let targ = self.ls_part[i].2.clone();

            if targ.len()!=0{
                body.push_str("self");
            }
            
            while j < targ.len(){
                body.push('.');
                body += &targ[j];

                j+=1;
            }

            body.push_str(")\n");

            i+=1;
        }

        wr.push_str("):\n");

        body.push_str("\t\treturn\n");
        wr += &body;


        //Add provided methods
        i = 0;
        while i < self.ls_provides.len() {
            wr.push_str("\n\tdef ");
            wr += &self.ls_provides[i].0;
            wr.push_str("(self) -> ");
            wr += &self.ls_provides[i].1;
            wr.push_str(":\n\t\treturn");

            
            let src = self.ls_provides[i].2.clone();
            match src {
                None => {}
                Some(v) => {
                    let mut j = 0;
                    wr.push_str(" self");

                    while j < v.len(){
                        wr.push('.');
                        wr += &v[j];

                        j+=1;
                    }
                    wr.push_str("()\n");
                }
            }

            i+=1;
        }


        //Create and fill file
        fs::write(&f_path, wr);
    }
}