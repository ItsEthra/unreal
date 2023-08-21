use anyhow::{anyhow, Context, Result};
use memflex::{
    external::{open_process_by_id, OwnedProcess},
    types::win::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
};
use names::NamePool;
use sdk::Sdk;
use std::{collections::HashMap, sync::OnceLock};

pub mod codegen {
    mod rust;
    pub use rust::*;
}

mod engine;
mod names;
mod objects;
mod process;
mod sdk;
mod utils;

mod offsets;
pub use offsets::Offsets;

pub struct DumperOptions {
    pub process_id: u32,
    pub names: usize,
    pub objects: usize,
    /// Options to merge two packages together to avoid cyclic dependencies
    pub merge: HashMap<String, String>,
    /// FQNs to trace
    pub trace: Vec<String>,
    pub allow_cycles: bool,
}

pub fn run(options: DumperOptions, offsets: Offsets) -> Result<Sdk> {
    let proc = open_process_by_id(
        options.process_id,
        false,
        PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
    )?;

    let module = proc
        .modules()?
        .find(|m| m.name.ends_with("exe"))
        .ok_or(anyhow!("Failed to find process executable image"))?;

    let state = State {
        base: module.base as usize,
        names: OnceLock::new(),
        offsets,
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
    offsets: Offsets,
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