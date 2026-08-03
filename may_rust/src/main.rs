use crate::modules::java::GenJava;
use crate::modules::python::GenPython;
use crate::modules::speadl::ast::Ast;
use crate::modules::speadl::parser::Parser;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::{read_dir, read_to_string};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub mod modules;

const HELP_TEXT: &str = "\
MAY Rust — Générateur de composants SpeADL

Usage:
  may_rust --target <python|java> --input <PATH> --output <PATH> [OPTIONS]

Options:
  -t, --target, --language <TARGET>  Cible obligatoire : `python` ou `java`
  -i, --input <PATH>                 Entrée SpeADL obligatoire (répétable)
  -o, --output <PATH>                Sortie obligatoire (répétable)
      --keep-intermediate            Conserve la représentation intermédiaire
  -h, --help                         Affiche cette aide

Comportement:
  Une entrée peut être un fichier `.speadl` ou un dossier parcouru récursivement.
  Pour plusieurs entrées, une sortie unique doit être un dossier.

Exemples:
  may_rust --target python -i ../speadl/ex1 -o ../output/python
  may_rust --target java   -i ../speadl/ex1 -o ../output/java
";

fn main() -> ExitCode {
    let options = match CliOptions::parse() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Utilisez `--help` pour afficher l'aide.");
            return ExitCode::from(2);
        }
    };

    if options.help {
        print_help();
        return ExitCode::SUCCESS;
    }

    let Some(target) = options.target else {
        return print_usage_error("vous devez spécifier `--target python` ou `--target java`.");
    };
    if options.inputs.is_empty() {
        return print_usage_error("vous devez spécifier une entrée SpeADL avec `--input <PATH>`.");
    }
    if options.outputs.is_empty() {
        return print_usage_error("vous devez spécifier une sortie avec `--output <PATH>`.");
    };

    match run(options, target) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    print!("{HELP_TEXT}");
}

fn print_usage_error(message: &str) -> ExitCode {
    println!("Error: {message}\n");
    print_help();
    ExitCode::from(2)
}

fn run(options: CliOptions, target: GenerationTarget) -> Result<(), Box<dyn Error>> {
    let input_paths = options.input_paths()?;
    let output_paths = options.output_paths(input_paths.len())?;
    let mut parsed_inputs = Vec::new();

    for (input_path, output_path) in input_paths.into_iter().zip(output_paths) {
        parsed_inputs.push((output_path, parse_input(&input_path)?));
    }

    let dependency_components = dependency_component_ids(&parsed_inputs);
    let component_catalog = parsed_inputs
        .iter()
        .map(|(_, ast)| ast.clone())
        .collect::<Vec<_>>();

    let mut generated_components = HashSet::new();

    for (output_path, ast) in &parsed_inputs {
        generate_import_dependencies(
            ast,
            output_path.clone(),
            options.keep_intermediate,
            target,
            &component_catalog,
            &mut generated_components,
        )?;
    }

    for (output_path, ast) in parsed_inputs {
        let print_ast = !component_id(&ast)
            .as_ref()
            .is_some_and(|id| dependency_components.contains(id));

        generate_component(
            ast,
            output_path,
            options.keep_intermediate,
            print_ast,
            target,
            &component_catalog,
            &mut generated_components,
        )?;
    }

    Ok(())
}

