use anyhow::{Context, Result};
use argh::FromArgs;
use dumper::{codegen::generate_rust_sdk, DumperOptions, Offsets};
use log::{info, warn, LevelFilter};
use petgraph::dot::{Config, Dot};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(FromArgs)]
/// Dumpes unreal engine SDK externally by accessing game memory through WinAPI.
struct Args {
    /// process id of the game
    #[argh(option, short = 'p')]
    pid: u32,
    /// FNamePool offset
    #[argh(option, short = 'N')]
    names: String,
    /// TUObjectArray offset
    #[argh(option, short = 'O')]
    objects: String,
    /// specifies packages to merge together in format `target:consumer`
    #[argh(option, short = 'm')]
    merge: Vec<String>,
    /// specifies fqn to trace, i.e. print detailed data
    #[cfg(debug_assertions)]
    #[argh(option, short = 't')]
    trace: Vec<String>,
    /// do not write generated SDK to the disk
    #[argh(switch, short = 'd')]
    dummy: bool,
    /// do not try to eliminate dependency cycles
    #[argh(switch, short = 'c')]
    allow_cycles: bool,
    /// generate dot file with dependency graph
    #[argh(option, short = 'o')]
    dot: Option<String>,
}

fn parse_hex_arg(arg: &str) -> Result<usize> {
    usize::from_str_radix(arg.strip_prefix("0x").unwrap_or(arg), 16).map_err(Into::into)
}

fn main() -> Result<()> {
    #[cfg(not(debug_assertions))]
    let mut filter = LevelFilter::Info;
    #[cfg(debug_assertions)]
    let mut filter = LevelFilter::Debug;

    let args = argh::from_env::<Args>();
    if !args.trace.is_empty() {
        filter = LevelFilter::Trace;
    }

    env_logger::builder()
        .format_target(false)
        .filter_level(filter)
        .parse_default_env()
        .init();

    if args.dummy {
        warn!("Running with dummy generator selected, no files will be written onto disk!");
        sleep(Duration::from_millis(1000));
    }

    let trace = args
        .trace
        .iter()
        .flat_map(|s| s.split(','))
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();

    let merge = args
        .merge
        .iter()
        .flat_map(|v| v.split(','))
        .map(|v| {
            v.split_once(':')
                .map(|(a, b)| (a.into(), b.into()))
                .context("Invalid merge argument")
        })
        .collect::<Result<HashMap<_, _>>>()?;

    let options = DumperOptions {
        names: parse_hex_arg(&args.names)?,
        objects: parse_hex_arg(&args.objects)?,
        allow_cycles: args.allow_cycles,
        process_id: args.pid,
        merge,
        trace,
    };

    let start = Instant::now();
    let sdk = dumper::run(options, Offsets::DEFAULT)?;
    info!("Dumper finished in {:.2?}", start.elapsed());

    if let Some(mut path) = args.dot {
        if !path.ends_with(".dot") {
            path = format!("{path}.dot")
        }

        let dot = Dot::with_config(&sdk.packages, &[Config::EdgeNoLabel]);
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?
            .write_all(format!("{dot:?}").as_bytes())?;
    }

    if !args.dummy {
        let start = Instant::now();
        generate_rust_sdk("./usdk", &sdk)?;
        info!("Sdk generation finished in {:.2?}", start.elapsed());
    }

    Ok(())
}
