use crate::modules::speadl::ast::{Ast, ProvidedServiceImplementation, ServiceReference};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};

const GENERATED_JAVA_EXAMPLES_DIR: &str = "examples/java";

/// Generates the Java component API and runtime scaffolding inferred by MAY from a
/// SPEADL component.
///
/// Component dependencies are deliberately supplied as parsed SPEADL ASTs.  They
/// are used to recover the signatures of a part's `Requires` interface and to
/// substitute the (single) generic parameter supported by the Rust SPEADL AST.
pub struct GenJava {
    ast: Ast,
    dependencies: Vec<Ast>,
    options: GeneratorOptions,
}

#[derive(Debug, Clone, Default)]
struct GeneratorOptions {
    keep_intermediate: bool,
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JavaComponent {
    namespace: Vec<String>,
    imports: Vec<Vec<String>>,
    name: String,
    parent: Option<String>,
    parent_argument: Option<String>,
    generic: Option<String>,
    requires: Vec<Port>,
    provides: Vec<ProvidedPort>,
    parts: Vec<ComponentPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Port {
    name: String,
    type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProvidedPort {
    name: String,
    type_name: String,
    implementation: ProvidedServiceImplementation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentPart {
    name: String,
    type_name: String,
    generic: Option<String>,
    bindings: Vec<Binding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Binding {
    required_name: String,
    target: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolvedPort {
    name: String,
    type_name: String,
    import: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedProvided {
    name: String,
    type_name: String,
    implementation: ProvidedServiceImplementation,
}

#[derive(Debug, Clone)]
struct ResolvedPart {
    name: String,
    component: JavaComponent,
    argument: Option<String>,
}

#[derive(Debug, Clone)]
struct Instance {
    component: JavaComponent,
    argument: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ComponentCatalog {
    by_fqcn: BTreeMap<String, JavaComponent>,
}

#[derive(Debug, Clone)]
struct GenerationError {
    message: String,
}

impl GenerationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GenerationError {}

type GenResult<T> = Result<T, GenerationError>;

impl GenJava {
    pub fn new(ast: Ast) -> Self {
        Self {
            ast,
            dependencies: Vec::new(),
            options: GeneratorOptions::default(),
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<Ast>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_output(mut self, output: Option<PathBuf>) -> Self {
        self.options.output = output;
        self
    }

    pub fn with_keep_intermediate(mut self, keep_intermediate: bool) -> Self {
        self.options.keep_intermediate = keep_intermediate;
        self
    }

    pub fn render(&self) -> Result<String, Box<dyn Error>> {
        let (component, catalog) = self.prepare()?;
        Ok(JavaRenderer::new(&component, &catalog).render()?)
    }

    pub fn generate(&self) -> Result<(), Box<dyn Error>> {
        let (component, catalog) = self.prepare()?;
        let source = JavaRenderer::new(&component, &catalog).render()?;
        let output_path = component.output_path(self.options.output.as_deref());

        if let Some(parent) = output_path.parent() {
            create_dir_all(parent)?;
        }
        fs::write(&output_path, source)?;

        if self.options.keep_intermediate {
            fs::write(
                intermediate_output_path(&output_path),
                component.describe(&catalog)?,
            )?;
        }
        Ok(())
    }

    fn prepare(&self) -> GenResult<(JavaComponent, ComponentCatalog)> {
        let component = JavaComponent::from_ast(&self.ast)?;
        let mut catalog = ComponentCatalog::default();
        catalog.add_ast(&self.ast)?;
        for dependency in &self.dependencies {
            catalog.add_ast(dependency)?;
        }
        catalog.validate_component(&component)?;
        Ok((component, catalog))
    }
}

impl JavaComponent {
    fn from_ast(ast: &Ast) -> GenResult<Self> {
        let Ast::SEQ(nodes) = ast else {
            return Err(GenerationError::new(
                "Java generation expects a top-level SPEADL sequence",
            ));
        };

        let imports = nodes
            .iter()
            .filter_map(|node| match node {
                Ast::Import { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let namespaces = nodes
            .iter()
            .filter_map(|node| match node {
                Ast::Namespace { path, body } => Some((path, body.as_ref())),
                _ => None,
            })
            .collect::<Vec<_>>();

        if namespaces.len() != 1 {
            return Err(GenerationError::new(format!(
                "Java generation expects exactly one namespace, found {}",
                namespaces.len()
            )));
        }
        let (namespace, body) = namespaces[0];
        let Ast::Component {
            name,
            specializes,
            generic,
            body,
        } = body
        else {
            return Err(GenerationError::new(
                "Java generation expects one component directly inside the namespace",
            ));
        };
        let Ast::SEQ(component_nodes) = body.as_ref() else {
            return Err(GenerationError::new(
                "Java generation expects the component body to be a sequence",
            ));
        };

        let mut requires = Vec::new();
        let mut provides = Vec::new();
        let mut parts = Vec::new();
        for node in component_nodes {
            match node {
                Ast::Requires { name, type_name } => requires.push(Port {
                    name: name.clone(),
                    type_name: type_name.clone(),
                }),
                Ast::Provides {
                    name,
                    type_name,
                    implementation,
                } => provides.push(ProvidedPort {
                    name: name.clone(),
                    type_name: type_name.clone(),
                    implementation: implementation.clone(),
                }),
                Ast::Part {
                    name,
                    type_name,
                    generic,
                    body,
                } => {
                    let Ast::SEQ(binding_nodes) = body.as_ref() else {
                        return Err(GenerationError::new(format!(
                            "part `{name}` must contain a sequence of bindings"
                        )));
                    };
                    let mut bindings = Vec::new();
                    for binding in binding_nodes {
                        let Ast::Bind { name, target } = binding else {
                            return Err(GenerationError::new(format!(
                                "part `{name}` contains an element other than a binding"
                            )));
                        };
                        bindings.push(Binding {
                            required_name: name.clone(),
                            target: target.clone(),
                        });
                    }
                    parts.push(ComponentPart {
                        name: name.clone(),
                        type_name: type_name.clone(),
                        generic: generic.clone(),
                        bindings,
                    });
                }
                Ast::Bind { .. }
                | Ast::Import { .. }
                | Ast::Namespace { .. }
                | Ast::Component { .. }
                | Ast::SEQ(_) => {
                    return Err(GenerationError::new(format!(
                        "component `{name}` contains an unsupported AST node"
                    )));
                }
            }
        }

        Ok(Self {
            namespace: namespace.clone(),
            imports,
            name: name.clone(),
            parent: specializes.as_ref().map(|value| value.parent.clone()),
            parent_argument: specializes
                .as_ref()
                .and_then(|value| value.argument.clone()),
            generic: generic.clone(),
            requires,
            provides,
            parts,
        })
    }

    fn fqcn(&self) -> String {
        let mut pieces = self.namespace.clone();
        pieces.push(self.name.clone());
        pieces.join(".")
    }

    fn own_instance(&self) -> Instance {
        Instance {
            component: self.clone(),
            argument: self.generic.clone(),
        }
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
        for package_part in &self.namespace {
            path.push(package_part);
        }
        path.push(format!("{}.java", self.name));
        path
    }

    fn describe(&self, catalog: &ComponentCatalog) -> GenResult<String> {
        let instance = self.own_instance();
        let parent = catalog
            .parent_instance(&instance)?
            .map(|value| {
                format!(
                    "{}{}",
                    value.component.fqcn(),
                    generic_use(value.argument.as_deref())
                )
            })
            .unwrap_or_else(|| "<none>".to_string());
        let all_requires = catalog
            .all_requires(&instance)?
            .into_iter()
            .map(|port| format!("{}: {}", port.name, port.type_name))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "component: {}\nparent: {}\ngeneric: {}\nrequires: [{}]\nprovides: {}\nparts: {}\n",
            self.fqcn(),
            parent,
            self.generic.as_deref().unwrap_or("<none>"),
            all_requires,
            self.provides.len(),
            self.parts.len()
        ))
    }
}

impl ComponentCatalog {
    fn add_ast(&mut self, ast: &Ast) -> GenResult<()> {
        let component = JavaComponent::from_ast(ast)?;
        self.insert(component)?;
        self.add_embedded_asts(ast)
    }

    fn add_embedded_asts(&mut self, ast: &Ast) -> GenResult<()> {
        match ast {
            Ast::SEQ(nodes) => {
                for node in nodes {
                    self.add_embedded_asts(node)?;
                }
            }
            Ast::Import {
                ast: Some(imported),
                ..
            } => {
                let component = JavaComponent::from_ast(imported)?;
                self.insert(component)?;
                self.add_embedded_asts(imported)?;
            }
            Ast::Namespace { body, .. } => self.add_embedded_asts(body)?,
            Ast::Component {
                specializes, body, ..
            } => {
                if let Some(parent) = specializes
                    && let Some(parent_file) = &parent.parent_file
                {
                    let component = JavaComponent::from_ast(parent_file)?;
                    self.insert(component)?;
                    self.add_embedded_asts(parent_file)?;
                }
                self.add_embedded_asts(body)?;
            }
            Ast::Part { body, .. } => self.add_embedded_asts(body)?,
            Ast::Import { ast: None, .. }
            | Ast::Requires { .. }
            | Ast::Provides { .. }
            | Ast::Bind { .. } => {}
        }
        Ok(())
    }

    fn insert(&mut self, component: JavaComponent) -> GenResult<()> {
        let fqcn = component.fqcn();
        if let Some(previous) = self.by_fqcn.get(&fqcn) {
            if previous != &component {
                return Err(GenerationError::new(format!(
                    "dependency catalog contains conflicting definitions for `{fqcn}`"
                )));
            }
            return Ok(());
        }
        self.by_fqcn.insert(fqcn, component);
        Ok(())
    }

    fn resolve_component(
        &self,
        owner: &JavaComponent,
        name: &str,
        context: &str,
    ) -> GenResult<JavaComponent> {
        if name.contains('.') {
            return self.by_fqcn.get(name).cloned().ok_or_else(|| {
                GenerationError::new(format!(
                    "component `{name}` used by {context} is missing from the dependency catalog"
                ))
            });
        }

        let imported = owner
            .imports
            .iter()
            .filter(|path| path.last().is_some_and(|last| last == name))
            .map(|path| path.join("."))
            .filter_map(|fqcn| self.by_fqcn.get(&fqcn).cloned())
            .collect::<Vec<_>>();
        if imported.len() == 1 {
            return Ok(imported[0].clone());
        }
        if imported.len() > 1 {
            return Err(GenerationError::new(format!(
                "component name `{name}` used by {context} is ambiguous in imports"
            )));
        }

        let mut same_package = owner.namespace.clone();
        same_package.push(name.to_string());
        if let Some(component) = self.by_fqcn.get(&same_package.join(".")) {
            return Ok(component.clone());
        }

        Err(GenerationError::new(format!(
            "component `{name}` used by {context} is not in the same package and has no explicit import"
        )))
    }

    fn parent_instance(&self, instance: &Instance) -> GenResult<Option<Instance>> {
        let Some(parent_name) = &instance.component.parent else {
            return Ok(None);
        };
        let parent = self.resolve_component(
            &instance.component,
            parent_name,
            &format!("specialization of `{}`", instance.component.fqcn()),
        )?;
        let explicit_argument = instance
            .component
            .parent_argument
            .as_deref()
            .map(|argument| {
                if instance.component.generic.as_deref() == Some(argument) {
                    instance.argument.clone()
                } else {
                    Some(argument.to_string())
                }
            });
        let argument = match (
            &parent.generic,
            explicit_argument,
            &instance.component.generic,
        ) {
            (Some(_), Some(argument), _) => argument,
            (Some(_), None, Some(_)) => instance.argument.clone(),
            (Some(_), None, None) => {
                return Err(GenerationError::new(format!(
                    "non-generic component `{}` cannot specialize generic component `{}`",
                    instance.component.fqcn(),
                    parent.fqcn()
                )));
            }
            (None, Some(_), _) => {
                return Err(GenerationError::new(format!(
                    "component `{}` supplies a type argument to non-generic parent `{}`",
                    instance.component.fqcn(),
                    parent.fqcn()
                )));
            }
            (None, None, _) => None,
        };
        Ok(Some(Instance {
            component: parent,
            argument,
        }))
    }

    fn part_instance(&self, owner: &JavaComponent, part: &ComponentPart) -> GenResult<Instance> {
        let component = self.resolve_component(
            owner,
            &part.type_name,
            &format!("part `{}` of `{}`", part.name, owner.fqcn()),
        )?;
        match (&component.generic, &part.generic) {
            (None, Some(argument)) => Err(GenerationError::new(format!(
                "part `{}` supplies type argument `{argument}` to non-generic component `{}`",
                part.name,
                component.fqcn()
            ))),
            _ => Ok(Instance {
                component,
                argument: part.generic.clone(),
            }),
        }
    }

    fn all_requires(&self, instance: &Instance) -> GenResult<Vec<ResolvedPort>> {
        let mut ports = if let Some(parent) = self.parent_instance(instance)? {
            self.all_requires(&parent)?
        } else {
            Vec::new()
        };
        ports.extend(instance.component.requires.iter().map(|port| ResolvedPort {
            name: port.name.clone(),
            type_name: substitute_type(instance, &port.type_name),
            import: resolved_type_import(instance, &port.type_name),
        }));
        Ok(ports)
    }

    fn all_provides(&self, instance: &Instance) -> GenResult<Vec<ResolvedProvided>> {
        let mut ports = if let Some(parent) = self.parent_instance(instance)? {
            self.all_provides(&parent)?
        } else {
            Vec::new()
        };
        for port in &instance.component.provides {
            let resolved = ResolvedProvided {
                name: port.name.clone(),
                type_name: substitute_type(instance, &port.type_name),
                implementation: port.implementation.clone(),
            };
            if let Some(index) = ports.iter().position(|value| value.name == port.name) {
                ports[index] = resolved;
            } else {
                ports.push(resolved);
            }
        }
        Ok(ports)
    }

    fn all_parts(&self, instance: &Instance) -> GenResult<Vec<ResolvedPart>> {
        let mut parts = if let Some(parent) = self.parent_instance(instance)? {
            self.all_parts(&parent)?
        } else {
            Vec::new()
        };
        for part in &instance.component.parts {
            let resolved = self.part_instance(&instance.component, part)?;
            if parts.iter().any(|value| value.name == part.name) {
                return Err(GenerationError::new(format!(
                    "part `{}` in `{}` conflicts with an inherited part",
                    part.name,
                    instance.component.fqcn()
                )));
            }
            parts.push(ResolvedPart {
                name: part.name.clone(),
                component: resolved.component,
                argument: resolved.argument,
            });
        }
        Ok(parts)
    }

    fn parent_provides(&self, component: &JavaComponent, name: &str) -> GenResult<bool> {
        let instance = component.own_instance();
        let Some(parent) = self.parent_instance(&instance)? else {
            return Ok(false);
        };
        Ok(self
            .all_provides(&parent)?
            .iter()
            .any(|port| port.name == name))
    }

    fn root_instance(&self, instance: &Instance) -> GenResult<Instance> {
        let mut current = instance.clone();
        let mut seen = HashSet::new();
        loop {
            if !seen.insert(current.component.fqcn()) {
                return Err(GenerationError::new(format!(
                    "specialization cycle detected at `{}`",
                    current.component.fqcn()
                )));
            }
            match self.parent_instance(&current)? {
                Some(parent) => current = parent,
                None => return Ok(current),
            }
        }
    }

    fn validate_component(&self, component: &JavaComponent) -> GenResult<()> {
        let mut visiting = HashSet::new();
        let mut validated = HashSet::new();
        self.validate_component_recursive(component, &mut visiting, &mut validated)
    }

    fn validate_component_recursive(
        &self,
        component: &JavaComponent,
        visiting: &mut HashSet<String>,
        validated: &mut HashSet<String>,
    ) -> GenResult<()> {
        let fqcn = component.fqcn();
        if validated.contains(&fqcn) {
            return Ok(());
        }
        if !visiting.insert(fqcn.clone()) {
            return Err(GenerationError::new(format!(
                "specialization cycle detected at `{fqcn}`"
            )));
        }

        validate_java_component_identifiers(component)?;
        validate_unique_names(component)?;

        let instance = component.own_instance();
        if let Some(parent) = self.parent_instance(&instance)? {
            self.validate_component_recursive(&parent.component, visiting, validated)?;
            if !self.all_parts(&parent)?.is_empty() {
                return Err(GenerationError::new(format!(
                    "component `{fqcn}` cannot specialize `{}` because the parent declares parts",
                    parent.component.fqcn()
                )));
            }
            let inherited_requires = self.all_requires(&parent)?;
            let inherited_provides = self.all_provides(&parent)?;
            for provided in &component.provides {
                if inherited_requires
                    .iter()
                    .any(|port| port.name == provided.name)
                {
                    return Err(GenerationError::new(format!(
                        "provided port `{}` in `{fqcn}` conflicts with an inherited required port",
                        provided.name
                    )));
                }
            }
            for part in &component.parts {
                if inherited_requires.iter().any(|port| port.name == part.name) {
                    return Err(GenerationError::new(format!(
                        "part `{}` in `{fqcn}` conflicts with an inherited required port",
                        part.name
                    )));
                }
                if inherited_provides.iter().any(|port| port.name == part.name) {
                    return Err(GenerationError::new(format!(
                        "part `{}` in `{fqcn}` conflicts with an inherited provided port",
                        part.name
                    )));
                }
            }
            if !component.requires.is_empty() {
                return Err(GenerationError::new(format!(
                    "specialized component `{fqcn}` cannot declare new required ports: \
                     the official Java model exposes `Requires` only on the root component"
                )));
            }
        }

        let effective_requires = self.all_requires(&instance)?;
        let effective_provides = self.all_provides(&instance)?;
        let effective_parts = self.all_parts(&instance)?;

        for part in &component.parts {
            let part_instance = self.part_instance(component, part)?;
            self.validate_component_recursive(&part_instance.component, visiting, validated)?;
            let required = self.all_requires(&part_instance)?;
            let mut bound = BTreeSet::new();
            for binding in &part.bindings {
                if !bound.insert(binding.required_name.clone()) {
                    return Err(GenerationError::new(format!(
                        "part `{}` binds required port `{}` more than once",
                        part.name, binding.required_name
                    )));
                }
                if !required
                    .iter()
                    .any(|port| port.name == binding.required_name)
                {
                    return Err(GenerationError::new(format!(
                        "part `{}` binds unknown required port `{}` of component `{}`",
                        part.name,
                        binding.required_name,
                        part_instance.component.fqcn()
                    )));
                }
                let required_port = required
                    .iter()
                    .find(|port| port.name == binding.required_name)
                    .expect("the required port was checked above");
                let (source_type, _) = resolve_binding_target(
                    binding,
                    &effective_requires,
                    &effective_provides,
                    &effective_parts,
                    self,
                )?;
                if source_type != required_port.type_name {
                    return Err(GenerationError::new(format!(
                        "type mismatch in part `{}` binding `{}`: required port expects `{}`, but target `{}` provides `{}`",
                        part.name,
                        binding.required_name,
                        required_port.type_name,
                        binding.target.join("."),
                        source_type
                    )));
                }
            }
            for required_port in required {
                if !bound.contains(&required_port.name) {
                    return Err(GenerationError::new(format!(
                        "part `{}` is missing a binding for required port `{}`",
                        part.name, required_port.name
                    )));
                }
            }
        }

        for provided in &component.provides {
            if let ProvidedServiceImplementation::Delegated(reference) = &provided.implementation {
                let delegated = resolve_delegation(reference, &effective_parts, self)?;
                if delegated.type_name != provided.type_name {
                    return Err(GenerationError::new(format!(
                        "type mismatch in delegated provided port `{}`: declared as `{}`, but `{}.{}` provides `{}`",
                        provided.name,
                        provided.type_name,
                        reference.part_name,
                        reference.service_name,
                        delegated.type_name
                    )));
                }
            }
        }

        visiting.remove(&fqcn);
        validated.insert(fqcn);
        Ok(())
    }
}

fn substitute_type(instance: &Instance, raw: &str) -> String {
    match (&instance.component.generic, &instance.argument) {
        (Some(parameter), Some(argument)) if raw == parameter => argument.clone(),
        (Some(parameter), None) if raw == parameter => "Object".to_string(),
        _ => raw.to_string(),
    }
}

fn resolved_type_import(instance: &Instance, raw: &str) -> Option<String> {
    if instance.component.generic.as_deref() == Some(raw) {
        return None;
    }
    let matches = instance
        .component
        .imports
        .iter()
        .filter(|path| path.last().is_some_and(|segment| segment == raw))
        .map(|path| path.join("."))
        .collect::<BTreeSet<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn validate_java_component_identifiers(component: &JavaComponent) -> GenResult<()> {
    for package_part in &component.namespace {
        validate_java_identifier(package_part, "package segment")?;
    }
    if component.namespace.is_empty() {
        return Err(GenerationError::new(
            "Java generation requires a non-empty namespace/package",
        ));
    }
    validate_java_identifier(&component.name, "component name")?;
    require_initial_case(&component.name, true, "component name")?;
    if let Some(generic) = &component.generic {
        validate_java_identifier(generic, "generic parameter")?;
        require_initial_case(generic, true, "generic parameter")?;
    }
    if let Some(parent) = &component.parent {
        validate_java_type_name(parent, "specialized component name")?;
    }
    if let Some(argument) = &component.parent_argument {
        validate_java_type_name(argument, "specialized component type argument")?;
    }
    for import in &component.imports {
        if import.is_empty() {
            return Err(GenerationError::new("an import path cannot be empty"));
        }
        for segment in import {
            validate_java_identifier(segment, "import segment")?;
        }
    }
    for port in &component.requires {
        validate_java_identifier(&port.name, "required port name")?;
        require_initial_case(&port.name, false, "required port name")?;
        validate_java_type_name(&port.type_name, "required port type")?;
    }
    for port in &component.provides {
        validate_java_identifier(&port.name, "provided port name")?;
        require_initial_case(&port.name, false, "provided port name")?;
        validate_java_type_name(&port.type_name, "provided port type")?;
        if let ProvidedServiceImplementation::Delegated(reference) = &port.implementation {
            validate_java_identifier(&reference.part_name, "delegated part name")?;
            validate_java_identifier(&reference.service_name, "delegated port name")?;
        }
    }
    for part in &component.parts {
        validate_java_identifier(&part.name, "part name")?;
        require_initial_case(&part.name, false, "part name")?;
        validate_java_type_name(&part.type_name, "part component type")?;
        if let Some(argument) = &part.generic {
            validate_java_type_name(argument, "part type argument")?;
        }
        for binding in &part.bindings {
            validate_java_identifier(&binding.required_name, "bound required port name")?;
            if !(1..=2).contains(&binding.target.len()) {
                return Err(GenerationError::new(format!(
                    "binding `{}` in part `{}` must target one port or `part.port`, found {} segments",
                    binding.required_name,
                    part.name,
                    binding.target.len()
                )));
            }
            for segment in &binding.target {
                validate_java_identifier(segment, "binding target segment")?;
            }
        }
    }
    Ok(())
}

fn validate_unique_names(component: &JavaComponent) -> GenResult<()> {
    ensure_unique(
        component.requires.iter().map(|port| port.name.as_str()),
        "required port",
        component,
    )?;
    ensure_unique(
        component.provides.iter().map(|port| port.name.as_str()),
        "provided port",
        component,
    )?;
    ensure_unique(
        component.parts.iter().map(|part| part.name.as_str()),
        "part",
        component,
    )?;

    let mut feature_names = BTreeMap::new();
    for (kind, name) in component
        .requires
        .iter()
        .map(|port| ("required port", port.name.as_str()))
        .chain(
            component
                .provides
                .iter()
                .map(|port| ("provided port", port.name.as_str())),
        )
        .chain(
            component
                .parts
                .iter()
                .map(|part| ("part", part.name.as_str())),
        )
    {
        if let Some(previous_kind) = feature_names.insert(name, kind) {
            return Err(GenerationError::new(format!(
                "component `{}` reuses feature name `{name}` for a {previous_kind} and a {kind}",
                component.fqcn(),
            )));
        }
    }
    let mut generated_methods = BTreeMap::new();
    for (name, origin) in [
        ("start".to_string(), "component lifecycle".to_string()),
        ("initParts".to_string(), "parts initializer".to_string()),
        (
            "initProvidedPorts".to_string(),
            "provided-ports initializer".to_string(),
        ),
    ] {
        generated_methods.insert(name, origin);
    }
    for port in &component.provides {
        register_generated_method(
            &mut generated_methods,
            &port.name,
            &format!("accessor for provided port `{}`", port.name),
            component,
        )?;
        register_generated_method(
            &mut generated_methods,
            &format!("init_{}", port.name),
            &format!("initializer for provided port `{}`", port.name),
            component,
        )?;
    }
    for part in &component.parts {
        register_generated_method(
            &mut generated_methods,
            &part.name,
            &format!("accessor for part `{}`", part.name),
            component,
        )?;
        register_generated_method(
            &mut generated_methods,
            &format!("init_{}", part.name),
            &format!("initializer for part `{}`", part.name),
            component,
        )?;
    }

    let mut generated_fields = BTreeMap::from([
        ("bridge".to_string(), "required-port bridge".to_string()),
        (
            "implementation".to_string(),
            "component implementation".to_string(),
        ),
    ]);
    for port in &component.provides {
        if matches!(port.implementation, ProvidedServiceImplementation::Local) {
            register_generated_field(
                &mut generated_fields,
                &port.name,
                &format!("storage for local provided port `{}`", port.name),
                component,
            )?;
        }
    }
    for part in &component.parts {
        register_generated_field(
            &mut generated_fields,
            &part.name,
            &format!("component storage for part `{}`", part.name),
            component,
        )?;
        register_generated_field(
            &mut generated_fields,
            &format!("implem_{}", part.name),
            &format!("implementation storage for part `{}`", part.name),
            component,
        )?;
    }
    Ok(())
}

fn register_generated_method(
    methods: &mut BTreeMap<String, String>,
    name: &str,
    origin: &str,
    component: &JavaComponent,
) -> GenResult<()> {
    if let Some(previous_origin) = methods.insert(name.to_string(), origin.to_string()) {
        return Err(GenerationError::new(format!(
            "generated Java method collision in `{}.ComponentImpl`: \
             `{name}()` is both {previous_origin} and {origin}",
            component.fqcn()
        )));
    }
    Ok(())
}

fn register_generated_field(
    fields: &mut BTreeMap<String, String>,
    name: &str,
    origin: &str,
    component: &JavaComponent,
) -> GenResult<()> {
    if let Some(previous_origin) = fields.insert(name.to_string(), origin.to_string()) {
        return Err(GenerationError::new(format!(
            "generated Java field collision in `{}.ComponentImpl`: \
             `{name}` is both {previous_origin} and {origin}",
            component.fqcn()
        )));
    }
    Ok(())
}

fn require_initial_case(name: &str, uppercase: bool, context: &str) -> GenResult<()> {
    let first = name
        .chars()
        .next()
        .expect("validated Java identifiers are never empty");
    let valid = if uppercase {
        first.is_uppercase()
    } else {
        first.is_lowercase()
    };
    if !valid {
        let expected = if uppercase {
            "an uppercase"
        } else {
            "a lowercase"
        };
        return Err(GenerationError::new(format!(
            "{context} `{name}` must start with {expected} letter"
        )));
    }
    Ok(())
}

fn ensure_unique<'a>(
    names: impl Iterator<Item = &'a str>,
    kind: &str,
    component: &JavaComponent,
) -> GenResult<()> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(GenerationError::new(format!(
                "duplicate {kind} `{name}` in component `{}`",
                component.fqcn()
            )));
        }
    }
    Ok(())
}

fn validate_java_type_name(name: &str, context: &str) -> GenResult<()> {
    if name.is_empty() {
        return Err(GenerationError::new(format!(
            "empty Java type name in {context}"
        )));
    }
    for segment in name.split('.') {
        validate_java_identifier(segment, context)?;
    }
    Ok(())
}

fn validate_java_identifier(identifier: &str, context: &str) -> GenResult<()> {
    let mut characters = identifier.chars();
    let valid_start = characters
        .next()
        .is_some_and(|character| character == '_' || character == '$' || character.is_alphabetic());
    let valid_tail = characters
        .all(|character| character == '_' || character == '$' || character.is_alphanumeric());
    if !valid_start || !valid_tail || JAVA_KEYWORDS.contains(&identifier) {
        return Err(GenerationError::new(format!(
            "invalid Java identifier `{identifier}` used as {context}"
        )));
    }
    Ok(())
}

const JAVA_KEYWORDS: &[&str] = &[
    "_",
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "exports",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "module",
    "native",
    "new",
    "non-sealed",
    "null",
    "open",
    "opens",
    "package",
    "permits",
    "private",
    "protected",
    "provides",
    "public",
    "record",
    "requires",
    "return",
    "sealed",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "to",
    "transient",
    "transitive",
    "true",
    "try",
    "uses",
    "var",
    "void",
    "volatile",
    "when",
    "while",
    "with",
    "yield",
];

fn resolve_binding_target(
    binding: &Binding,
    owner_requires: &[ResolvedPort],
    owner_provides: &[ResolvedProvided],
    owner_parts: &[ResolvedPart],
    catalog: &ComponentCatalog,
) -> GenResult<(String, String)> {
    match binding.target.as_slice() {
        [port_name] => {
            if let Some(port) = owner_requires.iter().find(|port| &port.name == port_name) {
                return Ok((
                    port.type_name.clone(),
                    format!("ComponentImpl.this.bridge.{port_name}()"),
                ));
            }
            if let Some(port) = owner_provides.iter().find(|port| &port.name == port_name) {
                return Ok((
                    port.type_name.clone(),
                    format!("ComponentImpl.this.{port_name}()"),
                ));
            }
            Err(GenerationError::new(format!(
                "binding `{}` targets unknown owner port `{port_name}`",
                binding.required_name
            )))
        }
        [part_name, port_name] => {
            let part = owner_parts
                .iter()
                .find(|part| &part.name == part_name)
                .ok_or_else(|| {
                    GenerationError::new(format!(
                        "binding `{}` targets unknown part `{part_name}`",
                        binding.required_name
                    ))
                })?;
            let instance = Instance {
                component: part.component.clone(),
                argument: part.argument.clone(),
            };
            let provided = catalog
                .all_provides(&instance)?
                .into_iter()
                .find(|port| &port.name == port_name)
                .ok_or_else(|| {
                    GenerationError::new(format!(
                        "binding `{}` targets unknown provided port `{part_name}.{port_name}`",
                        binding.required_name
                    ))
                })?;
            Ok((
                provided.type_name,
                format!("ComponentImpl.this.{part_name}().{port_name}()"),
            ))
        }
        _ => Err(GenerationError::new(format!(
            "binding `{}` has an invalid target path",
            binding.required_name
        ))),
    }
}

fn resolve_delegation(
    reference: &ServiceReference,
    owner_parts: &[ResolvedPart],
    catalog: &ComponentCatalog,
) -> GenResult<ResolvedProvided> {
    let part = owner_parts
        .iter()
        .find(|part| part.name == reference.part_name)
        .ok_or_else(|| {
            GenerationError::new(format!(
                "delegated service targets unknown part `{}`",
                reference.part_name
            ))
        })?;
    let instance = Instance {
        component: part.component.clone(),
        argument: part.argument.clone(),
    };
    catalog
        .all_provides(&instance)?
        .into_iter()
        .find(|port| port.name == reference.service_name)
        .ok_or_else(|| {
            GenerationError::new(format!(
                "delegated service targets unknown provided port `{}.{}`",
                reference.part_name, reference.service_name
            ))
        })
}

fn default_output_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(GENERATED_JAVA_EXAMPLES_DIR)
}

fn output_target_is_file(path: &Path) -> bool {
    !path.is_dir() && path.extension().is_some()
}

fn intermediate_output_path(output_path: &Path) -> PathBuf {
    let file_stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("java_model");
    let mut path = output_path.to_path_buf();
    path.set_file_name(format!("{file_stem}.java_model.txt"));
    path
}

struct JavaRenderer<'a> {
    component: &'a JavaComponent,
    catalog: &'a ComponentCatalog,
}

impl<'a> JavaRenderer<'a> {
    fn new(component: &'a JavaComponent, catalog: &'a ComponentCatalog) -> Self {
        Self { component, catalog }
    }

    fn render(&self) -> GenResult<String> {
        let mut java = JavaSource::default();
        java.line(format!("package {};", self.component.namespace.join(".")));
        java.blank();

        let own_fqcn = self.component.fqcn();
        let mut imports = self
            .component
            .imports
            .iter()
            .map(|path| path.join("."))
            .filter(|import| import != &own_fqcn)
            .collect::<BTreeSet<_>>();
        for part in &self.component.parts {
            let part_instance = self.catalog.part_instance(self.component, part)?;
            for required in self.catalog.all_requires(&part_instance)? {
                if let Some(import) = required.import
                    && import != own_fqcn
                {
                    imports.insert(import);
                }
            }
        }
        for import in imports {
            java.line(format!("import {import};"));
        }
        if !self.component.imports.is_empty() || !self.component.parts.is_empty() {
            java.blank();
        }

        let instance = self.component.own_instance();
        let parent = self.catalog.parent_instance(&instance)?;
        let abstract_component = self.is_abstract(&instance)?;
        let mut declaration = String::from("public ");
        if abstract_component {
            declaration.push_str("abstract ");
        }
        declaration.push_str("class ");
        declaration.push_str(&self.component.name);
        declaration.push_str(&generic_declaration(self.component.generic.as_deref()));
        if let Some(parent) = &parent {
            declaration.push_str(" extends ");
            declaration.push_str(&self.outer_ref(parent));
        }
        java.open(declaration);

        if parent.is_none() {
            self.render_requires_interface(&mut java, &instance)?;
            java.blank();
        }
        self.render_provides_interface(&mut java, parent.as_ref())?;
        java.blank();
        self.render_component_interface(&mut java, parent.as_ref())?;
        java.blank();
        self.render_parts_interface(&mut java, parent.as_ref())?;
        java.blank();
        self.render_outer_runtime(&mut java, &instance, parent.as_ref())?;
        java.blank();
        self.render_component_impl(&mut java, &instance, parent.as_ref())?;

        java.close();
        Ok(java.finish())
    }

    fn render_requires_interface(
        &self,
        java: &mut JavaSource,
        instance: &Instance,
    ) -> GenResult<()> {
        java.open(format!(
            "public static interface Requires{}",
            generic_declaration(self.component.generic.as_deref())
        ));
        for port in &instance.component.requires {
            java.line(format!(
                "{} {}();",
                substitute_type(instance, &port.type_name),
                port.name
            ));
        }
        java.close();
        Ok(())
    }

    fn render_provides_interface(
        &self,
        java: &mut JavaSource,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        let mut declaration = format!(
            "public static interface Provides{}",
            generic_declaration(self.component.generic.as_deref())
        );
        if let Some(parent) = parent {
            declaration.push_str(" extends ");
            declaration.push_str(&self.nested_ref(parent, "Provides"));
        }
        java.open(declaration);
        let own = self.component.own_instance();
        for port in &self.component.provides {
            java.line(format!(
                "{} {}();",
                substitute_type(&own, &port.type_name),
                port.name
            ));
        }
        java.close();
        Ok(())
    }

    fn render_component_interface(
        &self,
        java: &mut JavaSource,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        let mut declaration = format!(
            "public static interface Component{} extends ",
            generic_declaration(self.component.generic.as_deref())
        );
        if let Some(parent) = parent {
            declaration.push_str(&self.nested_ref(parent, "Component"));
            declaration.push_str(", ");
        }
        declaration.push_str(&local_nested_ref(
            "Provides",
            self.component.generic.as_deref(),
        ));
        java.open(declaration);
        java.close();
        Ok(())
    }

    fn render_parts_interface(
        &self,
        java: &mut JavaSource,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        let mut declaration = format!(
            "public static interface Parts{}",
            generic_declaration(self.component.generic.as_deref())
        );
        if let Some(parent) = parent {
            declaration.push_str(" extends ");
            declaration.push_str(&self.nested_ref(parent, "Parts"));
        }
        java.open(declaration);
        for part in &self.component.parts {
            let instance = self.catalog.part_instance(self.component, part)?;
            java.line(format!(
                "{} {}();",
                self.nested_ref(&instance, "Component"),
                part.name
            ));
        }
        java.close();
        Ok(())
    }

    fn render_outer_runtime(
        &self,
        java: &mut JavaSource,
        instance: &Instance,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        let component_impl = local_nested_ref("ComponentImpl", self.component.generic.as_deref());
        java.line("private boolean init = false;");
        java.line("private boolean started = false;");
        java.line(format!("private {component_impl} selfComponent;"));
        java.blank();

        if parent.is_some() {
            java.line("@Override");
        }
        java.open("protected void start()");
        java.open("if (!this.init || this.started)");
        java.line(
            "throw new RuntimeException(\"start() should not be called by hand: to create a new component, use newComponent().\");",
        );
        java.close();
        java.close();
        java.blank();

        if parent.is_some() {
            java.line("@Override");
        }
        java.open(format!(
            "protected {} provides()",
            local_nested_ref("Provides", self.component.generic.as_deref())
        ));
        render_component_access_checks(java, "provides");
        java.line("return this.selfComponent;");
        java.close();
        java.blank();

        if parent.is_some() {
            java.line("@Override");
        }
        java.open(format!(
            "protected {} requires()",
            self.root_requires_ref(instance)?
        ));
        render_component_access_checks(java, "requires");
        java.line("return this.selfComponent.bridge;");
        java.close();
        java.blank();

        if parent.is_some() {
            java.line("@Override");
        }
        java.open(format!(
            "protected {} parts()",
            local_nested_ref("Parts", self.component.generic.as_deref())
        ));
        render_component_access_checks(java, "parts");
        java.line("return this.selfComponent;");
        java.close();

        for port in &self.component.provides {
            let is_override = self.catalog.parent_provides(self.component, &port.name)?;
            match (&port.implementation, is_override) {
                (ProvidedServiceImplementation::Local, _) => {
                    java.blank();
                    if is_override {
                        java.line("@Override");
                    }
                    java.line(format!(
                        "protected abstract {} make_{}();",
                        substitute_type(instance, &port.type_name),
                        port.name
                    ));
                }
                (ProvidedServiceImplementation::Delegated(_), true) => {
                    java.blank();
                    java.line("@Override");
                    java.open(format!(
                        "protected final {} make_{}()",
                        substitute_type(instance, &port.type_name),
                        port.name
                    ));
                    java.line("throw new AssertionError(\"This is a bug\");");
                    java.close();
                }
                (ProvidedServiceImplementation::Delegated(_), false) => {}
            }
        }

        for part in &self.component.parts {
            let part_instance = self.catalog.part_instance(self.component, part)?;
            java.blank();
            java.line(format!(
                "protected abstract {} make_{}();",
                self.outer_ref(&part_instance),
                part.name
            ));
        }

        java.blank();
        if parent.is_some() {
            java.line("@Override");
        }
        java.open(format!(
            "public synchronized {} _newComponent({} b, boolean start)",
            local_nested_ref("Component", self.component.generic.as_deref()),
            self.root_requires_ref(instance)?
        ));
        java.open("if (b == null)");
        java.line("throw new NullPointerException(\"required-port bridge must not be null\");");
        java.close();
        java.open("if (this.init)");
        java.line(format!(
            "throw new RuntimeException(\"This instance of {} has already been used to create a component, use another one.\");",
            self.component.name
        ));
        java.close();
        java.line("this.init = true;");
        let constructor = if self.component.generic.is_some() {
            "new ComponentImpl<>(this, b, true)"
        } else {
            "new ComponentImpl(this, b, true)"
        };
        java.line(format!("{component_impl} component = {constructor};"));
        java.open("if (start)");
        java.line("component.start();");
        java.close();
        java.line("return component;");
        java.close();

        if self.catalog.all_requires(instance)?.is_empty() {
            java.blank();
            if parent.is_some() {
                java.line("@Override");
            }
            java.open(format!(
                "public {} newComponent()",
                local_nested_ref("Component", self.component.generic.as_deref())
            ));
            java.line(format!(
                "return this._newComponent(new {}() {{}}, true);",
                self.root_requires_ref(instance)?
            ));
            java.close();
        }
        Ok(())
    }

    fn render_component_impl(
        &self,
        java: &mut JavaSource,
        instance: &Instance,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        let mut declaration = format!(
            "public static class ComponentImpl{}",
            generic_declaration(self.component.generic.as_deref())
        );
        if let Some(parent) = parent {
            declaration.push_str(" extends ");
            declaration.push_str(&self.nested_ref(parent, "ComponentImpl"));
        }
        declaration.push_str(" implements ");
        declaration.push_str(&local_nested_ref(
            "Component",
            self.component.generic.as_deref(),
        ));
        declaration.push_str(", ");
        declaration.push_str(&local_nested_ref(
            "Parts",
            self.component.generic.as_deref(),
        ));
        java.open(declaration);

        java.line(format!(
            "private final {} bridge;",
            self.root_requires_ref(instance)?
        ));
        java.line(format!(
            "private final {} implementation;",
            self.outer_ref(instance)
        ));

        self.render_component_impl_start(java, parent)?;
        self.render_part_initializers(java)?;
        self.render_provided_initializers(java)?;
        self.render_component_impl_constructor(java, instance, parent)?;
        self.render_provided_fields_and_accessors(java, instance);
        self.render_part_fields_bridges_and_accessors(java, instance)?;

        java.close();
        Ok(())
    }

    fn render_component_impl_start(
        &self,
        java: &mut JavaSource,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        java.blank();
        if parent.is_some() {
            java.line("@Override");
        }
        java.open("public void start()");
        if parent.is_some() {
            java.line("super.start();");
        }
        for part in &self.component.parts {
            let part_instance = self.catalog.part_instance(self.component, part)?;
            java.open(format!("if (this.{} == null)", part.name));
            java.line(format!(
                "throw new IllegalStateException(\"part `{}` was not initialized\");",
                part.name
            ));
            java.close();
            java.line(format!(
                "(({}) this.{}).start();",
                self.nested_ref(&part_instance, "ComponentImpl"),
                part.name
            ));
        }
        java.line("this.implementation.start();");
        java.line("this.implementation.started = true;");
        java.close();
        Ok(())
    }

    fn render_part_initializers(&self, java: &mut JavaSource) -> GenResult<()> {
        for part in &self.component.parts {
            java.blank();
            java.open(format!("private void init_{}()", part.name));
            java.open(format!(
                "if (this.{} != null || this.implem_{} != null)",
                part.name, part.name
            ));
            java.line(format!(
                "throw new IllegalStateException(\"part `{}` initialized twice\");",
                part.name
            ));
            java.close();
            java.line(format!(
                "this.implem_{} = this.implementation.make_{}();",
                part.name, part.name
            ));
            java.open(format!("if (this.implem_{} == null)", part.name));
            java.line(format!(
                "throw new RuntimeException(\"make_{}() in {} should not return null.\");",
                part.name,
                self.component.fqcn()
            ));
            java.close();
            java.line(format!(
                "this.{} = this.implem_{}._newComponent(new BridgeImpl_{}(), false);",
                part.name, part.name, part.name
            ));
            java.open(format!("if (this.{} == null)", part.name));
            java.line(format!(
                "throw new RuntimeException(\"part `{}` component initialization returned null\");",
                part.name
            ));
            java.close();
            java.close();
        }

        java.blank();
        if self.component.parent.is_some() {
            java.line("@Override");
        }
        java.open("protected void initParts()");
        if self.component.parent.is_some() {
            java.line("super.initParts();");
        }
        for part in &self.component.parts {
            java.line(format!("init_{}();", part.name));
        }
        java.close();
        Ok(())
    }

    fn render_provided_initializers(&self, java: &mut JavaSource) -> GenResult<()> {
        for port in &self.component.provides {
            let is_override = self.catalog.parent_provides(self.component, &port.name)?;
            java.blank();
            if is_override {
                java.line("@Override");
            }
            java.open(format!("protected void init_{}()", port.name));
            if matches!(port.implementation, ProvidedServiceImplementation::Local) {
                java.open(format!("if (this.{} != null)", port.name));
                java.line(format!(
                    "throw new IllegalStateException(\"provided port `{}` initialized twice\");",
                    port.name
                ));
                java.close();
                java.line(format!(
                    "this.{} = this.implementation.make_{}();",
                    port.name, port.name
                ));
                java.open(format!("if (this.{} == null)", port.name));
                java.line(format!(
                    "throw new RuntimeException(\"make_{}() in {} should not return null.\");",
                    port.name,
                    self.component.fqcn()
                ));
                java.close();
            } else {
                java.line("// Delegated ports are resolved by their accessor.");
            }
            java.close();
        }

        java.blank();
        if self.component.parent.is_some() {
            java.line("@Override");
        }
        java.open("protected void initProvidedPorts()");
        if self.component.parent.is_some() {
            java.line("super.initProvidedPorts();");
        }
        for port in &self.component.provides {
            if !self.catalog.parent_provides(self.component, &port.name)? {
                java.line(format!("init_{}();", port.name));
            }
        }
        java.close();
        Ok(())
    }

    fn render_component_impl_constructor(
        &self,
        java: &mut JavaSource,
        instance: &Instance,
        parent: Option<&Instance>,
    ) -> GenResult<()> {
        java.blank();
        java.open(format!(
            "public ComponentImpl({} implem, {} b, boolean doInits)",
            self.outer_ref(instance),
            self.root_requires_ref(instance)?
        ));
        if parent.is_some() {
            java.line("super(implem, b, false);");
        }
        java.open("if (implem == null)");
        java.line("throw new NullPointerException(\"component implementation must not be null\");");
        java.close();
        java.open("if (b == null)");
        java.line("throw new NullPointerException(\"required-port bridge must not be null\");");
        java.close();
        java.line("this.bridge = b;");
        java.line("this.implementation = implem;");
        java.open("if (implem.selfComponent != null)");
        java.line(
            "throw new IllegalStateException(\"implementation is already associated with a component\");",
        );
        java.close();
        java.line("implem.selfComponent = this;");
        java.open("if (doInits)");
        java.line("initParts();");
        java.line("initProvidedPorts();");
        java.close();
        java.close();
        Ok(())
    }

    fn render_provided_fields_and_accessors(&self, java: &mut JavaSource, instance: &Instance) {
        for port in &self.component.provides {
            if matches!(port.implementation, ProvidedServiceImplementation::Local) {
                java.blank();
                java.line(format!(
                    "private {} {};",
                    substitute_type(instance, &port.type_name),
                    port.name
                ));
            }
            java.blank();
            java.line("@Override");
            java.open(format!(
                "public {} {}()",
                substitute_type(instance, &port.type_name),
                port.name
            ));
            match &port.implementation {
                ProvidedServiceImplementation::Local => {
                    java.line(format!("return this.{};", port.name));
                }
                ProvidedServiceImplementation::Delegated(reference) => {
                    java.line(format!(
                        "return this.{}().{}();",
                        reference.part_name, reference.service_name
                    ));
                }
            }
            java.close();
        }
    }

    fn render_part_fields_bridges_and_accessors(
        &self,
        java: &mut JavaSource,
        owner_instance: &Instance,
    ) -> GenResult<()> {
        let owner_requires = self.catalog.all_requires(owner_instance)?;
        let owner_provides = self.catalog.all_provides(owner_instance)?;
        let owner_parts = self.catalog.all_parts(owner_instance)?;

        for part in &self.component.parts {
            let part_instance = self.catalog.part_instance(self.component, part)?;
            java.blank();
            java.line(format!(
                "private {} {};",
                self.nested_ref(&part_instance, "Component"),
                part.name
            ));
            java.line(format!(
                "private {} implem_{};",
                self.outer_ref(&part_instance),
                part.name
            ));

            java.blank();
            java.open(format!(
                "private final class BridgeImpl_{} implements {}",
                part.name,
                self.root_requires_ref(&part_instance)?
            ));
            let required_ports = self.catalog.all_requires(&part_instance)?;
            for required in required_ports {
                let binding = part
                    .bindings
                    .iter()
                    .find(|binding| binding.required_name == required.name)
                    .ok_or_else(|| {
                        GenerationError::new(format!(
                            "part `{}` is missing a binding for `{}`",
                            part.name, required.name
                        ))
                    })?;
                let (_, expression) = resolve_binding_target(
                    binding,
                    &owner_requires,
                    &owner_provides,
                    &owner_parts,
                    self.catalog,
                )?;
                java.blank();
                java.line("@Override");
                java.open(format!(
                    "public final {} {}()",
                    required.type_name, required.name
                ));
                java.line(format!("return {expression};"));
                java.close();
            }
            java.close();

            java.blank();
            java.line("@Override");
            java.open(format!(
                "public final {} {}()",
                self.nested_ref(&part_instance, "Component"),
                part.name
            ));
            java.line(format!("return this.{};", part.name));
            java.close();
        }
        Ok(())
    }

    fn is_abstract(&self, instance: &Instance) -> GenResult<bool> {
        if !self.catalog.all_parts(instance)?.is_empty() {
            return Ok(true);
        }
        Ok(self
            .catalog
            .all_provides(instance)?
            .iter()
            .any(|port| matches!(port.implementation, ProvidedServiceImplementation::Local)))
    }

    fn visible_component_name(&self, target: &JavaComponent) -> String {
        if target.fqcn() == self.component.fqcn()
            || target.namespace == self.component.namespace
            || self
                .component
                .imports
                .iter()
                .any(|path| path.join(".") == target.fqcn())
        {
            target.name.clone()
        } else {
            target.fqcn()
        }
    }

    fn outer_ref(&self, instance: &Instance) -> String {
        format!(
            "{}{}",
            self.visible_component_name(&instance.component),
            generic_use(instance.argument.as_deref())
        )
    }

    fn nested_ref(&self, instance: &Instance, nested: &str) -> String {
        format!(
            "{}.{}{}",
            self.visible_component_name(&instance.component),
            nested,
            generic_use(instance.argument.as_deref())
        )
    }

    fn root_requires_ref(&self, instance: &Instance) -> GenResult<String> {
        let root = self.catalog.root_instance(instance)?;
        Ok(self.nested_ref(&root, "Requires"))
    }
}

fn generic_declaration(parameter: Option<&str>) -> String {
    parameter
        .map(|parameter| format!("<{parameter}>"))
        .unwrap_or_default()
}

fn generic_use(argument: Option<&str>) -> String {
    argument
        .map(|argument| format!("<{argument}>"))
        .unwrap_or_default()
}

fn local_nested_ref(nested: &str, argument: Option<&str>) -> String {
    format!("{nested}{}", generic_use(argument))
}

fn render_component_access_checks(java: &mut JavaSource, accessor: &str) {
    java.open("if (this.selfComponent == null)");
    java.line("throw new IllegalStateException(\"component runtime is not initialized\");");
    java.close();
    java.open("if (!this.init)");
    java.line(format!(
        "throw new RuntimeException(\"{accessor}() can't be accessed until a component has been created from this implementation; use start() instead of the constructor for initialization.\");"
    ));
    java.close();
}

#[derive(Debug, Default)]
struct JavaSource {
    source: String,
    indent: usize,
}

impl JavaSource {
    fn line(&mut self, line: impl AsRef<str>) {
        for _ in 0..self.indent {
            self.source.push_str("    ");
        }
        self.source.push_str(line.as_ref());
        self.source.push('\n');
    }

    fn blank(&mut self) {
        if !self.source.ends_with("\n\n") {
            self.source.push('\n');
        }
    }

    fn open(&mut self, declaration: impl AsRef<str>) {
        self.line(format!("{} {{", declaration.as_ref()));
        self.indent += 1;
    }

    fn close(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.line("}");
    }

    fn finish(mut self) -> String {
        if !self.source.ends_with('\n') {
            self.source.push('\n');
        }
        self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::speadl::parser::Parser;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse(source: &str) -> Ast {
        let mut parser = Parser::new(source);
        parser.next_token().expect("test source should lex");
        parser.namespace().expect("test source should parse")
    }

    fn examples() -> Vec<Ast> {
        vec![
            parse(
                "import ex1.Start namespace ex1.simple { \
                 component Simple { provides starter: Start } }",
            ),
            parse(
                "import ex1.Start namespace ex1.client { \
                 component Client { requires demarreur: Start provides letsgo: Runnable } }",
            ),
            parse(
                "namespace ex1.codec { \
                 component Codec[Service] { requires message: Service provides crypt: Service } }",
            ),
            parse(
                "import ex1.Start namespace ex1.traceur { \
                 component Traceur { requires starter: Start provides demarreur: Start } }",
            ),
            parse(
                "import ex1.simple.Simple import ex1.client.Client \
                 namespace ex1.composite { component Composite { \
                 provides service: Runnable = client.letsgo \
                 part simple: Simple {} \
                 part client: Client { bind demarreur to simple.starter } } }",
            ),
            parse(
                "import ex1.codec.Codec import ex1.traceur.Traceur import ex1.Start \
                 namespace ex1.cypher { component Cypher specializes Traceur { \
                 provides demarreur: Start = decodeur.crypt \
                 part codeur: Codec[Start] { bind message to starter } \
                 part decodeur: Codec[Start] { bind message to codeur.crypt } } }",
            ),
            parse(
                "namespace ex1.parent { \
                 component Parent[Service] { requires input: Service } }",
            ),
            parse(
                "import ex1.parent.Parent namespace ex1.child { \
                 component Child[T] specializes Parent[String] {} }",
            ),
        ]
    }

    #[test]
    fn emits_official_component_shape_and_eager_initialization() {
        let asts = examples();
        let source = GenJava::new(asts[4].clone())
            .with_dependencies(asts)
            .render()
            .expect("Composite generation should succeed");

        assert!(source.contains("public abstract class Composite"));
        assert!(source.contains("public static interface Requires"));
        assert!(source.contains("public static interface Provides"));
        assert!(source.contains("public static interface Component extends Provides"));
        assert!(source.contains("public static interface Parts"));
        assert!(source.contains("public static class ComponentImpl implements Component, Parts"));
        assert!(source.contains("protected abstract Simple make_simple();"));
        assert!(
            source.contains("private final class BridgeImpl_client implements Client.Requires")
        );
        assert!(source.contains("public final Start demarreur()"));
        assert!(source.contains("return ComponentImpl.this.simple().starter();"));
        assert!(source.contains("initParts();"));
        assert!(source.contains("initProvidedPorts();"));
        assert!(source.contains("return this.client().letsgo();"));
    }

    #[test]
    fn incompatible_binding_types_are_rejected_before_rendering() {
        let source = parse(
            "namespace mismatch { \
             component Source { provides value: String } }",
        );
        let sink = parse(
            "namespace mismatch { \
             component Sink { requires input: Integer } }",
        );
        let owner = parse(
            "import mismatch.Source import mismatch.Sink \
             namespace mismatch { component Owner { \
             part source: Source \
             part sink: Sink { bind input to source.value } } }",
        );

        let result = GenJava::new(owner)
            .with_dependencies(vec![source, sink])
            .render();

        assert!(
            result.is_err(),
            "Java generation must reject a String service bound to an Integer requirement"
        );
    }

    #[test]
    fn incompatible_delegated_port_types_are_rejected_before_rendering() {
        let source = parse(
            "namespace mismatch { \
             component Source { provides value: String } }",
        );
        let owner = parse(
            "import mismatch.Source namespace mismatch { component Owner { \
             provides value: Integer = source.value \
             part source: Source } }",
        );

        let result = GenJava::new(owner).with_dependencies(vec![source]).render();

        assert!(
            result.is_err(),
            "Java generation must reject delegation from String to Integer"
        );
    }

    #[test]
    fn specialization_and_single_generic_substitution_match_the_jvm_model() {
        let asts = examples();
        let source = GenJava::new(asts[5].clone())
            .with_dependencies(asts)
            .render()
            .expect("Cypher generation should succeed");

        assert!(source.contains("public abstract class Cypher extends Traceur"));
        assert!(!source.contains("interface Requires {"));
        assert!(source.contains("public static interface Provides extends Traceur.Provides"));
        assert!(source.contains("public static class ComponentImpl extends Traceur.ComponentImpl"));
        assert!(source.contains("protected final Start make_demarreur()"));
        assert!(source.contains("protected void init_demarreur()"));
        assert!(
            source
                .contains("private final class BridgeImpl_codeur implements Codec.Requires<Start>")
        );
        assert!(source.contains("public final Start message()"));
        assert!(!source.contains("public Component newComponent()"));
    }

    #[test]
    fn explicit_parent_argument_is_used_by_every_inherited_java_type() {
        let asts = examples();
        let source = GenJava::new(asts[7].clone())
            .with_dependencies(asts)
            .render()
            .expect("parameterized specialization should render");

        assert!(source.contains("public class Child<T> extends Parent<String>"));
        assert!(
            source.contains("public static interface Provides<T> extends Parent.Provides<String>")
        );
        assert!(source.contains(
            "public static interface Component<T> extends Parent.Component<String>, Provides<T>"
        ));
        assert!(source.contains("public static interface Parts<T> extends Parent.Parts<String>"));
        assert!(
            source.contains(
                "public static class ComponentImpl<T> extends Parent.ComponentImpl<String>"
            )
        );
        assert!(source.contains("Parent.Requires<String>"));
    }

    #[test]
    fn parent_parameter_is_substituted_when_a_child_is_used_as_a_part() {
        let parent = parse(
            "namespace generic { \
             component Parent[Service] { requires input: Service } }",
        );
        let child = parse(
            "namespace generic { \
             component Child[T] specializes Parent[T] {} }",
        );
        let owner = parse(
            "import generic.Child namespace demo { component Owner { \
             requires source: String \
             part child: Child[String] { bind input to source } } }",
        );
        let source = GenJava::new(owner)
            .with_dependencies(vec![parent, child])
            .render()
            .expect("the child argument should flow into its generic parent");

        assert!(source.contains("protected abstract Child<String> make_child();"));
        assert!(source.contains("implements generic.Parent.Requires<String>"));
        assert!(source.contains("public final String input()"));
    }

    #[test]
    fn absent_parent_argument_keeps_implicit_child_parameter_compatibility() {
        let parent = parse("namespace generic { component Parent[Service] {} }");
        let child = parse("namespace generic { component Child[T] specializes Parent {} }");
        let source = GenJava::new(child)
            .with_dependencies(vec![parent])
            .render()
            .expect("the historical implicit parent argument should remain supported");

        assert!(source.contains("public class Child<T> extends Parent<T>"));
        assert!(source.contains("Parent.ComponentImpl<T>"));
    }

    #[test]
    fn parent_argument_is_rejected_for_a_non_generic_parent() {
        let parent = parse("namespace generic { component Parent {} }");
        let child = parse("namespace generic { component Child specializes Parent[String] {} }");
        let error = GenJava::new(child)
            .with_dependencies(vec![parent])
            .render()
            .expect_err("a non-generic parent cannot receive a type argument");

        assert!(
            error
                .to_string()
                .contains("supplies a type argument to non-generic parent")
        );
    }

    #[test]
    fn invalid_java_identifiers_are_reported_before_rendering() {
        let ast = Ast::SEQ(vec![Ast::Namespace {
            path: vec!["demo".to_string()],
            body: Box::new(Ast::Component {
                name: "class".to_string(),
                specializes: None,
                generic: None,
                body: Box::new(Ast::SEQ(vec![Ast::Provides {
                    name: "service".to_string(),
                    type_name: "Runnable".to_string(),
                    implementation: ProvidedServiceImplementation::Local,
                }])),
            }),
        }]);

        let error = GenJava::new(ast)
            .render()
            .expect_err("a Java keyword cannot be a class name");
        assert!(
            error
                .to_string()
                .contains("invalid Java identifier `class`")
        );
    }

    #[test]
    fn feature_names_are_unique_across_requires_provides_and_parts() {
        let ast = parse(
            "namespace demo { component Collision { \
             requires service: Runnable provides service: Runnable } }",
        );
        let error = GenJava::new(ast)
            .render()
            .expect_err("required and provided ports cannot share a name");
        assert!(
            error
                .to_string()
                .contains("reuses feature name `service` for a required port and a provided port")
        );
    }

    #[test]
    fn accessors_cannot_collide_with_generated_initializer_methods() {
        let ast = parse(
            "namespace demo { component SyntheticCollision { \
             provides foo: Runnable provides init_foo: Runnable } }",
        );
        let error = GenJava::new(ast)
            .render()
            .expect_err("an accessor cannot reuse a generated initializer name");
        let message = error.to_string();
        assert!(message.contains("generated Java method collision"));
        assert!(message.contains("`init_foo()`"));
        assert!(message.contains("initializer for provided port `foo`"));
        assert!(message.contains("accessor for provided port `init_foo`"));
    }

    #[test]
    fn generated_component_impl_fields_cannot_collide() {
        let ast = parse(
            "namespace demo { component FieldCollision { \
             provides implem_worker: Runnable part worker: Worker {} } }",
        );
        let error = GenJava::new(ast)
            .render()
            .expect_err("a provided-port field cannot reuse a part implementation field");
        let message = error.to_string();
        assert!(message.contains("generated Java field collision"));
        assert!(message.contains("`implem_worker`"));
        assert!(message.contains("storage for local provided port `implem_worker`"));
        assert!(message.contains("implementation storage for part `worker`"));
    }

    #[test]
    fn specialized_features_respect_inherited_names_except_provided_overrides() {
        let parent = parse(
            "namespace hierarchy { component Base { \
             requires inherited: Runnable provides service: Runnable } }",
        );
        let valid_override = parse(
            "namespace hierarchy { component Child specializes Base { \
             provides service: Runnable } }",
        );
        GenJava::new(valid_override)
            .with_dependencies(vec![parent.clone()])
            .render()
            .expect("a provided port may override an inherited provided port");

        let conflicting_provided = parse(
            "namespace hierarchy { component BadProvided specializes Base { \
             provides inherited: Runnable } }",
        );
        let error = GenJava::new(conflicting_provided)
            .with_dependencies(vec![parent.clone()])
            .render()
            .expect_err("a provided port cannot reuse an inherited required-port name");
        assert!(
            error
                .to_string()
                .contains("conflicts with an inherited required port")
        );

        let leaf = parse("namespace hierarchy { component Leaf {} }");
        let conflicting_part = parse(
            "namespace hierarchy { component BadPart specializes Base { \
             provides other: Runnable part service: Leaf {} } }",
        );
        let error = GenJava::new(conflicting_part)
            .with_dependencies(vec![parent, leaf])
            .render()
            .expect_err("a part cannot reuse an inherited provided-port name");
        assert!(
            error
                .to_string()
                .contains("conflicts with an inherited provided port")
        );
    }

    #[test]
    fn component_references_require_an_import_or_the_same_package() {
        let dependency =
            parse("namespace remote { component Remote { provides service: Runnable } }");
        let owner = parse(
            "namespace local { component Owner { provides ready: Runnable \
             part remote: Remote {} } }",
        );
        let error = GenJava::new(owner)
            .with_dependencies(vec![dependency])
            .render()
            .expect_err("a globally unique simple name is not an implicit import");
        assert!(
            error
                .to_string()
                .contains("is not in the same package and has no explicit import")
        );
    }

    #[test]
    fn raw_generic_parts_use_object_for_erased_port_signatures() {
        if Command::new("javac").arg("-version").output().is_err() {
            return;
        }

        let generic = parse(
            "namespace generic { component Box[T] { \
             requires input: T provides output: T } }",
        );
        let owner = parse(
            "import generic.Box namespace demo { component RawOwner { \
             requires source: Object provides ready: Runnable \
             part box: Box { bind input to source } } }",
        );
        let dependencies = vec![generic.clone(), owner.clone()];
        let source = GenJava::new(owner.clone())
            .with_dependencies(dependencies.clone())
            .render()
            .expect("a raw generic part should be accepted");
        assert!(source.contains("protected abstract Box make_box();"));
        assert!(source.contains("implements Box.Requires"));
        assert!(source.contains("public final Object input()"));
        assert!(!source.contains("Box.Requires<Object>"));

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("may-java-raw-generic-{unique}"));
        let classes = root.join("classes");
        fs::create_dir_all(&classes).expect("classes directory should be created");
        for ast in [generic, owner] {
            GenJava::new(ast)
                .with_dependencies(dependencies.clone())
                .with_output(Some(root.clone()))
                .generate()
                .expect("raw generic fixture should be generated");
        }
        let mut sources = Vec::new();
        collect_java_sources(&root, &mut sources);
        let compilation = Command::new("javac")
            .arg("-Xlint:all")
            .arg("-d")
            .arg(&classes)
            .args(&sources)
            .output()
            .expect("javac should start");
        if !compilation.status.success() {
            panic!(
                "raw generic part failed to compile:\n{}",
                String::from_utf8_lossy(&compilation.stderr)
            );
        }
        fs::remove_dir_all(root).expect("temporary Java project should be removed");
    }

    #[test]
    fn speadl_java_naming_conventions_are_checked() {
        let ast = parse("namespace demo { component invalid { provides Service: Runnable } }");
        let error = GenJava::new(ast)
            .render()
            .expect_err("component names must start with an uppercase letter");
        assert!(
            error
                .to_string()
                .contains("component name `invalid` must start with an uppercase letter")
        );
    }

    #[test]
    fn generated_components_compile_and_execute_with_javac() {
        if Command::new("javac").arg("-version").output().is_err()
            || Command::new("java").arg("-version").output().is_err()
        {
            return;
        }

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("may-java-runtime-{unique}"));
        let classes = root.join("classes");
        fs::create_dir_all(root.join("ex1")).expect("fixture package should be created");
        fs::create_dir_all(&classes).expect("classes directory should be created");
        fs::write(
            root.join("ex1/Start.java"),
            "package ex1; public interface Start { void go(); }\n",
        )
        .expect("service contract should be written");

        let asts = examples();
        for ast in &asts {
            GenJava::new(ast.clone())
                .with_dependencies(asts.clone())
                .with_output(Some(root.clone()))
                .generate()
                .expect("Java component generation should succeed");
        }

        let harness = r#"
package ex1.runtime;

import ex1.Start;
import ex1.client.Client;
import ex1.composite.Composite;
import ex1.simple.Simple;

public final class Harness {
    private static int calls;

    private static final class SimpleImpl extends Simple {
        @Override protected Start make_starter() {
            return () -> calls++;
        }
    }

    private static final class ClientImpl extends Client {
        @Override protected Runnable make_letsgo() {
            return () -> requires().demarreur().go();
        }
    }

    private static final class CompositeImpl extends Composite {
        @Override protected Simple make_simple() {
            return new SimpleImpl();
        }

        @Override protected Client make_client() {
            return new ClientImpl();
        }
    }

    public static void main(String[] args) {
        Composite.Component component = new CompositeImpl().newComponent();
        component.service().run();
        component.service().run();
        if (calls != 2) throw new AssertionError("delegation/binding failed: " + calls);
    }
}
"#;
        let harness_path = root.join("ex1/runtime/Harness.java");
        fs::create_dir_all(
            harness_path
                .parent()
                .expect("harness should have a package directory"),
        )
        .expect("harness package should be created");
        fs::write(&harness_path, harness).expect("runtime harness should be written");

        let mut sources = Vec::new();
        collect_java_sources(&root, &mut sources);
        let compilation = Command::new("javac")
            .arg("-Xlint:all")
            .arg("-d")
            .arg(&classes)
            .args(&sources)
            .output()
            .expect("javac should start");
        if !compilation.status.success() {
            panic!(
                "generated Java failed to compile:\n{}",
                String::from_utf8_lossy(&compilation.stderr)
            );
        }

        let execution = Command::new("java")
            .arg("-cp")
            .arg(&classes)
            .arg("ex1.runtime.Harness")
            .output()
            .expect("java should start");
        if !execution.status.success() {
            panic!(
                "generated Java runtime failed:\n{}",
                String::from_utf8_lossy(&execution.stderr)
            );
        }
        fs::remove_dir_all(root).expect("temporary Java project should be removed");
    }

    fn collect_java_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("fixture directory should be readable") {
            let path = entry.expect("fixture entry should be readable").path();
            if path.is_dir() {
                collect_java_sources(&path, sources);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("java") {
                sources.push(path);
            }
        }
        sources.sort();
    }
}
