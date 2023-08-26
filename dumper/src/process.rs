use crate::{
    engine::{
        FBoolProperty, FFieldPtr, FPropertyPtr, UClassPtr, UEnumPtr, UFunctionPtr, UObjectPtr,
        UStructPtr,
    },
    sdk::{
        Enum, Field, FieldOptions, Function, FunctionArg, Object, Package, PropertyKind, Sdk,
        Struct,
    },
    utils::{sanitize_ident, strip_package_name, AccumulatorResult, BitfieldAccumulator, Layout},
    State,
};
use anyhow::{bail, Result};
use log::{info, warn};
use memflex::sizeof;
use petgraph::{
    graph::NodeIndex,
    stable_graph::StableGraph,
    Directed,
    Direction::{Incoming, Outgoing},
};
use std::{
    collections::{HashMap, HashSet},
    iter::successors,
    ops::RangeInclusive,
};
use ucore::{fqn, Fqn};

pub(crate) fn process(objects: &[UObjectPtr]) -> Result<Sdk> {
    let mut sdk = Sdk::default();
    let mut foreign_map: HashMap<NodeIndex, HashSet<Fqn>> = HashMap::new();

    for object in objects {
        let Some(outer) = get_outermost_object(*object)? else {
            continue;
        };

        assert!(!outer.is_null() && outer.outer()?.is_null());
        let outer_name = sanitize_ident(strip_package_name(outer.name().get()?));

        let object = if object.is_a(fqn!(CoreUObject.Enum))? {
            Object::Enum(index_enum(object.cast())?)
        } else if object.is_a(fqn!(CoreUObject.ScriptStruct))?
            || object.is_a(fqn!(CoreUObject.Class))?
        {
            let key = sdk.retrieve_key(&outer_name);
            let foreign = foreign_map.entry(key).or_default();
            Object::Struct(index_struct(object.cast(), foreign)?)
        } else {
            continue;
        };
        sdk.add(&outer_name, object);
    }
    info!("Found {} packages", sdk.packages.node_count());

    let mut num_funcs = 0;
    for object in objects {
        let Some(outer) = get_outermost_object(*object)? else {
            continue;
        };

        if object.is_a(fqn!(CoreUObject.Function))? {
            assert!(!outer.is_null() && outer.outer()?.is_null());
            let outer_name = sanitize_ident(strip_package_name(outer.name().get()?));

            let key = sdk.retrieve_key(&outer_name);
            let foreign = foreign_map.entry(key).or_default();
            let Ok(target) = object.outer()?.fqn() else {
                continue;
            };

            let function = index_function(*object, foreign)?;
            let (Object::Class(target) | Object::Struct(target)) =
                &*sdk.owned.get(&target).unwrap().ptr
            else {
                // Functions will be only in classes or structs.
                unreachable!()
            };

            target.functions.borrow_mut().push(function);
            num_funcs += 1;
        }
    }
    info!("Found {num_funcs} functions");

    populate_dependency_map(&mut sdk, foreign_map);
    shrink_base_classes(&sdk);

    if !State::get().options.allow_cycles {
        eliminate_dependency_cycles(&mut sdk);
    }

    Ok(sdk)
}

