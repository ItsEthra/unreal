use argh::FromArgs;
use eyre::Result;
use log::info;
use names::GNames;
use objects::GObjects;
use offsets::Offsets;
use package::dump_packages;
use process::{ExternalProcess, Process};
use ptr::Ptr;
use sourcer::{lang::RustSdkGenerator, ClassRegistry, SdkGenerator};
use std::{cell::RefCell, fs, io::Write, ops::Deref, rc::Rc};

mod macros;
mod names;
mod objects;
mod offsets;
mod package;
mod process;
mod ptr;
mod utils;

const OFFSETS: &Offsets = &offsets::DEFAULT;

pub struct GlobalProxy<T>(Option<T>);
impl<T> Deref for GlobalProxy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("Cell was not set")
    }
}

pub struct Info {
    process: Box<dyn Process>,
    names: GlobalProxy<GNames>,
    objects: GlobalProxy<GObjects>,

    names_dump: RefCell<Box<dyn Write>>,
    objects_dump: RefCell<Box<dyn Write>>,
}

#[derive(FromArgs)]
/// UE 4.27+ Dumper by @ItsEthra.
struct Args {
    #[argh(positional)]
    /// process id.
    pid: u32,

    #[argh(option, short = 'N')]
    /// address of GNames, in hex.
    names: Option<String>,

    #[argh(option, short = 'O')]
    /// address of GObjects, in hex.
    objects: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let args: Args = argh::from_env();

    let names_dump = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("NamesDump.txt")?;
    let objects_dump = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("ObjectsDump.txt")?;

    let mut info = Info {
        process: Box::new(ExternalProcess::new(args.pid)?),
        names: GlobalProxy(None),
        objects: GlobalProxy(None),

        names_dump: (Box::new(names_dump) as Box<dyn Write>).into(),
        objects_dump: (Box::new(objects_dump) as Box<dyn Write>).into(),
    };

    let map_addr_arg = |s: String| {
        let hex = s.strip_prefix("0x").unwrap_or(&s);
        usize::from_str_radix(hex, 16).expect("Invalid hex value")
    };

    let names_ptr = Ptr(args
        .names
        .map(map_addr_arg)
        .expect("GNames is required so far"));
    let objects_ptr = Ptr(args
        .objects
        .map(map_addr_arg)
        .expect("GObjects is required so far"))
        + 0x10;

    let gnames = names::dump_names(&info, names_ptr)?;
    info.names.0 = Some(gnames);

    let gobjects = objects::dump_objects(&info, objects_ptr)?;
    info.objects.0 = Some(gobjects);

    let mut sdkgen = RustSdkGenerator::new(".")?;

    let (packages, registry) = {
        let mut rg = ClassRegistry::default();
        let pkgs = dump_packages(&info, &mut rg)?;
        info!("Registry entries: {}", rg.len());

        (pkgs, Rc::new(rg))
    };

    for package in packages.iter() {
        let pkg_cg = sdkgen.begin_package(&package.name, &registry)?;

        package.process(&info, pkg_cg)?;
    }

    Ok(())
}
