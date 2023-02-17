use argh::FromArgs;
use offsets::Offsets;
use process::{ExternalProcess, Process};
use ptr::Ptr;
use std::{cell::RefCell, fs, io::Write};

mod names;
mod objects;
mod offsets;
mod process;
mod ptr;

pub struct Info {
    process: Box<dyn Process>,
    offsets: &'static Offsets,

    names_dump: RefCell<Box<dyn Write>>,
    objects_dump: RefCell<Box<dyn Write>>,
}

#[derive(FromArgs)]
#[allow(dead_code)]
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

fn main() -> eyre::Result<()> {
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

    let config = Info {
        process: Box::new(ExternalProcess::new(args.pid)?),
        offsets: &offsets::DEFAULT,

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

    let gnames = names::dump_names(&config, names_ptr)?;
    objects::dump_objects(&config, &gnames, objects_ptr)?;

    Ok(())
}