fn eliminate_dependency_cycles(sdk: &mut Sdk) {
    type G = StableGraph<Package, (), Directed>;
    type NI = NodeIndex;

    #[rustfmt::skip]
    fn format_cycle(chain: &[NI], g: &G) -> String {
        use std::fmt::Write;

        let mut s: String = "".into();
        for (i, link) in chain.iter().enumerate() {
            _ = match true {
                _ if i != chain.len() - 1 => write!(s, "{} -> ", g.node_weight(*link).unwrap().ident),
                _ => write!(s, "{}", g.node_weight(*link).unwrap().ident),
            };
        }

        s
    }

    fn inner(current: NI, g: &G, mut chain: Vec<NI>) -> Option<Vec<NI>> {
        let mut out = None;
        for neighbor in g.neighbors(current) {
            if let Some(i) = chain.iter().position(|n| *n == neighbor) {
                chain.push(neighbor);
                return Some(chain.split_off(i));
            }
        }

        for neighbor in g.neighbors(current) {
            let mut copy = chain.clone();
            copy.push(neighbor);
            out = out.or(inner(neighbor, g, copy));
        }

        out
    }

    loop {
        let cycle = |i: NI| inner(i, &sdk.packages, vec![i]);
        let Some(cycle) = sdk.packages.node_indices().find_map(cycle) else {
            break;
        };

        warn!(
            "Eliminating dependency cycle {}",
            format_cycle(&cycle, &sdk.packages)
        );

        let consumer = cycle[0];
        for idx in (cycle[1..cycle.len() - 1]).iter().rev() {
            for dependant in sdk
                .packages
                .neighbors_directed(*idx, Incoming)
                .collect::<Vec<_>>()
            {
                if dependant != consumer {
                    sdk.packages.update_edge(dependant, consumer, ());
                }

                let old = sdk.packages.find_edge(dependant, *idx).unwrap();
                sdk.packages.remove_edge(old);
            }

            for dependency in sdk
                .packages
                .neighbors_directed(*idx, Outgoing)
                .collect::<Vec<_>>()
            {
                if dependency != consumer {
                    sdk.packages.update_edge(consumer, dependency, ());
                }

                let old = sdk.packages.find_edge(*idx, dependency).unwrap();
                sdk.packages.remove_edge(old);
            }

            let Package { objects, .. } = sdk.packages.remove_node(*idx).unwrap();
            for object in &objects {
                sdk.owned.get_mut(&object.fqn()).unwrap().package = cycle[0];
            }

            let consumer = sdk.packages.node_weight_mut(cycle[0]).unwrap();
            consumer.objects.extend(objects);
        }
    }
}

#[rustfmt::skip]
fn shrink_base_classes(sdk: &Sdk) {
    for pkg in sdk.packages.node_weights() {
        for obj in pkg.objects.iter() {
            let (Object::Class(obj) | Object::Struct(obj)) = &**obj else { continue; };
            let Some(first_field_size) = obj.fields.first().map(|f| f.offset()) else { continue; };
            let Some(parent_fqn) = obj.parent else { continue; };
            let (Object::Class(parent) | Object::Struct(parent)) = &*sdk.lookup(&parent_fqn).unwrap().ptr else { unreachable!() };

            if first_field_size < parent.layout.size {
                let new = parent.shrink.get().unwrap_or(usize::MAX).min(first_field_size);
                parent.shrink.set(Some(new));
            }
        }
    }
}

fn populate_dependency_map(sdk: &mut Sdk, foreign_map: HashMap<NodeIndex, HashSet<Fqn>>) {
    for (pkg_idx, foreign) in foreign_map.into_iter() {
        let Some(pkg) = sdk.packages.node_weight(pkg_idx) else {
            continue;
        };
        let own = pkg.objects.iter().map(|o| o.fqn()).collect::<HashSet<_>>();

        for dep in foreign
            .difference(&own)
            .map(|i| sdk.owned.get(i).unwrap().package)
        {
            sdk.packages.update_edge(pkg_idx, dep, ());
        }
    }
}

fn get_outermost_object(object: UObjectPtr) -> Result<Option<UObjectPtr>> {
    Ok(successors(object.outer()?.non_null(), |obj| {
        obj.outer().unwrap().non_null()
    })
    .last())
}

fn index_function(object: UObjectPtr, foreign: &mut HashSet<Fqn>) -> Result<Function> {
    let ident = sanitize_ident(object.name().get()?).into_owned();
    let index = object.index()?;

    let flags = object.cast::<UFunctionPtr>().flags()?;

    let mut args = vec![];

    let ptr = object.cast::<UStructPtr>();
    for arg in successors(ptr.children_props()?.non_null(), |field| {
        field.next().unwrap().non_null()
    }) {
        let property = arg.cast::<FPropertyPtr>();

        let name = sanitize_ident(arg.name().get()?).into_owned();
        let kind = get_property_kind(arg, foreign)?;
        let flags = property.flags()?;

        let arg = FunctionArg { name, kind, flags };
        args.push(arg);
    }

    let function = Function {
        fqn: object.fqn()?,
        ident,
        index,
        flags,
        args,
    };
    Ok(function)
}

