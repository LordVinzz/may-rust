use super::ast::{
    PyAlias, PyArg, PyArguments, PyClassDef, PyExpr, PyFunctionDef, PyImportFrom,
    PyModule, PyStmt,
};
use crate::modules::speadl::ast::{Ast, ProvidedServiceImplementation, Specializes};
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::io;
use std::path::{Path, PathBuf};

const GENERATED_PYTHON_EXAMPLES_DIR: &str = "examples/python";

pub struct GenPython {
    ast: Ast,
    options: GeneratorOptions,
}

#[derive(Debug, Clone, Default)]
pub struct GeneratorOptions {
    keep_intermediate: bool,
    output: Option<PathBuf>,
}

struct PythonComponent {
    namespace: Vec<String>,
    imports: Vec<ImportList>,
    name: String,
    specializes: Option<SpecializedParent>,
    required_services: Vec<RequiredService>,
    provided_services: Vec<ProvidedService>,
    part_instances: Vec<PartInstance>,
}

struct ImportList {
    path: Vec<String>,
}

struct RequiredService {
    name: String,
    type_name: String,
}

struct SpecializedParent {
    name: String,
    required_services: Vec<RequiredService>,
}

struct ProvidedService {
    name: String,
    type_name: String,
    implementation: ProvidedServiceImplementation,
}

struct PartInstance {
    name: String,
    type_name: String,
    bindings: Vec<PartBinding>,
}

struct PartBinding {
    parameter_name: String,
    target: Vec<String>,
}

impl GenPython {
    pub fn new(ast: Ast) -> Self {
        Self {
            ast,
            options: GeneratorOptions::default(),
        }
    }

    pub fn with_options(ast: Ast, options: GeneratorOptions) -> Self {
        Self { ast, options }
    }

    pub fn with_keep_intermediate(mut self, keep_intermediate: bool) -> Self {
        self.options.keep_intermediate = keep_intermediate;
        self
    }

    pub fn with_output(mut self, output: Option<PathBuf>) -> Self {
        self.options.output = output;
        self
    }

    pub fn generate(&self) -> Result<(), Box<dyn Error>> {
        let component = PythonComponent::from_speadl_ast(&self.ast)?;
        let output_path = component.output_path(self.options.output.as_deref());
        let python_module = component.to_python_module();

        if let Some(parent) = output_path.parent() {
            create_dir_all(parent)?;
        }

        let mut source = if self.options.keep_intermediate {
            python_module.unparse_and_keep_intermediate(&intermediate_output_path(&output_path))?
        } else {
            python_module.unparse()?
        };

        if !source.ends_with('\n') {
            source.push('\n');
        }

        fs::write(output_path, source)?;

        Ok(())
    }
}

impl PythonComponent {
    fn from_speadl_ast(ast: &Ast) -> Result<Self, Box<dyn Error>> {
        let Ast::SEQ(nodes) = ast else {
            return Err(invalid_ast(
                "Python generation expects a top-level sequence",
            ));
        };

        let mut imports = Vec::new();

        for node in nodes {
            match node {
                Ast::Import { path, .. } => {
                    imports.push(ImportList { path: path.clone() });
                }
                Ast::Namespace { path, body } => {
                    return Self::from_namespace(path.clone(), imports, body);
                }
                _ => {}
            }
        }

        Err(invalid_ast(
            "Python generation expects a namespace after imports",
        ))
    }

    fn from_namespace(
        namespace: Vec<String>,
        imports: Vec<ImportList>,
        body: &Ast,
    ) -> Result<Self, Box<dyn Error>> {
        let Ast::Component {
            name,
            specializes,
            body,
            ..
        } = body
        else {
            return Err(invalid_ast(
                "Python generation expects a component in the namespace",
            ));
        };

        let mut component = Self {
            namespace,
            imports,
            name: name.clone(),
            specializes: specialized_parent(specializes.as_ref())?,
            required_services: Vec::new(),
            provided_services: Vec::new(),
            part_instances: Vec::new(),
        };

        component.read_component_body(body)?;

        Ok(component)
    }

