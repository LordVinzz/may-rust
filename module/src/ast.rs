#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    SEQ(Vec<Ast>),

    Package {
        name: String,
    },

    Class {
        name: String,
        access: Option<String>,
        body: Box<Ast>,
    },

    Function {
        name: String,
        type_name: String,
        access: Option<String>,
        body: Option<Box<Ast>>,
    },
}