fn index_enum(uenum_ptr: UEnumPtr) -> Result<Enum> {
    let fqn = uenum_ptr.cast::<UObjectPtr>().fqn()?;

    let variants = uenum_ptr
        .names()?
        .iter::<(u64, i64)>()
        .flatten()
        .map(|(n, v)| {
            let name = State::get().get_name(n as u32)?;
            let ident =
                sanitize_ident(name.split_once("::").map(|v| v.1).unwrap_or(name)).into_owned();
            Result::Ok((ident, v))
        })
        .filter(|v| !matches!(v, Ok((n, _)) if n.ends_with("_MAX")))
        .collect::<Result<Vec<_>>>()?;

    let size = pick_enum_size(variants.iter().map(|v| v.1));
    let layout = Layout::same(size);

    Ok(Enum {
        ident: sanitize_ident(fqn.name()).into_owned(),
        variants,
        layout,
        fqn,
    })
}

fn pick_enum_size(range: impl Iterator<Item = i64>) -> usize {
    let (mut min, mut max) = (0, 0);
    for v in range {
        min = v.min(min);
        max = v.max(max);
    }

    let suits = |v: RangeInclusive<i64>| v.contains(&min) && v.contains(&max);

    if suits(i8::MIN as i64..=u8::MAX as i64) {
        1
    } else if suits(i16::MIN as i64..=u16::MAX as i64) {
        2
    } else if suits(i32::MIN as i64..=u32::MAX as i64) {
        4
    } else {
        8
    }
}

fn select_prefix(ustruct: UStructPtr) -> Result<char> {
    let child_of = |fqn: Fqn| {
        successors(ustruct.non_null(), |s| s.super_struct().unwrap().non_null())
            .any(|s| s.cast::<UObjectPtr>().fqn().unwrap() == fqn)
    };

    let prefix = if child_of(fqn!(Engine.Actor)) {
        'A'
    } else if child_of(fqn!(CoreUObject.Object)) {
        'U'
    } else {
        'F'
    };

    Ok(prefix)
}

fn index_struct(ustruct_ptr: UStructPtr, foreign: &mut HashSet<Fqn>) -> Result<Struct> {
    let config = &State::get().config;
    let fqn = ustruct_ptr.cast::<UObjectPtr>().fqn()?;

    let prefix = select_prefix(ustruct_ptr.cast())?;
    let ident = format!("{prefix}{}", sanitize_ident(fqn.name()));
    let index = ustruct_ptr.cast::<UObjectPtr>().index()?;

    let size = ustruct_ptr.props_size()? as usize;
    let align = ustruct_ptr.min_align()? as usize;

    let parent = ustruct_ptr
        .super_struct()?
        .non_null()
        .map(|s| s.cast::<UObjectPtr>().fqn())
        .transpose()?;
    if let Some(fqn) = parent {
        foreign.insert(fqn);
    }

    let mut ustruct = Struct {
        is_uobject: !ustruct_ptr
            .cast::<UObjectPtr>()
            .is_a(fqn!(CoreUObject.ScriptStruct))?,
        layout: Layout { size, align },
        functions: vec![].into(),
        shrink: None.into(),
        fields: vec![],
        parent,
        ident,
        index,
        fqn,
    };
    let mut accumulator = BitfieldAccumulator::default();

    if let Some(offset) = (fqn == fqn!(Engine.Level))
        .then_some(config.level_actors)
        .flatten()
    {
        ustruct.fields.push(Field::Property {
            name: "Actors".into(),
            kind: PropertyKind::Vec(PropertyKind::Ptr(fqn!(Engine.Actor)).into()),
            options: FieldOptions {
                offset: offset as usize,
                elem_size: 0x10,
                array_dim: 1,
            },
        })
    }

    for field in successors(ustruct_ptr.children_props()?.non_null(), |field| {
        field.next().unwrap().non_null()
    }) {
        let fproperty = field.cast::<FPropertyPtr>();

        let name = sanitize_ident(field.name().get()?).into_owned();
        let kind = get_property_kind(field, foreign)?;
        let offset = fproperty.offset()? as usize;
        let array_dim = fproperty.array_dim()? as usize;
        let elem_size = fproperty.element_size()? as usize;

        if matches!(kind, PropertyKind::Bool) {
            let vars = fproperty.cast::<FBoolProperty>().vars()?;
            assert_eq!(vars.byte_offset, 0);
            assert_eq!(vars.field_size, 1);
        }

        let acc_result = accumulator.accumulate(&name, fproperty, &kind, offset)?;
        match acc_result {
            AccumulatorResult::Skip => continue,
            AccumulatorResult::Append(groups) => {
                for group in groups {
                    ustruct.fields.push(Field::Bitfields(group))
                }
            }
        };

        let field = Field::Property {
            name,
            kind,
            options: FieldOptions {
                offset,
                elem_size,
                array_dim,
            },
        };
        ustruct.fields.push(field);
    }

    ustruct.fields.sort_by_key(|f| f.offset());

    Ok(ustruct)
}

