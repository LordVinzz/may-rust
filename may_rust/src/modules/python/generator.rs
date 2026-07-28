use super::ast::{
    PyAlias, PyArg, PyArguments, PyClassDef, PyExpr, PyFunctionDef, PyIf, PyImportFrom, PyModule,
    PyStmt,
};
use crate::modules::speadl::ast::{Ast, ProvidedServiceImplementation, ServiceReference};
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::io;
use std::path::{Path, PathBuf};

const GENERATED_PYTHON_EXAMPLES_DIR: &str = "examples/python";
const UNSET_NAME: &str = "_UNSET";

pub struct GenPython {
    ast: Ast,
    options: GeneratorOptions,
}

#[derive(Debug, Clone, Default)]
pub struct GeneratorOptions {
    keep_intermediate: bool,
    output: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct PythonComponent {
    namespace: Vec<String>,
    imports: Vec<ImportList>,
    name: String,
    parent: Option<String>,
    generic: Option<String>,
    required_services: Vec<RequiredService>,
    provided_services: Vec<ProvidedService>,
    parts: Vec<Part>,
}

#[derive(Debug, Clone)]
struct ImportList {
    path: Vec<String>,
}

#[derive(Debug, Clone)]
struct RequiredService {
    name: String,
    type_name: String,
}

#[derive(Debug, Clone)]
struct ProvidedService {
    name: String,
    type_name: String,
    implementation: ProvidedServiceImplementation,
}

#[derive(Debug, Clone)]
struct Part {
    name: String,
    type_name: String,
    generic: Option<String>,
    bindings: Vec<PartBinding>,
}

#[derive(Debug, Clone)]
struct PartBinding {
    required_name: String,
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

