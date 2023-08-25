use anyhow::{Context, Result};
use clap::Parser;
use dumper::{codegen::generate_rust_sdk, Config, DumperOptions};
use log::{info, warn, LevelFilter};
use petgraph::dot;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    thread::sleep,
    time::{Duration, Instant},
};

/// Dumpes unreal engine SDK externally by accessing game memory through WinAPI.
#[derive(Parser)]
#[clap(version, author)]
struct Args {
    /// process id of the game
    #[clap(short = 'p', long = "process-id")]
    pid: u32,
    /// FNamePool offset
    #[clap(short = 'N', long)]
    names: String,
    /// TUObjectArray offset
    #[clap(short = 'O', long)]
    objects: String,
    /// specifies packages to merge together in format `target:consumer`
    #[clap(short = 'm', long)]
    merge: Vec<String>,
    /// do not write generated SDK to the disk
    #[clap(short = 'd', long = "dry-run")]
    dry: bool,
    /// do not try to eliminate dependency cycles
    #[clap(short = 'b', long)]
    allow_cycles: bool,
    /// generate dot file with dependency graph
    #[clap(short = 'g', long)]
    dot: Option<String>,
    /// output folder for the generated SDK
    #[clap(short = 'o', long)]
    output: Option<String>,
    /// config file path
    #[clap(short = 'c', long)]
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

    let args = Args::parse();
    env_logger::builder()
        .format_target(false)
        .filter_level(filter)
        .parse_default_env()
        .init();

    if args.dry {
        warn!("Performing a dry run, no SDK will be written to the disk");
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

        let dot = dot::Dot::with_config(&sdk.packages, &[dot::Config::EdgeNoLabel]);
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?
            .write_all(format!("{dot:?}").as_bytes())?;
        info!("Saved dependency graph file as {path}");
    }

    if !args.dry {
        let start = Instant::now();
        generate_rust_sdk(args.output.unwrap_or("usdk".into()), &sdk)?;
        info!("Sdk generation finished in {:.2?}", start.elapsed());
    }

    Ok(())
}

fn fetch_offsets(config: &Option<String>) -> Result<Config> {
    if let Some(path) = config {
        let text = fs::read_to_string(path)?;
        let config = toml::from_str(&text)?;
        info!("Loaded config file from {path}");

        Ok(config)
    } else {
        Ok(Config::default())
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