fn get_property_kind(field: FFieldPtr, foreign: &mut HashSet<Fqn>) -> Result<PropertyKind> {
    let State {
        proc,
        config: offsets,
        ..
    } = State::get();

    let property = field.cast::<FPropertyPtr>();
    let classname = field.class()?.name().get()?;
    let kind = match classname {
        "BoolProperty" => PropertyKind::Bool,
        "NameProperty" => PropertyKind::Name,
        "StrProperty" => PropertyKind::String,
        "TextProperty" => PropertyKind::Text,

        "FloatProperty" => PropertyKind::Float32,
        "DoubleProperty" => PropertyKind::Float64,

        "Int8Property" => PropertyKind::Int8,
        "Int16Property" => PropertyKind::Int16,
        "IntProperty" => PropertyKind::Int32,
        "Int64Property" => PropertyKind::Int64,

        "ByteProperty" => PropertyKind::UInt8,
        "UInt16Property" => PropertyKind::UInt16,
        "UInt32Property" => PropertyKind::UInt32,
        "UInt64Property" => PropertyKind::UInt64,

        "ClassProperty" | "ObjectProperty" => {
            let uclass = proc.read::<UClassPtr>(field.0 + offsets.fproperty.size)?;
            let fqn = uclass.cast::<UObjectPtr>().fqn()?;
            foreign.insert(fqn);

            PropertyKind::Ptr(fqn)
        }
        "StructProperty" => {
            let ustruct = proc.read::<UStructPtr>(field.0 + offsets.fproperty.size)?;
            let fqn = ustruct.cast::<UObjectPtr>().fqn()?;
            foreign.insert(fqn);

            PropertyKind::Inline(fqn)
        }
        "EnumProperty" => {
            let uenum = proc.read::<UEnumPtr>(field.0 + offsets.fproperty.size + sizeof!(usize))?;
            let fqn = uenum.cast::<UObjectPtr>().fqn()?;
            foreign.insert(fqn);

            PropertyKind::Inline(fqn)
        }
        "ArrayProperty" => {
            let inner = proc.read::<FPropertyPtr>(field.0 + offsets.fproperty.size)?;
            PropertyKind::Vec(get_property_kind(inner.cast(), foreign)?.into())
        }
        "SetProperty" => {
            let inner = proc.read::<FPropertyPtr>(field.0 + offsets.fproperty.size)?;
            PropertyKind::Set(get_property_kind(inner.cast(), foreign)?.into())
        }
        "MapProperty" => {
            let key = proc.read::<FPropertyPtr>(field.0 + offsets.fproperty.size)?;
            let value =
                proc.read::<FPropertyPtr>(field.0 + offsets.fproperty.size + sizeof!(usize))?;

            PropertyKind::Map {
                key: get_property_kind(key.cast(), foreign)?.into(),
                value: get_property_kind(value.cast(), foreign)?.into(),
            }
        }
        "ClassPtrProperty"
        | "DelegateProperty"
        | "FieldPathProperty"
        | "InterfaceProperty"
        | "LazyObjectProperty"
        | "SoftClassProperty"
        | "SoftObjectProperty"
        | "WeakObjectProperty"
        | "MulticastInlineDelegateProperty"
        | "MulticastSparseDelegateProperty" => PropertyKind::Unknown,
        other => bail!("Unrecogninzed property classname {other}"),
    };

    let array_dim = property.array_dim()? as usize;

    if array_dim != 1 {
        assert!(!matches!(kind, PropertyKind::Unknown));
        Ok(PropertyKind::Array {
            kind: kind.into(),
            size: array_dim,
        })
    } else {
        Ok(kind)
    }
}
