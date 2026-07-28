use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyModule {
    pub body: Vec<PyStmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyStmt {
    ImportFrom(PyImportFrom),
    ClassDef(PyClassDef),
    FunctionDef(PyFunctionDef),
    Expr(PyExpr),
    Return(Option<PyExpr>),
    Raise(PyExpr),
    If(PyIf),
    Assign { targets: Vec<PyExpr>, value: PyExpr },
    Pass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyImportFrom {
    pub module: String,
    pub names: Vec<PyAlias>,
    pub level: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyAlias {
    pub name: String,
    pub asname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyClassDef {
    pub name: String,
    pub bases: Vec<PyExpr>,
    pub body: Vec<PyStmt>,
    pub decorators: Vec<PyExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyFunctionDef {
    pub name: String,
    pub args: PyArguments,
    pub body: Vec<PyStmt>,
    pub returns: Option<PyExpr>,
    pub decorators: Vec<PyExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyIf {
    pub test: PyExpr,
    pub body: Vec<PyStmt>,
    pub orelse: Vec<PyStmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyArguments {
    pub args: Vec<PyArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyArg {
    pub name: String,
    pub annotation: Option<PyExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyExpr {
    Name {
        id: String,
        ctx: PyExprContext,
    },
    Attribute {
        value: Box<PyExpr>,
        attr: String,
        ctx: PyExprContext,
    },
    Call {
        func: Box<PyExpr>,
        args: Vec<PyExpr>,
        keywords: Vec<PyKeyword>,
    },
    ConstantString(String),
    Compare {
        left: Box<PyExpr>,
        operator: PyCmpOp,
        right: Box<PyExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyKeyword {
    pub arg: Option<String>,
    pub value: PyExpr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyExprContext {
    Load,
    Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyCmpOp {
    Is,
}

#[derive(Debug)]
pub enum PythonUnparseError {
    Io(io::Error),
    Failed(String),
}

impl fmt::Display for PythonUnparseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to run Python AST unparser: {error}"),
            Self::Failed(message) => write!(f, "Python AST unparser failed: {message}"),
        }
    }
}

impl Error for PythonUnparseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Failed(_) => None,
        }
    }
}

impl From<io::Error> for PythonUnparseError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl PyModule {
    pub fn unparse(&self) -> Result<String, PythonUnparseError> {
        self.unparse_with_intermediate(None)
    }

    pub fn unparse_and_keep_intermediate(&self, path: &Path) -> Result<String, PythonUnparseError> {
        self.unparse_with_intermediate(Some(path))
    }

    fn unparse_with_intermediate(
        &self,
        intermediate_path: Option<&Path>,
    ) -> Result<String, PythonUnparseError> {
        let script = format!(
            r#"import ast

def class_def(name, bases, body, decorators):
    fields = {{
        "name": name,
        "bases": bases,
        "keywords": [],
        "body": body,
        "decorator_list": decorators,
    }}
    if "type_params" in ast.ClassDef._fields:
        fields["type_params"] = []
    return ast.ClassDef(**fields)

def function_def(name, args, body, returns, decorators):
    fields = {{
        "name": name,
        "args": args,
        "body": body,
        "decorator_list": decorators,
        "returns": returns,
        "type_comment": None,
    }}
    if "type_params" in ast.FunctionDef._fields:
        fields["type_params"] = []
    return ast.FunctionDef(**fields)

tree = {tree}
tree = ast.fix_missing_locations(tree)
print(ast.unparse(tree), end="")
"#,
            tree = self.to_python_ast_constructor()
        );

        if let Some(path) = intermediate_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, &script)?;
        }

        let output = run_python_ast_unparser(&script)?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(PythonUnparseError::Failed(stderr.trim().to_string()))
    }

    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.Module(body={}, type_ignores=[])",
            py_list(self.body.iter().map(PyStmt::to_python_ast_constructor))
        )
    }
}

impl PyStmt {
    fn to_python_ast_constructor(&self) -> String {
        match self {
            Self::ImportFrom(import_from) => import_from.to_python_ast_constructor(),
            Self::ClassDef(class_def) => class_def.to_python_ast_constructor(),
            Self::FunctionDef(function_def) => function_def.to_python_ast_constructor(),
            Self::Expr(expr) => format!("ast.Expr(value={})", expr.to_python_ast_constructor()),
            Self::Return(value) => match value {
                Some(value) => format!("ast.Return(value={})", value.to_python_ast_constructor()),
                None => String::from("ast.Return(value=None)"),
            },
            Self::Raise(exception) => format!(
                "ast.Raise(exc={}, cause=None)",
                exception.to_python_ast_constructor()
            ),
            Self::If(if_statement) => if_statement.to_python_ast_constructor(),
            Self::Assign { targets, value } => format!(
                "ast.Assign(targets={}, value={}, type_comment=None)",
                py_list(targets.iter().map(PyExpr::to_python_ast_constructor)),
                value.to_python_ast_constructor()
            ),
            Self::Pass => String::from("ast.Pass()"),
        }
    }
}

impl PyImportFrom {
    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.ImportFrom(module={}, names={}, level={})",
            py_string(&self.module),
            py_list(self.names.iter().map(PyAlias::to_python_ast_constructor)),
            self.level
        )
    }
}

impl PyAlias {
    pub fn import_all() -> Self {
        Self {
            name: String::from("*"),
            asname: None,
        }
    }

    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.alias(name={}, asname={})",
            py_string(&self.name),
            py_optional_string(self.asname.as_deref())
        )
    }
}

