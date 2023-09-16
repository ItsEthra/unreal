use super::Codegen;
use crate::{
    engine::{FunctionFlags, PropertyFlags},
    sdk::{Enum, Field, FieldOptions, Function, Object, Package, PropertyKind, Sdk, Struct},
    utils::Bitfield,
    State,
};
use anyhow::Result;
use petgraph::Direction::Outgoing;
use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt::Write as WriteFmt,
    fs::{self, File, OpenOptions},
    hash::Hasher,
    io::{BufWriter, Write as WriteIo},
    iter::successors,
    path::{Path, PathBuf},
};
use ucore::fqn;

pub struct RustOptions {
    pub path: PathBuf,
    pub glam: bool,
}

pub struct RustCodegen<'a> {
    options: &'a RustOptions,
    sdk: &'a Sdk,
}

impl<'a> Codegen<'a> for RustCodegen<'a> {
    type Options = RustOptions;

    fn new(sdk: &'a Sdk, options: &'a Self::Options) -> Result<Self> {
        Ok(Self { options, sdk })
    }

    fn generate(&self) -> Result<()> {
        let Self {
            options: RustOptions { path, .. },
            sdk,
        } = self;
        let crates = path.join("crates");

        fs::create_dir_all(&crates)?;
        let opts = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .clone();

        let mut lib = BufWriter::new(opts.open(path.join("lib.rs"))?);

        let mut workspace = BufWriter::new(opts.open(path.join("Cargo.toml"))?);
        writeln!(&mut workspace, "{}", include_str!("workspace.toml"))?;

        writeln!(
            &mut workspace,
            r#"[workspace.dependencies]
ucore = {{ path = "../ucore" }}
uproxy = {{ path = "uproxy" }}
memflex = "*"
"#
        )?;

        self.generate_proxy()?;

        for pkg in sdk.packages.node_weights() {
            writeln!(
                &mut workspace,
                "{pkg} = {{ path = \"crates/{pkg}\" }}",
                pkg = &*pkg.ident
            )?;

            self.generate_package(pkg, &crates)?;
        }

        const EPILOG: &str = r#"
[package]
name = "usdk"
version.workspace = true
edition.workspace = true

[lib]
path = "lib.rs"

[dependencies]
"#;

        writeln!(&mut workspace, "{EPILOG}")?;
        for pkg in sdk.packages.node_weights().map(|v| &*v.ident) {
            writeln!(
                &mut workspace,
                "{pkg} = {{ workspace = true, optional = true }}"
            )?;

            writeln!(&mut lib, "#[cfg(feature = \"{pkg}\")]")?;
            writeln!(&mut lib, "pub use {pkg};")?;
        }

        Ok(())
    }
}

impl RustCodegen<'_> {
    fn generate_proxy(&self) -> Result<()> {
        let proxy = self.options.path.join("uproxy");
        fs::create_dir_all(&proxy)?;

        let cargo = proxy.join("Cargo.toml");
        let lib = proxy.join("lib.rs");

        const CARGO: &str = r#"[package]
name = "uproxy"
version.workspace = true
edition.workspace = true  
"#;

        let mut cargo = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(cargo)?;
        cargo.write_all(CARGO.as_bytes())?;

        let mut lib = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(lib)?;

        let config = &State::get().config;
        writeln!(
            lib,
            "pub const PROCESS_EVENT_INDEX: usize = {:#X};",
            config.process_event
        )?;

        Ok(())
    }

    fn generate_package(&self, pkg: &Package, crates: &Path) -> Result<()> {
        let folder = crates.join(&*pkg.ident);
        fs::create_dir_all(&folder)?;

        let opts = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .clone();

        const PRELUDE: &str = r#"#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_imports,
    dead_code
)]

use ucore::{UObject, Ptr, TArray, TSet, TMap, FString, FName, SyncLazy, impl_uobject_like, impl_process_event_fns};
use std::{ptr::NonNull, mem::zeroed};
use uproxy::PROCESS_EVENT_INDEX;

