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
    /// do not write generated SDK to the disk
    #[argh(switch, short = 'd')]
    dummy: bool,
    /// do not try to eliminate dependency cycles
    #[argh(switch, short = 'b')]
    allow_cycles: bool,
    /// generate dot file with dependency graph
    #[argh(option, short = 'g')]
    dot: Option<String>,
    /// output folder for the generated SDK
    #[argh(option, short = 'o')]
    output: Option<String>,
    /// config file path
    #[argh(option, short = 'c')]
    config: Option<String>,
}

fn parse_hex_arg(arg: &str) -> Result<usize> {
    usize::from_str_radix(arg.strip_prefix("0x").unwrap_or(arg), 16).map_err(Into::into)
}

fn main() -> Result<()> {
    #[cfg(not(debug_assertions))]
    let filter = LevelFilter::Info;
    #[cfg(debug_assertions)]
    let filter = LevelFilter::Debug;

    let args = argh::from_env::<Args>();
    env_logger::builder()
        .format_target(false)
        .filter_level(filter)
        .parse_default_env()
        .init();

    if args.dummy {
        warn!("Running with dummy generator selected, no files will be written onto disk!");
        sleep(Duration::from_millis(1000));
    }

    let options = DumperOptions {
        objects: parse_hex_arg(&args.objects)?,
        names: parse_hex_arg(&args.names)?,
        merge: parse_merge_args(&args.merge)?,
        allow_cycles: args.allow_cycles,
        process_id: args.pid,
    };

    let offsets = fetch_offsets(&args.config)?;

    let start = Instant::now();
    let sdk = dumper::run(options, offsets)?;
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
        info!("Saved dependency graph file as {path}");
    }

    if !args.dummy {
        let start = Instant::now();
        generate_rust_sdk(args.output.unwrap_or("usdk".into()), &sdk)?;
        info!("Sdk generation finished in {:.2?}", start.elapsed());
    }

    Ok(())
}

fn fetch_offsets(config: &Option<String>) -> Result<Offsets> {
    if let Some(path) = config {
        let text = fs::read_to_string(path)?;
        let config = toml::from_str(&text)?;
        info!("Loaded config file from {path}");

        Ok(config)
    } else {
        Ok(Offsets::default())
    }
}

fn parse_merge_args(merge: &[String]) -> Result<HashMap<String, String>> {
    Ok(merge
        .iter()
        .flat_map(|v| v.split(','))
        .map(|v| {
            v.split_once(':')
                .map(|(a, b)| (a.into(), b.into()))
                .context("Invalid merge argument")
        })
        .collect::<Result<HashMap<_, _>>>()?)
}