impl PyClassDef {
    fn to_python_ast_constructor(&self) -> String {
        format!(
            "class_def(name={}, bases={}, body={}, decorators={})",
            py_string(&self.name),
            py_list(self.bases.iter().map(PyExpr::to_python_ast_constructor)),
            py_list(self.body.iter().map(PyStmt::to_python_ast_constructor)),
            py_list(
                self.decorators
                    .iter()
                    .map(PyExpr::to_python_ast_constructor)
            )
        )
    }
}

impl PyFunctionDef {
    fn to_python_ast_constructor(&self) -> String {
        format!(
            "function_def(name={}, args={}, body={}, returns={}, decorators={})",
            py_string(&self.name),
            self.args.to_python_ast_constructor(),
            py_list(self.body.iter().map(PyStmt::to_python_ast_constructor)),
            py_optional_expr(self.returns.as_ref()),
            py_list(
                self.decorators
                    .iter()
                    .map(PyExpr::to_python_ast_constructor)
            )
        )
    }
}

impl PyIf {
    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.If(test={}, body={}, orelse={})",
            self.test.to_python_ast_constructor(),
            py_list(self.body.iter().map(PyStmt::to_python_ast_constructor)),
            py_list(self.orelse.iter().map(PyStmt::to_python_ast_constructor))
        )
    }
}

impl PyArguments {
    pub fn new(args: Vec<PyArg>) -> Self {
        Self { args }
    }

    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.arguments(posonlyargs=[], args={}, vararg=None, kwonlyargs=[], kw_defaults=[], kwarg=None, defaults=[])",
            py_list(self.args.iter().map(PyArg::to_python_ast_constructor))
        )
    }
}

impl PyArg {
    pub fn without_annotation(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            annotation: None,
        }
    }

    pub fn with_annotation(name: impl Into<String>, annotation: PyExpr) -> Self {
        Self {
            name: name.into(),
            annotation: Some(annotation),
        }
    }

    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.arg(arg={}, annotation={}, type_comment=None)",
            py_string(&self.name),
            py_optional_expr(self.annotation.as_ref())
        )
    }
}

impl PyExpr {
    pub fn load_name(id: impl Into<String>) -> Self {
        Self::Name {
            id: id.into(),
            ctx: PyExprContext::Load,
        }
    }

    pub fn store_name(id: impl Into<String>) -> Self {
        Self::Name {
            id: id.into(),
            ctx: PyExprContext::Store,
        }
    }

