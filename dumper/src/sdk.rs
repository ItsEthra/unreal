use crate::{
    engine::{FunctionFlags, PropertyFlags},
    utils::{BitfieldGroup, Fqn, Layout},
    State,
};
use petgraph::{graph::NodeIndex, stable_graph::StableGraph, Directed};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::{self, Debug},
    rc::Rc,
};

/// TODO: Someone please educate me on weak pointers.
/// I can only assume that in this file, I would mostly want to use Weak instead of Rc pointers,
/// but they are not very convenient to work with and I don't know if
/// there is currently any drawbacks due to only using Rc.

#[derive(Default)]
pub struct Sdk {
    pub packages: StableGraph<Package, (), Directed>,
    pub indices: HashMap<Rc<str>, NodeIndex>,
    pub owned: HashMap<Fqn, ObjectInfo>,
}

pub struct Package {
    pub ident: Rc<str>,
    pub objects: Vec<Rc<Object>>,
}

impl Debug for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ident)
    }
}

impl Package {
    fn empty(ident: &Rc<str>) -> Self {
        Self {
            ident: ident.clone(),
            objects: vec![],
        }
    }
}

pub struct ObjectInfo {
    pub package: NodeIndex,
    pub ptr: Rc<Object>,
}

#[derive(Debug)]
pub enum Object {
    Enum(Enum),
    Struct(Struct),
    Class(Struct),
}

impl Object {
    pub fn ident(&self) -> &str {
        match self {
            Object::Enum(v) => &v.ident,
            Object::Class(v) | Object::Struct(v) => &v.ident,
        }
    }

    pub fn fqn(&self) -> Fqn {
        match self {
            Object::Enum(v) => v.fqn,
            Object::Class(v) | Object::Struct(v) => v.fqn,
        }
    }

    pub fn layout(&self) -> Layout {
        match self {
            Object::Enum(v) => v.layout,
            Object::Class(v) | Object::Struct(v) => v.layout,
        }
    }
}

impl Sdk {
    pub fn lookup(&self, fqn: &Fqn) -> Option<&ObjectInfo> {
        self.owned.get(fqn)
    }

    pub fn retrieve_key(&mut self, package_ident: &str) -> NodeIndex {
        let merged_ident = State::get()
            .options
            .merge
            .get(package_ident)
            .map(|s| s.as_str())
            .unwrap_or(package_ident);

        if let Some(key) = self.indices.get(merged_ident) {
            *key
        } else {
            let k: Rc<str> = merged_ident.into();
            let v = self.packages.add_node(Package::empty(&k));
            self.indices.insert(k, v);
            v
        }
    }

    pub fn add(&mut self, package_ident: &str, object: Object) {
        let idx = self.retrieve_key(package_ident);
        let package = self.packages.node_weight_mut(idx).unwrap();

        let object_ref = Rc::new(object);
        self.owned.insert(
            object_ref.fqn(),
            ObjectInfo {
                ptr: object_ref.clone(),
                package: idx,
            },
        );
        package.objects.push(object_ref);
    }
}

#[derive(Debug)]
pub struct Enum {
    pub fqn: Fqn,
    pub ident: String,
    pub layout: Layout,
    pub variants: Vec<(String, i64)>,
}

#[derive(Debug)]
pub struct Struct {
    pub fqn: Fqn,
    pub index: u32,
    pub parent: Option<Fqn>,
    pub ident: String,
    pub shrink: Cell<Option<usize>>,
    pub layout: Layout,
    pub fields: Vec<Field>,
    pub functions: RefCell<Vec<Function>>,
}

#[derive(Debug)]
pub struct Function {
    pub ident: String,
    pub index: u32,
    pub flags: FunctionFlags,
    pub args: Vec<FunctionArg>,
    pub ret: Vec<FunctionArg>,
}

#[derive(Debug)]
pub struct FunctionArg {
    pub name: String,
    pub kind: PropertyKind,
    pub flags: PropertyFlags,
}

#[derive(Debug)]
pub enum Field {
    Property {
        name: String,
        kind: PropertyKind,
        options: FieldOptions,
    },
    Bitfields(BitfieldGroup),
}

impl Field {
    pub fn offset(&self) -> usize {
        match self {
            Field::Property { options, .. } => options.offset,
            Field::Bitfields(group) => group.offset,
        }
    }
}

#[derive(Debug)]
pub struct FieldOptions {
    pub offset: usize,
    pub elem_size: usize,
    pub array_dim: usize,
}

impl FieldOptions {
    pub fn total_size(&self) -> usize {
        self.array_dim * self.elem_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[rustfmt::skip]
pub enum PropertyKind {
    Bool,
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Name,        // FName
    String,      // FString
    Text,        // FText
    Ptr(Fqn),    // Pointer to an object
    Inline(Fqn), // Inline enum or a struct
    Array {
        kind: Box<PropertyKind>,
        size: usize,
    },
    Vec(Box<PropertyKind>), // TArray,
    Set(Box<PropertyKind>), // TSet,
    Map {
        key: Box<PropertyKind>,
        value: Box<PropertyKind>,
    }, // TMap,
    Unknown
}
