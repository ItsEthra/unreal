use crate::package::merge;
use argh::FromArgs;
use eyre::Result;
use log::{info, warn};
use names::GNames;
use objects::GObjects;
use offsets::Offsets;
use package::dump_packages;
use process::{ExternalProcess, Process};
use ptr::Ptr;
use sourcer::{
    lang::{DummySdkGenerator, RustSdkGenerator},
    PackageRegistry, SdkGenerator,
};
use std::{
    cell::RefCell,
    env, fs,
    io::{self, Write},
    ops::Deref,
    rc::Rc,
    time::Instant,
};

mod macros;
mod names;
mod objects;
mod package;
mod process;
mod ptr;
mod utils;

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
    offsets: &'static Offsets,

    names_dump: RefCell<Box<dyn Write>>,
    objects_dump: RefCell<Box<dyn Write>>,
}

#[derive(FromArgs)]
/// UE 4.25+ Dumper by @ItsEthra.
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

    #[argh(switch)]
    /// enable dummy mode, prevents generator from writing sdk to disk.
    dummy: bool,

    #[argh(option, short = 'M')]
    /// string of format `consumer:target`. Instructs dumper to merge `target` package into `consumer`.
    merge: Vec<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::builder().format_timestamp(None).init();

    #[cfg(unix)]
    let user_id = {
        env::var("SUDO_UID")
            .ok()
            .and_then(|uid| uid.parse::<u32>().ok())
            .unwrap_or_else(|| unsafe { libc::getuid() })
    };

    let args: Args = argh::from_env();
    let merges: Vec<(&str, &str)> = args
        .merge
        .iter()
        .flat_map(|s| s.split(','))
        .map(|s| s.split_once(':').expect("Invalid merge string"))
        .collect();

    let create_sink = |file_name: &str| -> Result<RefCell<Box<dyn Write>>> {
        let sink = if args.dummy {
            Box::new(io::sink()) as Box<dyn Write>
        } else {
            let file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(file_name)?;
            Box::new(file) as Box<dyn Write>
        }
        .into();

        Ok(sink)
    };

    let mut info = Info {
        process: Box::new(ExternalProcess::new(args.pid)?),
        names: GlobalProxy(None),
        objects: GlobalProxy(None),
        offsets: &offsets::DEFAULT,

        names_dump: create_sink("NamesDump.txt")?,
        objects_dump: create_sink("ObjectsDump.txt")?,
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

    let start = Instant::now();

    info!("Dumping names");
    let gnames = names::dump_names(&info, names_ptr)?;
    info.names.0 = Some(gnames);

    info!("Dumping objects");
    let gobjects = objects::dump_objects(&info, objects_ptr)?;
    info.objects.0 = Some(gobjects);

    let mut sdkgen = if args.dummy {
        warn!("Running in dummy mode, no files to disc will be written.");
        Box::new(DummySdkGenerator::new(".", info.offsets)?) as Box<dyn SdkGenerator>
    } else {
        Box::new(RustSdkGenerator::new(".", info.offsets)?) as Box<dyn SdkGenerator>
    };

    let (packages, registry) = {
        let mut rg = PackageRegistry::default();
        let mut pkgs = dump_packages(&info, &mut rg)?;
        info!("Registry entries: {}", rg.len());

        for (consumer, target) in &merges {
            merge(*consumer, *target, &mut rg, &mut pkgs)?;
        }

        (pkgs, Rc::new(rg))
    };

    for package in packages.iter() {
        let mut pkg_cg = sdkgen.begin_package(&package.name, &registry)?;

        package.process(&info, &mut *pkg_cg)?;

        pkg_cg.end()?;
    }

    sdkgen.end()?;

    info!("Finished in {:?}", start.elapsed());

    #[cfg(unix)]
    if !args.dummy {
        info!("Fixing permissions");
        const TROUBLESOME_PATHS: &[&[u8]] = &[b"usdk\0", b"NamesDump.txt\0", b"ObjectsDump.txt\0"];

        TROUBLESOME_PATHS.iter().for_each(|p| {
            unsafe { libc::chown(p.as_ptr() as _, user_id, user_id) };
        });
    }

    Ok(())
}