    fn read_component_body(&mut self, body: &Ast) -> Result<(), Box<dyn Error>> {
        let Ast::SEQ(nodes) = body else {
            return Err(invalid_ast(
                "Python generation expects a component body sequence",
            ));
        };

        for node in nodes {
            match node {
                Ast::Requires { name, type_name } => {
                    self.required_services.push(RequiredService {
                        name: name.clone(),
                        type_name: type_name.clone(),
                    });
                }
                Ast::Provides {
                    name,
                    type_name,
                    implementation,
                } => {
                    self.provided_services.push(ProvidedService {
                        name: name.clone(),
                        type_name: type_name.clone(),
                        implementation: implementation.clone(),
                    });
                }
                Ast::Part {
                    name,
                    type_name,
                    body,
                    ..
                } => {
                    self.part_instances.push(PartInstance {
                        name: name.clone(),
                        type_name: type_name.clone(),
                        bindings: part_bindings(body)?,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn to_python_module(&self) -> PyModule {
        let mut body = Vec::new();

        for import in &self.imports {
            body.push(PyStmt::ImportFrom(PyImportFrom {
                module: import.path.join("."),
                names: vec![PyAlias::import_all()],
                level: 0,
            }));
        }

        body.push(PyStmt::ClassDef(PyClassDef {
            name: self.name.clone(),
            bases: self.class_bases(),
            body: self.class_body(),
        }));

        PyModule { body }
    }

    fn class_body(&self) -> Vec<PyStmt> {
        let mut body = vec![self.init_function()];

        for provided_service in &self.provided_services {
            body.push(provided_service.to_python_function());
        }

        body
    }

    fn class_bases(&self) -> Vec<PyExpr> {
        self.specializes
            .as_ref()
            .map(|parent| vec![PyExpr::load_name(parent.name.clone())])
            .unwrap_or_default()
    }

    fn init_function(&self) -> PyStmt {
        let mut args = vec![PyArg::without_annotation("self")];

        for required_service in self.init_required_services() {
            args.push(PyArg::with_annotation(
                required_service.name.clone(),
                python_type_annotation(&required_service.type_name),
            ));
        }

        for part_instance in self.init_part_instance() {
            args.push(PyArg::with_annotation(
                part_instance.name.clone(),
                python_type_annotation(&part_instance.type_name),
            ));
        }

        let mut body = Vec::new();

        if let Some(parent) = &self.specializes {
            body.push(PyStmt::Expr(PyExpr::call(
                PyExpr::load_attribute(
                    PyExpr::call(PyExpr::load_name("super"), Vec::new()),
                    "__init__",
                ),
                parent
                    .required_services
                    .iter()
                    .map(|required_service| PyExpr::load_name(required_service.name.clone()))
                    .collect(),
            )));
        }

        for required_service in &self.required_services {
            body.push(PyStmt::Assign {
                targets: vec![self_store_attribute(&required_service.name)],
                value: PyExpr::load_name(required_service.name.clone()),
            });
        }

        for part_instance in &self.part_instances {
            body.push(PyStmt::Assign {
                targets: vec![self_store_attribute(&part_instance.name)],
                value: PyExpr::load_name(part_instance.name.clone()),
            });
        }

        for part_instance in &self.part_instances {
            let mut ind = 0;
            while ind < part_instance.bindings.len(){
                body.push(part_instance.to_python_assignment(ind));
                ind += 1;
            }
        }

        body.push(PyStmt::Return(None));

        PyStmt::FunctionDef(PyFunctionDef {
            name: String::from("__init__"),
            args: PyArguments::new(args),
            body,
            returns: None,
        })
    }

    fn init_required_services(&self) -> Vec<&RequiredService> {
        let mut required_services = Vec::new();

        if let Some(parent) = &self.specializes {
            for required_service in &parent.required_services {
                required_services.push(required_service);
            }
        }

        for required_service in &self.required_services {
            if !required_services
                .iter()
                .any(|existing| existing.name == required_service.name)
            {
                required_services.push(required_service);
            }
        }

        required_services
    }

    fn init_part_instance(&self) -> Vec<&PartInstance> {
        let mut part_instances = Vec::new();

        for part_instance in &self.part_instances {
            if !part_instances
                .iter()
                .any(|existing: &&PartInstance| existing.name == part_instance.name)
            {
                part_instances.push(part_instance);
            }
        }

        part_instances
    }

    fn output_path(&self, output: Option<&Path>) -> PathBuf {
        match output {
            Some(path) if output_target_is_file(path) => path.to_path_buf(),
            Some(root) => self.output_path_under(root),
            None => self.output_path_under(&default_output_root()),
        }
    }

    fn output_path_under(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();

        for namespace_part in &self.namespace {
            path.push(namespace_part);
        }

        path.push(format!("{}.py", self.name));
        path
    }
}

fn specialized_parent(
    specializes: Option<&Specializes>,
) -> Result<Option<SpecializedParent>, Box<dyn Error>> {
    let Some(specializes) = specializes else {
        return Ok(None);
    };

    let Some(parent_file) = &specializes.parent_file else {
        return Ok(Some(SpecializedParent {
            name: specializes.parent.clone(),
            required_services: Vec::new(),
        }));
    };

    Ok(Some(SpecializedParent {
        name: specializes.parent.clone(),
        required_services: component_required_services(parent_file)?,
    }))
}

fn component_required_services(ast: &Ast) -> Result<Vec<RequiredService>, Box<dyn Error>> {
    let Ast::SEQ(nodes) = ast else {
        return Err(invalid_ast(
            "Python generation expects a top-level sequence",
        ));
    };

    for node in nodes {
        if let Ast::Namespace { body, .. } = node {
            let Ast::Component { body, .. } = body.as_ref() else {
                return Err(invalid_ast(
                    "Python generation expects a component in the namespace",
                ));
            };
            let Ast::SEQ(nodes) = body.as_ref() else {
                return Err(invalid_ast(
                    "Python generation expects a component body sequence",
                ));
            };

            return Ok(nodes
                .iter()
                .filter_map(|node| {
                    if let Ast::Requires { name, type_name } = node {
                        Some(RequiredService {
                            name: name.clone(),
                            type_name: type_name.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect());
        }
    }

    Err(invalid_ast(
        "Python generation expects a namespace after imports",
    ))
}

impl GeneratorOptions {
    pub fn keep_intermediate(mut self, keep_intermediate: bool) -> Self {
        self.keep_intermediate = keep_intermediate;
        self
    }

    pub fn output(mut self, output: Option<PathBuf>) -> Self {
        self.output = output;
        self
    }
}

impl ProvidedService {
    fn to_python_function(&self) -> PyStmt {
        PyStmt::FunctionDef(PyFunctionDef {
            name: self.name.clone(),
            args: PyArguments::new(vec![PyArg::without_annotation("self")]),
            body: vec![PyStmt::Return(self.return_value())],
            returns: Some(python_type_annotation(&self.type_name)),
        })
    }

    fn return_value(&self) -> Option<PyExpr> {
        match &self.implementation {
            ProvidedServiceImplementation::Local => None,
            ProvidedServiceImplementation::Delegated(service_reference) => Some(PyExpr::call(
                self_load_path(&[
                    service_reference.part_name.clone(),
                    service_reference.service_name.clone(),
                ]),
                Vec::new(),
            )),
        }
    }
}

impl PartInstance {
    fn to_python_assignment(&self, ind: usize) -> PyStmt {
        PyStmt::Assign {
            targets: vec![self_store_attribute(&vec![self.name.clone(), self.bindings[ind].parameter_name.clone()].join("."))],
            value: PyExpr::load_name(vec![String::from("self"), self.bindings[ind].target.join(".")].join("."))
        }
    }
}

fn part_bindings(body: &Ast) -> Result<Vec<PartBinding>, Box<dyn Error>> {
    let Ast::SEQ(nodes) = body else {
        return Err(invalid_ast(
            "Python generation expects a part body sequence",
        ));
    };

    let mut bindings = Vec::new();

    for node in nodes {
        if let Ast::Bind { name, target } = node {
            bindings.push(PartBinding {
                parameter_name: name.clone(),
                target: target.clone(),
            });
        }
    }

    Ok(bindings)
}

fn python_type_annotation(type_name: &str) -> PyExpr {
    PyExpr::load_name(type_name)
}

fn self_store_attribute(name: &str) -> PyExpr {
    PyExpr::store_attribute(PyExpr::load_name("self"), name)
}

fn self_load_path(path: &[String]) -> PyExpr {
    let mut expr = PyExpr::load_name("self");

    for item in path {
        expr = PyExpr::load_attribute(expr, item);
    }

    expr
}

fn invalid_ast(message: &str) -> Box<dyn Error> {
    Box::new(io::Error::new(
        io::ErrorKind::InvalidData,
        message.to_string(),
    ))
}

fn default_output_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(GENERATED_PYTHON_EXAMPLES_DIR)
}

fn output_target_is_file(path: &Path) -> bool {
    !path.is_dir() && path.extension().is_some()
}

fn intermediate_output_path(output_path: &Path) -> PathBuf {
    let file_stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("python_ast_unparse");
    let mut path = output_path.to_path_buf();

    path.set_file_name(format!("{file_stem}.python_ast_unparse.py"));
    path
}
