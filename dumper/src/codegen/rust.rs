use crate::{
    fqn,
    sdk::{Enum, Field, FieldOptions, Function, Object, Package, PropertyKind, Sdk, Struct},
    utils::Bitfield,
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
    path::Path,
};

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

    const PRELUDE: &str = r#"
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_imports,
    dead_code
)]

use ucore::{Ptr, TArray, TSet, TMap, FString, FName, SyncLazy};
use std::{ptr::NonNull, mem::zeroed};

type UObject = ucore::UObject<0x4D>;

"#;

    let mut lib = BufWriter::new(opts.open(folder.join(format!("{}.rs", pkg.ident)))?);
    lib.write_all(PRELUDE.as_bytes())?;

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
            "    // Inherited = {:#X}{}",
            parent.layout.size,
            if let Some(size) = parent.shrink.get() {
                Cow::from(format!(", Shrunk = {size:#X}"))
            } else {
                Cow::from("")
            }
        )?;
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
                let Some(repr) = stringify_type(kind, sdk) else {
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

                let idx = dedup.dedup(name);
                match idx {
                    1 => writeln!(
                        w,
                        "        pub {name}: {repr}, // {offset:#X}({total_size:#X})"
                    )?,
                    _ => writeln!(
                        w,
                        "        pub {name}_{idx}: {repr}, // {offset:#X}({total_size:#X})"
                    )?,
                }

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
                        "        {name}: {offset}..={},",
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

    if !bitfields.is_empty() {
        writeln!(w, "memflex::bitfields! {{")?;
        write!(w, "{bitfields}")?;
        writeln!(w, "}}\n")?;
    }

    if !functions.borrow().is_empty() {
        writeln!(w, "impl {ident} {{")?;
    }

    for func in functions.borrow().iter() {
        let Function {
            ident,
            index,
            args,
            ret,
        } = func;

        let mut separated: String = "".into();
        let mut names: String = "".into();
        let mut fields: String = "".into();

        let mut dedup = NameDedup::default();
        for (name, kind) in args {
            let ty = stringify_type(kind, sdk).unwrap_or(Cow::from("*const ()"));
            let i = dedup.dedup(name);
            let name = match i {
                1 => Cow::from(name),
                _ => Cow::from(format!("{name}_{i}")),
            };
            write!(separated, ", {name}: {ty}",)?;
            write!(names, "{name}, ")?;
            writeln!(fields, "            {name}: {ty},")?;
        }

        write!(w, "    pub unsafe fn {ident}(&self{separated})")?;
        if let Some(ret) = ret {
            let ty = stringify_type(ret, sdk).unwrap_or(Cow::from("*const ()"));
            write!(w, " -> {ty}",)?;
            writeln!(fields, "            out: {ty},")?;
        }

        writeln!(
            w,
            "{{\n        static FUNCTION: SyncLazy<Ptr<ucore::UObject<0x4D>>> = SyncLazy::new(|| ucore::UObject::get_by_index({index}));\n"
        )?;
        writeln!(w, "       #[repr(C)]\n         struct Args {{\n{fields}        }}\n")?;

        writeln!(
            w,
            "        let args = Args {{ {names}{} }};",
            if ret.is_some() { "out: zeroed()" } else { "" }
        )?;
        writeln!(
            w,
            "        self.cast_ref::<ucore::UObject<0x4D>>().process_event(*SyncLazy::force(&FUNCTION), &args);"
        )?;

        if ret.is_some() {
            writeln!(w, "        args.out")?;
        }

        writeln!(w, "    }}\n")?;
    }

    if !functions.borrow().is_empty() {
        writeln!(w, "}}\n")?;
    }

    Ok(())
}

fn stringify_type(kind: &PropertyKind, sdk: &Sdk) -> Option<Cow<'static, str>> {
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
            format!("Option<Ptr<{}>>", object.ptr.ident()).into()
        }
        PropertyKind::Inline(inner) => {
            let object = sdk.lookup(inner).unwrap();
            object.ptr.ident().to_owned().into()
        }
        PropertyKind::Array { kind, size } => {
            format!("[{}; {:#X}]", stringify_type(kind, sdk)?, *size).into()
        }
        PropertyKind::Vec(inner) => format!("TArray<{}>", stringify_type(inner, sdk)?).into(),
        PropertyKind::Set(inner) => format!("TSet<{}>", stringify_type(inner, sdk)?).into(),
        PropertyKind::Map { key, value } => format!(
            "TMap<{}, {}>",
            stringify_type(key, sdk)?,
            stringify_type(value, sdk)?
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
    fn dedup(&mut self, name: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        hasher.write_usize(name.len());
        hasher.write(name.as_bytes());

        let i = self.0.entry(hasher.finish()).or_insert(0);
        *i += 1;

        *i
    }
}