fn generate_import_dependencies(
    ast: &Ast,
    output_path: Option<PathBuf>,
    keep_intermediate: bool,
    target: GenerationTarget,
    component_catalog: &[Ast],
    generated_components: &mut HashSet<String>,
) -> Result<(), Box<dyn Error>> {
    match ast {
        Ast::SEQ(nodes) => {
            for node in nodes {
                generate_import_dependencies(
                    node,
                    output_path.clone(),
                    keep_intermediate,
                    target,
                    component_catalog,
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
                target,
                component_catalog,
                generated_components,
            )?;
            generate_component(
                import_ast.as_ref().clone(),
                dependency_output(output_path),
                keep_intermediate,
                false,
                target,
                component_catalog,
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

    parser.next_token()?;
    Ok(parser.namespace()?)
}

fn generate_component(
    ast: Ast,
    output_path: Option<PathBuf>,
    keep_intermediate: bool,
    print_ast: bool,
    target: GenerationTarget,
    component_catalog: &[Ast],
    generated_components: &mut HashSet<String>,
) -> Result<(), Box<dyn Error>> {
    if let Some(id) = component_id(&ast)
        && !generated_components.insert(id)
    {
        return Ok(());
    }

    if print_ast {
        println!("Syntaxe valide");
        println!("{:#?}", ast);
    }

    match target {
        GenerationTarget::Python => {
            GenPython::new(ast)
                .with_dependencies(component_catalog.to_vec())
                .with_keep_intermediate(keep_intermediate)
                .with_output(output_path)
                .generate()?;
        }
        GenerationTarget::Java => {
            GenJava::new(ast)
                .with_dependencies(component_catalog.to_vec())
                .with_keep_intermediate(keep_intermediate)
                .with_output(output_path)
                .generate()?;
        }
    }

    Ok(())
}

fn dependency_component_ids(parsed_inputs: &[(Option<PathBuf>, Ast)]) -> HashSet<String> {
    let mut ids = HashSet::new();

    for (_, ast) in parsed_inputs {
        collect_dependency_component_ids(ast, &mut ids);
    }

    ids
}

fn collect_dependency_component_ids(ast: &Ast, ids: &mut HashSet<String>) {
    match ast {
        Ast::SEQ(nodes) => {
            for node in nodes {
                collect_dependency_component_ids(node, ids);
            }
        }
        Ast::Import {
            ast: Some(import_ast),
            ..
        } => {
            if let Some(id) = component_id(import_ast) {
                ids.insert(id);
            }
            collect_dependency_component_ids(import_ast, ids);
        }
        _ => {}
    }
}

fn component_id(ast: &Ast) -> Option<String> {
    match ast {
        Ast::SEQ(nodes) => nodes.iter().find_map(component_id),
        Ast::Namespace { path, body } => component_name(body).map(|name| {
            if path.is_empty() {
                name
            } else {
                format!("{}.{}", path.join("."), name)
            }
        }),
        _ => None,
    }
}

fn component_name(ast: &Ast) -> Option<String> {
    match ast {
        Ast::Component { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn dependency_output(output_path: Option<PathBuf>) -> Option<PathBuf> {
    output_path.map(|path| {
        if path_looks_like_file(&path) {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            path
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerationTarget {
    Java,
    Python,
}

impl GenerationTarget {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.to_ascii_lowercase().as_str() {
            "java" => Ok(Self::Java),
            "python" | "py" => Ok(Self::Python),
            _ => Err(invalid_input(format!(
                "unknown target `{value}`; expected `java` or `python`"
            ))),
        }
    }
}

#[derive(Debug, Default)]
struct CliOptions {
    keep_intermediate: bool,
    help: bool,
    target: Option<GenerationTarget>,
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
                "-h" | "--help" => options.help = true,
                "-t" | "--target" | "--language" => {
                    options.target = Some(next_target_arg(&mut args, &arg)?)
                }
                "-i" | "--input" => options.inputs.push(next_path_arg(&mut args, &arg)?),
                "-o" | "--output" => options.outputs.push(next_path_arg(&mut args, &arg)?),
                _ => {
                    if let Some(value) = arg.strip_prefix("--input=") {
                        options.inputs.push(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--output=") {
                        options.outputs.push(PathBuf::from(value));
                    } else if let Some(value) = arg
                        .strip_prefix("--target=")
                        .or_else(|| arg.strip_prefix("--language="))
                    {
                        options.target = Some(GenerationTarget::parse(value)?);
                    } else {
                        return Err(invalid_input(format!("unknown argument `{arg}`")));
                    }
                }
            }
        }

        Ok(options)
    }

    fn input_paths(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        if self.inputs.is_empty() {
            return Err(invalid_input("at least one input path is required"));
        }

        let mut inputs = Vec::new();
        for input in &self.inputs {
            if input.is_dir() {
                inputs.extend(speadl_files_in(input)?);
            } else {
                inputs.push(input.clone());
            }
        }

        if inputs.is_empty() {
            return Err(invalid_input("no SpeADL input files were found"));
        }

        Ok(inputs)
    }

    fn output_paths(&self, input_count: usize) -> Result<Vec<Option<PathBuf>>, Box<dyn Error>> {
        match self.outputs.len() {
            0 => Err(invalid_input("at least one output path is required")),
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

fn next_target_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<GenerationTarget, Box<dyn Error>> {
    let value = args
        .next()
        .ok_or_else(|| invalid_input(format!("missing target after `{flag}`")))?;
    GenerationTarget::parse(&value)
}

fn next_path_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input(format!("missing path after `{flag}`")))
}

fn speadl_files_in(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = Vec::new();
    let mut directories = vec![dir.to_path_buf()];

    while let Some(directory) = directories.pop() {
        for entry in read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let path = entry.path();

            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("speadl")
            {
                paths.push(path);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_generation_targets() {
        assert_eq!(
            GenerationTarget::parse("java").expect("Java should be accepted"),
            GenerationTarget::Java
        );
        assert_eq!(
            GenerationTarget::parse("PY").expect("py should be accepted"),
            GenerationTarget::Python
        );
        assert!(GenerationTarget::parse("kotlin").is_err());
    }

    #[test]
    fn target_is_not_selected_implicitly() {
        assert_eq!(CliOptions::default().target, None);
        assert!(HELP_TEXT.contains("--target <python|java>"));
        assert!(HELP_TEXT.contains("Cible obligatoire"));
    }

    #[test]
    fn input_and_output_are_not_selected_implicitly() {
        let options = CliOptions::default();

        assert!(options.input_paths().is_err());
        assert!(options.output_paths(1).is_err());
        assert!(HELP_TEXT.contains("--input <PATH>"));
        assert!(HELP_TEXT.contains("--output <PATH>"));
    }

    #[test]
    fn component_ids_include_the_namespace() {
        let ast = Ast::SEQ(vec![Ast::Namespace {
            path: vec![String::from("one"), String::from("two")],
            body: Box::new(Ast::Component {
                name: String::from("Example"),
                specializes: None,
                generic: None,
                body: Box::new(Ast::SEQ(Vec::new())),
            }),
        }]);

        assert_eq!(component_id(&ast).as_deref(), Some("one.two.Example"));
    }

    #[test]
    fn input_directories_are_expanded_recursively() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "may-rust-cli-input-{}-{unique}",
            std::process::id()
        ));
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("fixture directories should be created");
        fs::write(root.join("Root.speadl"), "").expect("root fixture should be written");
        fs::write(nested.join("Nested.speadl"), "").expect("nested fixture should be written");
        fs::write(nested.join("Ignored.java"), "").expect("ignored fixture should be written");

        let paths = speadl_files_in(&root).expect("directory expansion should succeed");

        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("Nested.speadl"));
        assert!(paths[1].ends_with("Root.speadl"));
        fs::remove_dir_all(root).expect("fixture directory should be removed");
    }
}
