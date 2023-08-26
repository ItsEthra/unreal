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
    path::Path,
};
use ucore::fqn;

pub fn generate_rust_sdk(path: impl AsRef<Path>, sdk: &Sdk) -> Result<()> {
    let path = path.as_ref().to_path_buf();
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
        "[workspace.dependencies]\nucore = {{ path = \"../ucore\" }}\nmemflex = \"0.7.0\""
    )?;

    for pkg in sdk.packages.node_weights() {
        writeln!(
            &mut workspace,
            "{pkg} = {{ path = \"crates/{pkg}\" }}",
            pkg = &*pkg.ident
        )?;

        generate_package(pkg, &crates, sdk)?;
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

fn generate_package(pkg: &Package, crates: &Path, sdk: &Sdk) -> Result<()> {
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

use ucore::{Ptr, TArray, TSet, TMap, FString, FName, SyncLazy, impl_uobject_like, impl_process_event_fns};
use std::{ptr::NonNull, mem::zeroed};

type UObject = ucore::UObject<%>;

"#;

    let mut lib = BufWriter::new(opts.open(folder.join(format!("{}.rs", pkg.ident)))?);
    lib.write_all(
        PRELUDE
            .replace('%', &format!("{:#X}", State::get().config.process_event))
            .as_bytes(),
    )?;

    for dep in sdk
        .packages
        .neighbors_directed(sdk.indices[&pkg.ident], Outgoing)
    {
        let pkg = sdk.packages.node_weight(dep).unwrap();
        writeln!(&mut lib, "use {}::*;", &pkg.ident)?;
    }
    writeln!(&mut lib)?;

    for obj in pkg.objects.iter() {
        match &**obj {
            Object::Enum(uenum) => generate_enum(&mut lib, uenum)?,
            Object::Struct(ustruct) | Object::Class(ustruct) => {
                generate_struct(&mut lib, ustruct, sdk)?
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
        "[dependencies]\nucore.workspace = true\nmemflex.workspace = true\n"
    )?;

    for dep in sdk
        .packages
        .neighbors_directed(sdk.indices[&pkg.ident], Outgoing)
    {
        let pkg = sdk.packages.node_weight(dep).unwrap();
        writeln!(&mut cargo, "{}.workspace = true", &pkg.ident)?;
    }

    Ok(())
}

fn generate_enum(w: &mut dyn WriteIo, uenum: &Enum) -> Result<()> {
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
        "#[repr(transparent)]\npub struct {ident}(pub u{});\n\nimpl {ident} {{",
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

fn generate_struct(w: &mut dyn WriteIo, ustruct: &Struct, sdk: &Sdk) -> Result<()> {
    let Struct {
        fqn,
        parent,
        ident,
        layout,
        fields,
        shrink,
        functions,
        index,
        is_uobject,
    } = ustruct;

    if *fqn == fqn!("CoreUObject.Object") {
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
            &*sdk.lookup(parent_fqn).unwrap().ptr
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
            let (Object::Class(parent) | Object::Struct(parent)) = &*sdk.lookup(fqn).unwrap().ptr
            else {
                unreachable!()
            };

            parent.parent
        })
        .map(|fqn| sdk.lookup(&fqn).unwrap().ptr.ident())
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
                let Some(repr) = stringify_type(kind, sdk, PointerMode::Ptr) else {
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

    let config = &State::get().config;

    writeln!(w, "    }}\n}}\n")?;
    if *is_uobject {
        writeln!(
            w,
            "impl_uobject_like!({ident}, {:#X}, {index});\n",
            config.process_event
        )?;
    }

    if !bitfields.is_empty() {
        writeln!(w, "memflex::bitfields! {{")?;
        write!(w, "{bitfields}")?;
        writeln!(w, "}}\n")?;
    }

    let funcs = functions.borrow();
    let (static_fns, nonstatic_fns) = funcs
        .iter()
        .partition::<Vec<_>, _>(|f| f.flags.contains(FunctionFlags::Static));

    if !static_fns.is_empty() {
        writeln!(
            w,
            "impl_process_event_fns! {{\n    [{ident}, {:#X}],\n",
            config.process_event
        )?;

        let mut funcd = NameDedup::default();
        for func in static_fns.iter() {
            write_function(w, ident, func, sdk, &mut funcd)?;
        }

        writeln!(w, "}}\n")?;
    }

    // Doing it separately beceause `impl_process_event_fns` doesn't support static and non static functions in the same invokation,
    // and implementing it would be a pain in the ass.
    if !nonstatic_fns.is_empty() {
        writeln!(
            w,
            "impl_process_event_fns! {{\n    [{ident}, {:#X}],\n",
            config.process_event
        )?;

        let mut funcd = NameDedup::default();
        for func in nonstatic_fns.iter() {
            write_function(w, ident, func, sdk, &mut funcd)?;
        }

        writeln!(w, "}}\n")?;
    }

    Ok(())
}

fn write_function(
    w: &mut dyn WriteIo,
    ident: &str,
    func: &Function,
    sdk: &Sdk,
    funcd: &mut NameDedup,
) -> Result<()> {
    let Function {
        ident: func_ident,
        index,
        args,
        ret,
        flags,
    } = func;

    let mut argd = NameDedup::default();

    let args = args
        .iter()
        .map(|arg| {
            let mode = if arg.flags.contains(PropertyFlags::ConstParm) {
                PointerMode::Const
            } else {
                PointerMode::Mut
            };

            format!(
                "{}: {}",
                argd.entry(&arg.name),
                stringify_type(&arg.kind, sdk, mode).unwrap_or_else(|| Cow::from("*const ()"))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    write!(
        w,
        "    pub {}fn {}({args}) ",
        if flags.contains(FunctionFlags::Static) {
            "static "
        } else {
            ""
        },
        funcd.entry(func_ident)
    )?;

    match ret.len() {
        0 => writeln!(w, "= {index:#X};")?,
        1 => {
            let ty = stringify_type(&ret[0].kind, sdk, PointerMode::Ptr)
                .unwrap_or_else(|| Cow::from("*const ()"));
            writeln!(
                w,
                "-> [<{ident}_{func_ident}Result> {}: {ty}] = {index:#X};",
                argd.entry(&ret[0].name)
            )?;
        }
        _ => {
            write!(w, "-> [<{ident}_{func_ident}Result> ")?;
            for (i, arg) in ret.iter().enumerate() {
                let ty = stringify_type(&arg.kind, sdk, PointerMode::Ptr)
                    .unwrap_or_else(|| Cow::from("*const ()"));
                write!(
                    w,
                    "{}: {}{}",
                    argd.entry(&arg.name),
                    ty,
                    if i == ret.len() - 1 { "" } else { ", " }
                )?;
            }
            writeln!(w, "] = {index:#X};")?;
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum PointerMode {
    Const,
    Mut,
    Ptr,
}

fn stringify_type(kind: &PropertyKind, sdk: &Sdk, mode: PointerMode) -> Option<Cow<'static, str>> {
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
            let object = sdk.lookup(inner).unwrap();
            match mode {
                PointerMode::Const => format!("*const {}", object.ptr.ident()).into(),
                PointerMode::Mut => format!("*mut {}", object.ptr.ident()).into(),
                PointerMode::Ptr => format!("Option<Ptr<{}>>", object.ptr.ident()).into(),
            }
        }
        PropertyKind::Inline(inner) => {
            let object = sdk.lookup(inner).unwrap();
            object.ptr.ident().to_owned().into()
        }
        PropertyKind::Array { kind, size } => {
            format!("[{}; {:#X}]", stringify_type(kind, sdk, mode)?, *size).into()
        }
        PropertyKind::Vec(inner) => format!("TArray<{}>", stringify_type(inner, sdk, mode)?).into(),
        PropertyKind::Set(inner) => format!("TSet<{}>", stringify_type(inner, sdk, mode)?).into(),
        PropertyKind::Map { key, value } => format!(
            "TMap<{}, {}>",
            stringify_type(key, sdk, mode)?,
            stringify_type(value, sdk, mode)?
        )
        .into(),
        // TODO: stringify text and implement in ucore
        PropertyKind::Text | PropertyKind::Unknown => return None,
    };

    Some(repr)
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
