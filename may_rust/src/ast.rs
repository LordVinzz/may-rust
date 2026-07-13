#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceReference {
    pub part_name: String,
    pub service_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct  Specializes {
    pub parent: String,
    pub parent_file: Option<Box<Ast>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvidedServiceImplementation {
    Local,
    Delegated(ServiceReference),
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    SEQ(Vec<Ast>),

    Import {
        path: Vec<String>,
    },

    Namespace {
        path: Vec<String>,
        body: Box<Ast>,
    },

    Component {
        name: String,
        specializes: Option<Specializes>,
        generic: Option<String>,
        body: Box<Ast>,
    },

    Requires {
        name: String,
        type_name: String,
    },

    Provides {
        name: String,
        type_name: String,
        implementation: ProvidedServiceImplementation,
    },

    Part {
        name: String,
        type_name: String,
        generic: Option<String>,
        body: Box<Ast>,
    },

    Bind {
        name: String,
        target: Vec<String>,
    },
}