    pub fn render(&self) -> Result<String, Box<dyn Error>> {
        let component = PythonComponent::from_speadl_ast(&self.ast)?;
        let mut source = component.to_python_module().unparse()?;
        if !source.ends_with('\n') {
            source.push('\n');
        }
        Ok(source)
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
            generic,
            body,
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
            parent: specializes.as_ref().map(|parent| parent.parent.clone()),
            generic: generic.clone(),
            required_services: Vec::new(),
            provided_services: Vec::new(),
            parts: Vec::new(),
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
                    generic,
                    body,
                } => {
                    self.parts.push(Part {
                        name: name.clone(),
                        type_name: type_name.clone(),
                        generic: generic.clone(),
                        bindings: part_bindings(body)?,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn to_python_module(&self) -> PyModule {
        let mut body = vec![
            import_from("__future__", vec!["annotations"]),
            import_from("abc", vec!["ABC", "abstractmethod"]),
        ];

        if self.generic.is_some() {
            body.push(import_from("typing", vec!["Generic", "TypeVar"]));
        }
        for import in &self.imports {
            body.push(PyStmt::ImportFrom(PyImportFrom {
                module: import.path.join("."),
                names: vec![PyAlias::import_all()],
                level: 0,
            }));
        }

        body.push(assign(
            PyExpr::store_name(UNSET_NAME),
            PyExpr::call(PyExpr::load_name("object"), Vec::new()),
        ));
        if let Some(generic) = &self.generic {
            body.push(assign(
                PyExpr::store_name(generic),
                PyExpr::call(PyExpr::load_name("TypeVar"), vec![PyExpr::string(generic)]),
            ));
        }
        body.push(PyStmt::ClassDef(self.component_class()));
        PyModule { body }
    }

    fn component_class(&self) -> PyClassDef {
        let mut body = vec![self.requires_view_class(), self.parts_view_class()];
        body.push(self.init_function());
        body.push(self.requires_function());
        body.push(self.parts_function());

        for required in &self.required_services {
            body.push(required.bind_function());
        }
        for provided in &self.provided_services {
            body.push(provided.make_function());
            body.push(provided.accessor_function());
        }
        for part in &self.parts {
            body.push(part.make_function());
            body.push(part.get_function());
        }

        PyClassDef {
            name: self.name.clone(),
            bases: self.class_bases(),
            body,
            decorators: Vec::new(),
        }
    }

    fn class_bases(&self) -> Vec<PyExpr> {
        let mut bases = Vec::new();
        if let Some(parent) = &self.parent {
            bases.push(PyExpr::load_name(parent));
        } else {
            bases.push(PyExpr::load_name("ABC"));
        }
        if let Some(generic) = &self.generic {
            bases.insert(0, PyExpr::load_name(format!("Generic[{generic}]")));
        }
        bases
    }

    fn requires_view_class(&self) -> PyStmt {
        let mut body = Vec::new();
        if self.parent.is_none() {
            body.push(owner_init_function());
        }
        for required in &self.required_services {
            body.push(required.view_accessor_function());
        }
        if body.is_empty() {
            body.push(PyStmt::Pass);
        }

        PyStmt::ClassDef(PyClassDef {
            name: String::from("_Requires"),
            bases: self
                .parent
                .as_ref()
                .map(|parent| {
                    vec![PyExpr::load_attribute(
                        PyExpr::load_name(parent),
                        "_Requires",
                    )]
                })
                .unwrap_or_default(),
            body,
            decorators: Vec::new(),
        })
    }

    fn parts_view_class(&self) -> PyStmt {
        let mut body = Vec::new();
        if self.parent.is_none() {
            body.push(owner_init_function());
        }
        for part in &self.parts {
            body.push(part.view_accessor_function());
        }
        if body.is_empty() {
            body.push(PyStmt::Pass);
        }

        PyStmt::ClassDef(PyClassDef {
            name: String::from("_Parts"),
            bases: self
                .parent
                .as_ref()
                .map(|parent| vec![PyExpr::load_attribute(PyExpr::load_name(parent), "_Parts")])
                .unwrap_or_default(),
            body,
            decorators: Vec::new(),
        })
    }

    fn init_function(&self) -> PyStmt {
        let mut body = Vec::new();
        if self.parent.is_some() {
            body.push(PyStmt::Expr(PyExpr::call(
                PyExpr::load_attribute(
                    PyExpr::call(PyExpr::load_name("super"), Vec::new()),
                    "__init__",
                ),
                Vec::new(),
            )));
        }
        for required in &self.required_services {
            body.push(assign(
                self_store_attribute(&required.storage_name()),
                PyExpr::load_name(UNSET_NAME),
            ));
        }
        for provided in &self.provided_services {
            body.push(assign(
                self_store_attribute(&provided.storage_name()),
                PyExpr::load_name(UNSET_NAME),
            ));
        }
        for part in &self.parts {
            body.push(assign(
                self_store_attribute(&part.storage_name()),
                PyExpr::load_name(UNSET_NAME),
            ));
        }
        body.push(assign(
            self_store_attribute("_requires_view"),
            call_self_method("_Requires", vec![PyExpr::load_name("self")]),
        ));
        body.push(assign(
            self_store_attribute("_parts_view"),
            call_self_method("_Parts", vec![PyExpr::load_name("self")]),
        ));

        function("__init__", vec![self_arg()], body, None, Vec::new())
    }

    fn requires_function(&self) -> PyStmt {
        function(
            "requires",
            vec![self_arg()],
            vec![PyStmt::Return(Some(self_load_attribute("_requires_view")))],
            Some(PyExpr::load_name("_Requires")),
            Vec::new(),
        )
    }

    fn parts_function(&self) -> PyStmt {
        function(
            "parts",
            vec![self_arg()],
            vec![PyStmt::Return(Some(self_load_attribute("_parts_view")))],
            Some(PyExpr::load_name("_Parts")),
            Vec::new(),
        )
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

impl RequiredService {
    fn storage_name(&self) -> String {
        format!("_required_{}", self.name)
    }

    fn bind_function(&self) -> PyStmt {
        function(
            format!("_bind_{}", self.name),
            vec![
                self_arg(),
                PyArg::with_annotation("service", python_type_annotation(&self.type_name, None)),
            ],
            vec![assign(
                self_store_attribute(&self.storage_name()),
                PyExpr::load_name("service"),
            )],
            None,
            Vec::new(),
        )
    }

    fn view_accessor_function(&self) -> PyStmt {
        let value = owner_load_attribute(&self.storage_name());
        function(
            &self.name,
            vec![self_arg()],
            vec![
                PyStmt::If(PyIf {
                    test: PyExpr::is(value.clone(), PyExpr::load_name(UNSET_NAME)),
                    body: vec![PyStmt::Raise(PyExpr::call(
                        PyExpr::load_name("RuntimeError"),
                        vec![PyExpr::string(format!(
                            "required service `{}` is not bound",
                            self.name
                        ))],
                    ))],
                    orelse: Vec::new(),
                }),
                PyStmt::Return(Some(value)),
            ],
            Some(python_type_annotation(&self.type_name, None)),
            Vec::new(),
        )
    }
}

impl ProvidedService {
    fn storage_name(&self) -> String {
        format!("_provided_{}", self.name)
    }

    fn make_function(&self) -> PyStmt {
        let (body, decorators) = match &self.implementation {
            ProvidedServiceImplementation::Local => (
                vec![PyStmt::Raise(PyExpr::call(
                    PyExpr::load_name("NotImplementedError"),
                    vec![PyExpr::string(format!(
                        "implement `make_{}` in a concrete component",
                        self.name
                    ))],
                ))],
                vec![PyExpr::load_name("abstractmethod")],
            ),
            ProvidedServiceImplementation::Delegated(reference) => (
                vec![PyStmt::Return(Some(part_service_expression(reference)))],
                Vec::new(),
            ),
        };

        function(
            format!("make_{}", self.name),
            vec![self_arg()],
            body,
            Some(python_type_annotation(&self.type_name, None)),
            decorators,
        )
    }

    fn accessor_function(&self) -> PyStmt {
        let storage = self_load_attribute(&self.storage_name());
        function(
            &self.name,
            vec![self_arg()],
            vec![
                PyStmt::If(PyIf {
                    test: PyExpr::is(storage.clone(), PyExpr::load_name(UNSET_NAME)),
                    body: vec![assign(
                        self_store_attribute(&self.storage_name()),
                        call_self_method(format!("make_{}", self.name), Vec::new()),
                    )],
                    orelse: Vec::new(),
                }),
                PyStmt::Return(Some(storage)),
            ],
            Some(python_type_annotation(&self.type_name, None)),
            Vec::new(),
        )
    }
}

impl Part {
    fn annotation(&self) -> PyExpr {
        python_type_annotation(&self.type_name, self.generic.as_deref())
    }

    fn storage_name(&self) -> String {
        format!("_part_{}_cache", self.name)
    }

    fn make_function(&self) -> PyStmt {
        function(
            format!("make_{}", self.name),
            vec![self_arg()],
            vec![PyStmt::Raise(PyExpr::call(
                PyExpr::load_name("NotImplementedError"),
                vec![PyExpr::string(format!(
                    "implement `make_{}` in a concrete component",
                    self.name
                ))],
            ))],
            Some(self.annotation()),
            vec![PyExpr::load_name("abstractmethod")],
        )
    }

    fn get_function(&self) -> PyStmt {
        let storage = self_load_attribute(&self.storage_name());
        let mut initialize = vec![
            assign(
                PyExpr::store_name("part"),
                call_self_method(format!("make_{}", self.name), Vec::new()),
            ),
            assign(
                self_store_attribute(&self.storage_name()),
                PyExpr::load_name("part"),
            ),
        ];
        for binding in &self.bindings {
            initialize.push(PyStmt::Expr(PyExpr::call(
                PyExpr::load_attribute(
                    PyExpr::load_name("part"),
                    format!("_bind_{}", binding.required_name),
                ),
                vec![binding_source_expression(&binding.target)],
            )));
        }

        function(
            format!("_get_part_{}", self.name),
            vec![self_arg()],
            vec![
                PyStmt::If(PyIf {
                    test: PyExpr::is(storage.clone(), PyExpr::load_name(UNSET_NAME)),
                    body: initialize,
                    orelse: Vec::new(),
                }),
                PyStmt::Return(Some(storage)),
            ],
            Some(self.annotation()),
            Vec::new(),
        )
    }

    fn view_accessor_function(&self) -> PyStmt {
        function(
            &self.name,
            vec![self_arg()],
            vec![PyStmt::Return(Some(call_owner_method(
                format!("_get_part_{}", self.name),
                Vec::new(),
            )))],
            Some(self.annotation()),
            Vec::new(),
        )
    }
}

fn part_bindings(body: &Ast) -> Result<Vec<PartBinding>, Box<dyn Error>> {
    let Ast::SEQ(nodes) = body else {
        return Err(invalid_ast(
            "Python generation expects a part body sequence",
        ));
    };
    Ok(nodes
        .iter()
        .filter_map(|node| match node {
            Ast::Bind { name, target } => Some(PartBinding {
                required_name: name.clone(),
                target: target.clone(),
            }),
            _ => None,
        })
        .collect())
}

fn owner_init_function() -> PyStmt {
    function(
        "__init__",
        vec![self_arg(), PyArg::without_annotation("owner")],
        vec![assign(
            PyExpr::store_attribute(PyExpr::load_name("self"), "_owner"),
            PyExpr::load_name("owner"),
        )],
        None,
        Vec::new(),
    )
}

fn function(
    name: impl Into<String>,
    args: Vec<PyArg>,
    body: Vec<PyStmt>,
    returns: Option<PyExpr>,
    decorators: Vec<PyExpr>,
) -> PyStmt {
    PyStmt::FunctionDef(PyFunctionDef {
        name: name.into(),
        args: PyArguments::new(args),
        body,
        returns,
        decorators,
    })
}

fn import_from(module: impl Into<String>, names: Vec<&str>) -> PyStmt {
    PyStmt::ImportFrom(PyImportFrom {
        module: module.into(),
        names: names
            .into_iter()
            .map(|name| PyAlias {
                name: name.to_string(),
                asname: None,
            })
            .collect(),
        level: 0,
    })
}

fn assign(target: PyExpr, value: PyExpr) -> PyStmt {
    PyStmt::Assign {
        targets: vec![target],
        value,
    }
}

fn self_arg() -> PyArg {
    PyArg::without_annotation("self")
}

fn python_type_annotation(type_name: &str, generic: Option<&str>) -> PyExpr {
    match generic {
        Some(generic) => PyExpr::load_name(format!("{type_name}[{generic}]")),
        None => PyExpr::load_name(type_name),
    }
}

fn self_store_attribute(name: &str) -> PyExpr {
    PyExpr::store_attribute(PyExpr::load_name("self"), name)
}

fn self_load_attribute(name: &str) -> PyExpr {
    PyExpr::load_attribute(PyExpr::load_name("self"), name)
}

fn owner_load_attribute(name: &str) -> PyExpr {
    PyExpr::load_attribute(
        PyExpr::load_attribute(PyExpr::load_name("self"), "_owner"),
        name,
    )
}

fn call_self_method(name: impl Into<String>, args: Vec<PyExpr>) -> PyExpr {
    PyExpr::call(
        PyExpr::load_attribute(PyExpr::load_name("self"), name),
        args,
    )
}

fn call_owner_method(name: impl Into<String>, args: Vec<PyExpr>) -> PyExpr {
    PyExpr::call(
        PyExpr::load_attribute(
            PyExpr::load_attribute(PyExpr::load_name("self"), "_owner"),
            name,
        ),
        args,
    )
}

fn part_service_expression(reference: &ServiceReference) -> PyExpr {
    let part = PyExpr::call(
        PyExpr::load_attribute(call_self_method("parts", Vec::new()), &reference.part_name),
        Vec::new(),
    );
    PyExpr::call(
        PyExpr::load_attribute(part, &reference.service_name),
        Vec::new(),
    )
}

fn binding_source_expression(target: &[String]) -> PyExpr {
    match target {
        [required] => PyExpr::call(
            PyExpr::load_attribute(call_self_method("requires", Vec::new()), required),
            Vec::new(),
        ),
        [part_name, service_name] => part_service_expression(&ServiceReference {
            part_name: part_name.clone(),
            service_name: service_name.clone(),
        }),
        _ => PyExpr::call(
            PyExpr::load_name("RuntimeError"),
            vec![PyExpr::string("invalid SPEADL binding target")],
        ),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::speadl::parser::Parser;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse(source: &str) -> Ast {
        let mut parser = Parser::new(source);
        parser.next_token();
        parser.namespace()
    }

    #[test]
    fn local_services_are_abstract_factories_with_cached_accessors() {
        let source = GenPython::new(parse(
            "namespace demo { component Simple { provides starter: Start } }",
        ))
        .render()
        .expect("generation should succeed");

        assert!(source.contains("class Simple(ABC):"));
        assert!(source.contains("@abstractmethod\n    def make_starter(self) -> Start:"));
        assert!(source.contains("self._provided_starter = self.make_starter()"));
        assert!(source.contains("def starter(self) -> Start:"));
    }

    #[test]
    fn parts_bindings_and_delegations_are_generated_automatically() {
        let source = GenPython::new(parse(
            r#"
            namespace demo {
                component Composite {
                    provides service: Runnable = client.letsgo
                    part simple: Simple {}
                    part client: Client {
                        bind demarreur to simple.starter
                    }
                }
            }
            "#,
        ))
        .render()
        .expect("generation should succeed");

        assert!(source.contains("def make_simple(self) -> Simple:"));
        assert!(source.contains("def make_client(self) -> Client:"));
        assert!(source.contains("part._bind_demarreur(self.parts().simple().starter())"));
        assert!(source.contains("return self.parts().client().letsgo()"));
    }

    #[test]
    fn specialization_extends_parent_views_and_implements_delegated_override() {
        let source = GenPython::new(parse(
            r#"
            import demo.Traceur
            namespace demo {
                component Cypher specializes Traceur {
                    provides demarreur: Start = decodeur.crypt
                    part decodeur: Codec[Start] {
                        bind message to starter
                    }
                }
            }
            "#,
        ))
        .render()
        .expect("generation should succeed");

        assert!(source.contains("class Cypher(Traceur):"));
        assert!(source.contains("class _Requires(Traceur._Requires):"));
        assert!(source.contains("class _Parts(Traceur._Parts):"));
        assert!(source.contains("def make_demarreur(self) -> Start:"));
        assert!(!source.contains("@abstractmethod\n    def make_demarreur(self) -> Start:"));
        assert!(source.contains("part._bind_message(self.requires().starter())"));
    }

    #[test]
    fn generated_components_execute_factories_bindings_and_delegation() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("may-python-runtime-{unique}"));
        let contract_dir = root.join("ex1");
        fs::create_dir_all(&contract_dir).expect("contract directory should be created");
        fs::write(
            contract_dir.join("Start.py"),
            "class Start:\n    def __init__(self): self.calls = 0\n    def go(self): self.calls += 1\n",
        )
        .expect("service contract should be written");

        for source in [
            "import ex1.Start\nnamespace ex1.simple { component Simple { provides starter: Start } }",
            "import ex1.Start\nnamespace ex1.client { component Client { requires demarreur: Start provides letsgo: Runnable } }",
            r#"
                import ex1.simple.Simple
                import ex1.client.Client
                namespace ex1.composite {
                    component Composite {
                        provides service: Runnable = client.letsgo
                        part simple: Simple {}
                        part client: Client {
                            bind demarreur to simple.starter
                        }
                    }
                }
            "#,
        ] {
            GenPython::new(parse(source))
                .with_output(Some(root.clone()))
                .generate()
                .expect("component generation should succeed");
        }

        let script = r#"
from ex1.Start import Start
from ex1.simple.Simple import Simple
from ex1.client.Client import Client
from ex1.composite.Composite import Composite

class SimpleImpl(Simple):
    def __init__(self):
        super().__init__()
        self.created = 0

    def make_starter(self):
        self.created += 1
        return Start()

class ClientImpl(Client):
    def make_letsgo(self):
        return self.requires().demarreur().go

class CompositeImpl(Composite):
    def make_simple(self):
        return SimpleImpl()

    def make_client(self):
        return ClientImpl()

component = CompositeImpl()
action = component.service()
action()
action()
start = component.parts().simple().starter()
assert start.calls == 2
assert component.service() is action
assert component.parts().simple().created == 1
assert component.parts().simple() is component.parts().simple()
"#;
        let output = Command::new("python3")
            .arg("-c")
            .arg(script)
            .env("PYTHONPATH", &root)
            .output()
            .expect("Python runtime should start");

        if !output.status.success() {
            panic!(
                "generated component runtime failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }
}
