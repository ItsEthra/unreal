use anyhow::{anyhow, Context, Result};
use memflex::external::OwnedProcess;
use names::NamePool;
use sdk::Sdk;
use std::{collections::HashMap, sync::OnceLock};

pub mod codegen {
    mod rust;
    pub use rust::*;
}

pub(crate) mod cycles;
mod engine;
mod names;
mod objects;
mod process;
mod sdk;
mod utils;

mod config;
pub use config::Config;

pub struct DumperOptions {
    pub process_id: u32,
    pub names: usize,
    pub objects: usize,
    /// Options to merge two packages together to avoid cyclic dependencies
    pub merge: HashMap<String, String>,
    pub allow_cycles: bool,
}

pub fn run(options: DumperOptions, offsets: Config) -> Result<Sdk> {
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

    let state = State {
        base: module.base as usize,
        names: OnceLock::new(),
        config: offsets,
        options,
        proc,
    };
    _ = STATE.set(state);

    let names = names::dump_names()?;
    _ = State::get().names.set(names);

    let objects = objects::dump_objects()?;
    let sdk = process::process(&objects)?;
    Ok(sdk)
}

static STATE: OnceLock<State> = OnceLock::new();

struct State {
    options: DumperOptions,
    proc: OwnedProcess,
    config: Config,
    base: usize,
    names: OnceLock<NamePool>,
}

impl State {
    fn get_name(&self, id: u32) -> Result<&str> {
        self.names
            .get()
            .unwrap()
            .get(id)
            .context("Name was not found")
    }

    fn get() -> &'static Self {
        STATE.get().unwrap()
    }
}