    pub fn load_attribute(value: PyExpr, attr: impl Into<String>) -> Self {
        Self::Attribute {
            value: Box::new(value),
            attr: attr.into(),
            ctx: PyExprContext::Load,
        }
    }

    pub fn store_attribute(value: PyExpr, attr: impl Into<String>) -> Self {
        Self::Attribute {
            value: Box::new(value),
            attr: attr.into(),
            ctx: PyExprContext::Store,
        }
    }

    pub fn call(func: PyExpr, args: Vec<PyExpr>) -> Self {
        Self::Call {
            func: Box::new(func),
            args,
            keywords: Vec::new(),
        }
    }

    pub fn call_with_keywords(func: PyExpr, keywords: Vec<PyKeyword>) -> Self {
        Self::Call {
            func: Box::new(func),
            args: Vec::new(),
            keywords,
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::ConstantString(value.into())
    }

    pub fn is(left: PyExpr, right: PyExpr) -> Self {
        Self::Compare {
            left: Box::new(left),
            operator: PyCmpOp::Is,
            right: Box::new(right),
        }
    }

    fn to_python_ast_constructor(&self) -> String {
        match self {
            Self::Name { id, ctx } => format!(
                "ast.Name(id={}, ctx={})",
                py_string(id),
                ctx.to_python_ast_constructor()
            ),
            Self::Attribute { value, attr, ctx } => format!(
                "ast.Attribute(value={}, attr={}, ctx={})",
                value.to_python_ast_constructor(),
                py_string(attr),
                ctx.to_python_ast_constructor()
            ),
            Self::Call {
                func,
                args,
                keywords,
            } => format!(
                "ast.Call(func={}, args={}, keywords={})",
                func.to_python_ast_constructor(),
                py_list(args.iter().map(PyExpr::to_python_ast_constructor)),
                py_list(keywords.iter().map(PyKeyword::to_python_ast_constructor))
            ),
            Self::ConstantString(value) => {
                format!("ast.Constant(value={})", py_string(value))
            }
            Self::Compare {
                left,
                operator,
                right,
            } => format!(
                "ast.Compare(left={}, ops=[{}], comparators=[{}])",
                left.to_python_ast_constructor(),
                operator.to_python_ast_constructor(),
                right.to_python_ast_constructor()
            ),
        }
    }
}

impl PyKeyword {
    pub fn named(arg: impl Into<String>, value: PyExpr) -> Self {
        Self {
            arg: Some(arg.into()),
            value,
        }
    }

    fn to_python_ast_constructor(&self) -> String {
        format!(
            "ast.keyword(arg={}, value={})",
            py_optional_string(self.arg.as_deref()),
            self.value.to_python_ast_constructor()
        )
    }
}

impl PyExprContext {
    fn to_python_ast_constructor(self) -> &'static str {
        match self {
            Self::Load => "ast.Load()",
            Self::Store => "ast.Store()",
        }
    }
}

impl PyCmpOp {
    fn to_python_ast_constructor(self) -> &'static str {
        match self {
            Self::Is => "ast.Is()",
        }
    }
}

fn run_python_ast_unparser(script: &str) -> Result<Output, PythonUnparseError> {
    match Command::new("python3").arg("-c").arg(script).output() {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(Command::new("python").arg("-c").arg(script).output()?)
        }
        Err(error) => Err(error.into()),
    }
}

fn py_optional_expr(value: Option<&PyExpr>) -> String {
    match value {
        Some(value) => value.to_python_ast_constructor(),
        None => String::from("None"),
    }
}

fn py_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => py_string(value),
        None => String::from("None"),
    }
}

fn py_list(values: impl Iterator<Item = String>) -> String {
    format!("[{}]", values.collect::<Vec<_>>().join(", "))
}

fn py_string(value: &str) -> String {
    let mut quoted = String::from("'");

    for ch in value.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '\'' => quoted.push_str("\\'"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(ch),
        }
    }

    quoted.push('\'');
    quoted
}
