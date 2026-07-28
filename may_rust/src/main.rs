use crate::modules::python::GenPython;
use crate::modules::speadl::ast::Ast;
use crate::modules::speadl::parser::Parser;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::{read_dir, read_to_string};
use std::io;
use std::path::{Path, PathBuf};

pub mod modules;

fn main() -> Result<(), Box<dyn Error>> {
    let options = CliOptions::parse()?;
    let input_paths = options.input_paths()?;
    let output_paths = options.output_paths(input_paths.len())?;
    let mut parsed_inputs = Vec::new();

    for (input_path, output_path) in input_paths.into_iter().zip(output_paths) {
        parsed_inputs.push((output_path, parse_input(&input_path)?));
    }

    let dependency_components = dependency_component_names(&parsed_inputs);

    let mut generated_components = HashSet::new();

    for (output_path, ast) in &parsed_inputs {
        generate_import_dependencies(
            ast,
            output_path.clone(),
            options.keep_intermediate,
            &mut generated_components,
        )?;
    }

    for (output_path, ast) in parsed_inputs {
        let print_ast = !component_name(&ast)
            .as_ref()
            .is_some_and(|name| dependency_components.contains(name));

        generate_python(
            ast,
            output_path,
            options.keep_intermediate,
            print_ast,
            &mut generated_components,
        )?;
    }

    Ok(())
}

fn generate_import_dependencies(
    ast: &Ast,
    output_path: Option<PathBuf>,
    keep_intermediate: bool,
    generated_components: &mut HashSet<String>,
) -> Result<(), Box<dyn Error>> {
    match ast {
        Ast::SEQ(nodes) => {
            for node in nodes {
                generate_import_dependencies(
                    node,
                    output_path.clone(),
                    keep_intermediate,
                    generated_components,
                )?;
            }
        }
        Ast::Import {
            ast: Some(import_ast),
            ..
        } => {
            generate_import_dependencies(
                import_ast,
                output_path.clone(),
                keep_intermediate,
                generated_components,
            )?;
            generate_python(
                import_ast.as_ref().clone(),
                output_path,
                keep_intermediate,
                false,
                generated_components,
            )?;
        }
        _ => {}
    }

    Ok(())
}

fn parse_input(input_path: &Path) -> Result<Ast, Box<dyn Error>> {
    let source = read_to_string(input_path)?;
    let mut parser = Parser::new_with_path(&source, input_path);

    parser.next_token();
    Ok(parser.namespace())
}

fn generate_python(
    ast: Ast,
    output_path: Option<PathBuf>,
    keep_intermediate: bool,
    print_ast: bool,
    generated_components: &mut HashSet<String>,
) -> Result<(), Box<dyn Error>> {
    if let Some(name) = component_name(&ast)
        && !generated_components.insert(name)
    {
        return Ok(());
    }

    if print_ast {
        println!("Syntaxe valide");
        println!("{:#?}", ast);
    }

    let gen_p = GenPython::new(ast)
        .with_keep_intermediate(keep_intermediate)
        .with_output(output_path);
    gen_p.generate()?;

    Ok(())
}

fn dependency_component_names(parsed_inputs: &[(Option<PathBuf>, Ast)]) -> HashSet<String> {
    let mut names = HashSet::new();

    for (_, ast) in parsed_inputs {
        collect_dependency_component_names(ast, &mut names);
    }

    names
}

fn collect_dependency_component_names(ast: &Ast, names: &mut HashSet<String>) {
    match ast {
        Ast::SEQ(nodes) => {
            for node in nodes {
                collect_dependency_component_names(node, names);
            }
        }
        Ast::Import {
            ast: Some(import_ast),
            ..
        } => {
            if let Some(name) = component_name(import_ast) {
                names.insert(name);
            }
            collect_dependency_component_names(import_ast, names);
        }
        _ => {}
    }
}

fn component_name(ast: &Ast) -> Option<String> {
    match ast {
        Ast::SEQ(nodes) => nodes.iter().find_map(component_name),
        Ast::Namespace { body, .. } => component_name(body),
        Ast::Component { name, .. } => Some(name.clone()),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct CliOptions {
    keep_intermediate: bool,
    inputs: Vec<PathBuf>,
    outputs: Vec<PathBuf>,
}

impl CliOptions {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut options = Self::default();

        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--keep-intermediate" => options.keep_intermediate = true,
                "-i" | "--input" => options.inputs.push(next_path_arg(&mut args, &arg)?),
                "-o" | "--output" => options.outputs.push(next_path_arg(&mut args, &arg)?),
                _ => {
                    if let Some(value) = arg.strip_prefix("--input=") {
                        options.inputs.push(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--output=") {
                        options.outputs.push(PathBuf::from(value));
                    } else {
                        return Err(invalid_input(format!("unknown argument `{arg}`")));
                    }
                }
            }
        }

        Ok(options)
    }

    fn input_paths(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let inputs = if self.inputs.is_empty() {
            speadl_files_in(&default_input_dir())?
        } else {
            let mut expanded = Vec::new();

            for input in &self.inputs {
                if input.is_dir() {
                    expanded.extend(speadl_files_in(input)?);
                } else {
                    expanded.push(input.clone());
                }
            }

            expanded
        };

        if inputs.is_empty() {
            return Err(invalid_input(format!(
                "no input files found in `{}`",
                default_input_dir().display()
            )));
        }

        Ok(inputs)
    }

    fn output_paths(&self, input_count: usize) -> Result<Vec<Option<PathBuf>>, Box<dyn Error>> {
        match self.outputs.len() {
            0 => Ok(vec![None; input_count]),
            1 if input_count == 1 => Ok(vec![Some(self.outputs[0].clone())]),
            1 => {
                let output = self.outputs[0].clone();
                if path_looks_like_file(&output) {
                    return Err(invalid_input(
                        "a single output file cannot be used with multiple inputs; pass an output directory or one `-o` per input",
                    ));
                }

                Ok(vec![Some(output); input_count])
            }
            count if count == input_count => Ok(self.outputs.iter().cloned().map(Some).collect()),
            count => Err(invalid_input(format!(
                "received {count} output paths for {input_count} input files"
            ))),
        }
    }
}

fn next_path_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input(format!("missing path after `{flag}`")))
}

fn default_input_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("all")
        .join("ex1")
}

fn speadl_files_in(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = Vec::new();

    for entry in read_dir(dir)? {
        let path = entry?.path();

        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("speadl") {
            paths.push(path);
        }
    }

    sort_paths(&mut paths);
    Ok(paths)
}

fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| {
        left.to_string_lossy()
            .to_lowercase()
            .cmp(&right.to_string_lossy().to_lowercase())
    });
}

fn path_looks_like_file(path: &Path) -> bool {
    !path.is_dir() && path.extension().is_some()
}

fn invalid_input(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}