"#;

        let mut lib = BufWriter::new(opts.open(folder.join(format!("{}.rs", pkg.ident)))?);
        lib.write_all(
            PRELUDE
                .replace('%', &format!("{:#X}", State::get().config.process_event))
                .as_bytes(),
        )?;

        for dep in self
            .sdk
            .packages
            .neighbors_directed(self.sdk.indices[&pkg.ident], Outgoing)
        {
            let pkg = self.sdk.packages.node_weight(dep).unwrap();
            writeln!(&mut lib, "use {}::*;", &pkg.ident)?;
        }
        writeln!(&mut lib)?;

        for obj in pkg.objects.iter() {
            match &**obj {
                Object::Enum(uenum) => self.generate_enum(&mut lib, uenum)?,
                Object::Struct(ustruct) | Object::Class(ustruct) => {
                    self.generate_struct(&mut lib, ustruct)?
                }
            }
        }

        let mut cargo: BufWriter<File> = BufWriter::new(opts.open(folder.join("Cargo.toml"))?);

        const PACKAGE: &str = r#"[package]
name = "%"
version.workspace = true
edition.workspace = true  

[lib]
path = "%.rs"

"#;

        cargo.write_all(PACKAGE.replace('%', &pkg.ident).as_bytes())?;

        writeln!(
            &mut cargo,
            r#"[dependencies]
memflex.workspace = true
uproxy.workspace = true
ucore.workspace = true
"#
        )?;

        for dep in self
            .sdk
            .packages
            .neighbors_directed(self.sdk.indices[&pkg.ident], Outgoing)
        {
            let pkg = self.sdk.packages.node_weight(dep).unwrap();
            writeln!(&mut cargo, "{}.workspace = true", &pkg.ident)?;
        }

        Ok(())
    }

    fn generate_enum(&self, w: &mut dyn WriteIo, uenum: &Enum) -> Result<()> {
        let Enum {
            fqn,
            ident,
            layout,
            variants,
        } = uenum;

        writeln!(w, "// `{fqn}`")?;
        writeln!(w, "// Size = {}", layout.size)?;
        writeln!(
            w,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(transparent)]"
        )?;
        writeln!(
            w,
            "pub struct {ident}(pub u{});\n\nimpl {ident} {{",
            layout.size * 8,
        )?;

        let mut used_names = HashSet::new();

        for (name, value) in variants.iter() {
            writeln!(
                w,
                "    pub const {name}{}: Self = Self({value}i64 as u{});",
                if used_names.contains(name) {
                    Cow::from(format!("_{value}"))
                } else {
                    Cow::from("")
                },
                uenum.layout.size * 8
            )?;

            used_names.insert(name);
        }

        writeln!(w, "}}\n")?;

        Ok(())
    }

    fn generate_struct(&self, w: &mut dyn WriteIo, ustruct: &Struct) -> Result<()> {
        let Struct {
            fqn,
            parent,
            ident,
            layout,
            fields,
            shrink,
            functions,
            is_uobject,
            ..
        } = ustruct;

        if *fqn == fqn!(CoreUObject.Object) {
            return Ok(());
        }

        writeln!(w, "memflex::makestruct! {{")?;
        writeln!(
            w,
            "    // Size = {:#X}({:#X}), Alignment = {:#X}{}",
            layout.size,
            layout.get_aligned_size(),
            layout.align,
            if let Some(size) = shrink.get() {
                Cow::from(format!(", Shrunk = {size:#X}"))
            } else {
                Cow::from("")
            }
        )?;

        let mut offset = 0;

        if let Some(ref parent_fqn) = parent {
            let (Object::Class(parent) | Object::Struct(parent)) =
            &*self.sdk.lookup(parent_fqn).unwrap().ptr
        else {
            unreachable!()
        };

            offset = parent
                .shrink
                .get()
                .unwrap_or(parent.layout.get_aligned_size());
            writeln!(w, "    // Name = `{fqn}`, Parent = `{parent_fqn}`")?;
            writeln!(
                w,
                "    // Parent = {:#X}{}",
                parent.layout.size,
                if let Some(size) = parent.shrink.get() {
                    Cow::from(format!(", Shrunk = {size:#X}"))
                } else {
                    Cow::from("")
                }
            )?;

            let chain = successors(Some(*parent_fqn), |fqn| {
            let (Object::Class(parent) | Object::Struct(parent)) = &*self.sdk.lookup(fqn).unwrap().ptr
            else {
                unreachable!()
            };

            parent.parent
        })
        .map(|fqn| self.sdk.lookup(&fqn).unwrap().ptr.ident())
        .collect::<Vec<_>>()
        .join(" -> ");
            writeln!(w, "    // Inheritance: {chain}")?;

            writeln!(w, "    pub struct {ident} : pub {} {{", &parent.ident)?;
        } else {
            writeln!(w, "    // Name = `{fqn}`")?;
            writeln!(w, "    pub struct {ident} {{")?;
        }

        let mut dedup = NameDedup::default();
        let mut bitfields = String::new();

        for field in fields {
            match field {
                Field::Property {
                    name,
                    kind,
                    options:
                        FieldOptions {
                            offset: field_offset,
                            elem_size,
                            array_dim,
                        },
                } => {
                    let Some(repr) = self.stringify_type(kind, PointerMode::Ptr) else {
                    continue;
                };

                    let total_size = *elem_size * *array_dim;
                    if *field_offset > offset {
                        writeln!(
                            w,
                            "        _pad_{offset:#X}: [u8; {size:#X}], // {offset:#X}({size:#X})",
                            size = *field_offset - offset
                        )?;

                        offset = *field_offset;
                    }

                    writeln!(
                        w,
                        "        pub {}: {repr}, // {offset:#X}({total_size:#X})",
                        dedup.entry(name)
                    )?;

                    offset += total_size;
                }
                Field::Bitfields(group) => {
                    if offset < group.offset {
                        writeln!(
                            w,
                            "        _pad_{o:#X}: [u8; {size:#X}], // {o:#X}({size:#X})",
                            size = group.offset - offset,
                            o = group.offset,
                        )?;
                    }

                    writeln!(
                        w,
                        "        bitfield_{o:#X}: u8, // {o:#X}(0x8)",
                        o = group.offset,
                    )?;

                    writeln!(
                        bitfields,
                        "    {}.bitfield_{:#X} : u8 {{",
                        ident, group.offset
                    )?;
                    for Bitfield { name, offset, len } in &group.items {
                        writeln!(
                            bitfields,
                            "        pub {name}: {offset}..={},",
                            *offset + *len - 1
                        )?;
                    }
                    writeln!(bitfields, "    }}\n")?;

                    // Groups are always 1 byte.
                    offset = group.offset + 1;
                }
            }
        }

        let struct_size = shrink.get().unwrap_or(layout.size);
        if offset < struct_size {
            writeln!(
                w,
                "        _pad_{offset:#X}: [u8; {size:#X}], // {offset:#X}({size:#X})",
                size = struct_size - offset,
            )?;
        }

        writeln!(w, "    }}\n}}\n")?;
        if *is_uobject {
            writeln!(w, "impl_uobject_like!({ident}, \"{fqn}\");\n",)?;
        }

        if !bitfields.is_empty() {
            writeln!(w, "memflex::bitfields! {{")?;
            write!(w, "{bitfields}")?;
            writeln!(w, "}}\n")?;
        }

        let funcs = functions.borrow();
        if !funcs.is_empty() {
            let (static_fns, nonstatic_fns) = funcs
                .iter()
                .partition::<Vec<_>, _>(|f| f.flags.contains(FunctionFlags::Static));

            writeln!(
                w,
                "impl_process_event_fns! {{\n    [{ident}, PROCESS_EVENT_INDEX]\n",
            )?;

            let mut funcd = NameDedup::default();
            for func in static_fns.iter() {
                self.write_function(w, ident, func, &mut funcd)?;
            }

            let mut funcd = NameDedup::default();
            for func in nonstatic_fns.iter() {
                self.write_function(w, ident, func, &mut funcd)?;
            }

            writeln!(w, "}}\n")?;
        }

        Ok(())
    }

    fn write_function(
        &self,
        w: &mut dyn WriteIo,
        ident: &str,
        func: &Function,
        funcd: &mut NameDedup,
    ) -> Result<()> {
        let Function {
            ident: func_ident,
            args,
            flags,
            fqn,
            ..
        } = func;

        let mut argd = NameDedup::default();
        let mut args = args.to_vec();
        for arg in args.iter_mut() {
            arg.name = argd.entry(&arg.name).to_string()
        }
        drop(argd);

        let mut fargs = "".to_owned();
        let mut params = "".to_owned();

        for arg in args.iter() {
            let mode = if arg.flags.contains(PropertyFlags::OutParm) {
                PointerMode::Ptr
            } else {
                PointerMode::Mut
            };

            let part = format!(
                "{}: {}",
                &arg.name,
                self.stringify_type(&arg.kind, mode)
                    .unwrap_or_else(|| Cow::from("*const ()"))
            );

            if !arg.flags.contains(PropertyFlags::OutParm) {
                fargs += &part;
                fargs += ", ";
            }

            params += &part;
            params += ", ";
        }

        if !params.is_empty() {
            params.truncate(params.len() - 2);
        }

        if !fargs.is_empty() {
            fargs.truncate(fargs.len() - 2);
        }

        write!(
            w,
            "    {} {}({fargs}) ",
            if flags.contains(FunctionFlags::Static) {
                "static"
            } else {
                "fn"
            },
            funcd.entry(func_ident)
        )?;

        let ret = args
            .iter()
            .filter(|a| a.flags.contains(PropertyFlags::OutParm))
            .collect::<Vec<_>>();

        match ret.len() {
            0 => writeln!(w, "= \"{fqn}\"; {{ {params} }}")?,
            1 => {
                let ty = self
                    .stringify_type(&ret[0].kind, PointerMode::Ptr)
                    .unwrap_or_else(|| Cow::from("*const ()"));
                writeln!(
                    w,
                    "-> [{ident}_{func_ident}Result; {}: {ty}] = \"{fqn}\"; {{ {params} }}",
                    &ret[0].name
                )?;
            }
            _ => {
                write!(w, "-> [{ident}_{func_ident}Result; ")?;
                for (i, arg) in ret.iter().enumerate() {
                    let ty = self
                        .stringify_type(&arg.kind, PointerMode::Ptr)
                        .unwrap_or_else(|| Cow::from("*const ()"));
                    write!(
                        w,
                        "{}: {}{}",
                        &arg.name,
                        ty,
                        if i == ret.len() - 1 { "" } else { ", " }
                    )?;
                }
                writeln!(w, "] = \"{fqn}\"; {{ {params} }}")?;
            }
        }

        Ok(())
    }
    fn stringify_type(&self, kind: &PropertyKind, mode: PointerMode) -> Option<Cow<'static, str>> {
        let repr: Cow<str> = match kind {
            PropertyKind::Bool => "bool".into(),
            PropertyKind::Int8 => "i8".into(),
            PropertyKind::Int16 => "i16".into(),
            PropertyKind::Int32 => "i32".into(),
            PropertyKind::Int64 => "i64".into(),
            PropertyKind::UInt8 => "u8".into(),
            PropertyKind::UInt16 => "u16".into(),
            PropertyKind::UInt32 => "u32".into(),
            PropertyKind::UInt64 => "u64".into(),
            PropertyKind::Float32 => "f32".into(),
            PropertyKind::Float64 => "f64".into(),
            PropertyKind::Name => "FName".into(),
            PropertyKind::String => "FString".into(),
            PropertyKind::Ptr(inner) => {
                let object = self.sdk.lookup(inner).unwrap();
                match mode {
                    PointerMode::Mut => format!("*mut {}", object.ptr.ident()).into(),
                    PointerMode::Ptr => format!("Option<Ptr<{}>>", object.ptr.ident()).into(),
                }
            }
            PropertyKind::Inline(inner) => {
                let object = self.sdk.lookup(inner).unwrap();
                object.ptr.ident().to_owned().into()
            }
            PropertyKind::Array { kind, size } => {
                format!("[{}; {:#X}]", self.stringify_type(kind, mode)?, *size).into()
            }
            PropertyKind::Vec(inner) => {
                format!("TArray<{}>", self.stringify_type(inner, mode)?).into()
            }
            PropertyKind::Set(inner) => {
                format!("TSet<{}>", self.stringify_type(inner, mode)?).into()
            }
            PropertyKind::Map { key, value } => format!(
                "TMap<{}, {}>",
                self.stringify_type(key, mode)?,
                self.stringify_type(value, mode)?
            )
            .into(),
            // TODO: stringify text and implement in ucore
            PropertyKind::Text | PropertyKind::Unknown => return None,
        };

        Some(repr)
    }
}

#[derive(Clone, Copy)]
enum PointerMode {
    Mut,
    Ptr,
}

#[derive(Default)]
struct NameDedup(HashMap<u64, usize>);

impl NameDedup {
    fn entry<'n>(&mut self, name: &'n str) -> Cow<'n, str> {
        let mut hasher = DefaultHasher::new();
        hasher.write_usize(name.len());
        hasher.write(name.as_bytes());

        let i = self.0.entry(hasher.finish()).or_insert(0);
        *i += 1;

        match *i {
            1 => Cow::Borrowed(name),
            j => Cow::Owned(format!("{name}_{j}")),
        }
    }
}
