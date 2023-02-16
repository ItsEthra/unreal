#![allow(dead_code)]

use argh::FromArgs;
use offsets::Offsets;
use process::{ExternalProcess, Process, Ptr};

mod names;
mod objects;
mod offsets;
mod process;

pub struct Info {
    process: Box<dyn Process>,
    offsets: &'static Offsets,
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

    let config = Info {
        process: Box::new(ExternalProcess::new(args.pid)?),
        offsets: &offsets::DEFAULT,
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
        .expect("GObjects is required so far"));

    names::dump_names(&config, names_ptr)?;
    objects::dump_objects(&config, objects_ptr)?;

    Ok(())
}
