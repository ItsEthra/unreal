use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use log::{info, warn, LevelFilter};
use memflex::external::OwnedProcess;
use petgraph::dot;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    thread::sleep,
    time::{Duration, Instant},
};
use uedumper::{
    codegen::{Codegen, RustCodegen, RustOptions},
    Config, DumperOptions, External,
};

/// Dumpes unreal engine SDK externally by accessing game memory through WinAPI.
#[derive(Parser)]
#[clap(version, author)]
struct Args {
    /// process id of the game
    #[clap(short = 'p', long = "process-id")]
    pid: Option<u32>,

    /// FNamePool offset
    #[clap(short = 'N', long)]
    names: Option<String>,

    /// TUObjectArray offset
    #[clap(short = 'O', long)]
    objects: Option<String>,

    /// specifies packages to merge together in format `target:consumer`
    #[clap(short = 'M', long)]
    merge: Vec<String>,

    /// do not write generated SDK to the disk
    #[clap(short = 'd', long = "dry-run")]
    dry: bool,

    /// use glam structures instead of auto-generated
    #[clap(short = 'm', long)]
    glam: bool,

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

fn get_process_id(arg_id: Option<u32>) -> Result<u32> {
    #[cfg(windows)]
    let find = || {
        let window = memflex::external::find_window("UnrealWindow".into(), None)?;
        let (pid, _) = memflex::external::find_window_process_thread(window)?;
        Result::Ok(pid)
    };

    #[cfg(unix)]
    let find = || {
        Err(anyhow!(
            "Window lookup is not supported on unix, you must specify process id"
        ))
    };

    if let Some(id) = arg_id {
        Ok(id)
    } else {
        find()
    }
}

fn get_offset(from_cfg: Option<usize>, from_arg: &Option<String>, name: &str) -> Result<usize> {
    if let Some(offset) = from_cfg {
        Ok(offset)
    } else if let Some(arg) = from_arg {
        Ok(parse_hex_arg(arg)?)
    } else {
        bail!("Missing {name} offset")
    }
}

fn main() -> Result<()> {
    #[cfg(not(debug_assertions))]
    let filter = LevelFilter::Info;
    #[cfg(debug_assertions)]
    let filter = LevelFilter::Debug;

    let args = Args::parse();
    env_logger::builder()
        .format_target(false)
        .format_timestamp(None)
        .filter_level(filter)
        .parse_default_env()
        .init();

    if args.dry {
        warn!("Performing a dry run, no SDK will be written to the disk");
        sleep(Duration::from_millis(1000));
    }

    let config = fetch_offsets(&args.config)?;
    let options = DumperOptions {
        objects: get_offset(
            config.offsets.as_ref().and_then(|o| o.objects),
            &args.objects,
            "TUObjectArray",
        )?,
        names: get_offset(
            config.offsets.as_ref().and_then(|o| o.names),
            &args.names,
            "FNamePool",
        )?,
        merge: parse_merge_args(&args.merge)?,
        allow_cycles: args.allow_cycles,
        process_id: get_process_id(args.pid)?,
    };

    #[cfg(windows)]
    use memflex::types::win::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    #[cfg(windows)]
    let proc = memflex::external::open_process_by_id(
        options.process_id,
        false,
        PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
    )?;

    #[cfg(unix)]
    let proc = memflex::external::find_process_by_id(options.process_id)?;

    let module = proc
        .modules()?
        .find(|m| m.name.ends_with("exe"))
        .ok_or(anyhow!("Failed to find process executable image"))?;

    let start = Instant::now();
    let sdk = uedumper::run(options, config, Box::new(Wrapper(proc)), module.base as _)?;
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

        let options = RustOptions {
            path: args.output.unwrap_or("usdk".into()).into(),
            glam: args.glam,
        };
        let codegen = RustCodegen::new(&sdk, &options)?;
        codegen.generate()?;

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
    merge
        .iter()
        .flat_map(|v| v.split(','))
        .map(|v| {
            v.split_once(':')
                .map(|(a, b)| (a.into(), b.into()))
                .context("Invalid merge argument")
        })
        .collect::<Result<HashMap<_, _>>>()
}

struct Wrapper(OwnedProcess);
impl External for Wrapper {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        OwnedProcess::read_buf(&self.0, address, buf)
            .map(|_| ())
            .map_err(Into::into)
    }
}
