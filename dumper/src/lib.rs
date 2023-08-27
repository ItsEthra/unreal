use anyhow::{anyhow, Context, Result};
use memflex::external::OwnedProcess;
use names::NamePool;
use sdk::Sdk;
use std::{
    collections::HashMap,
    mem::{size_of, zeroed},
    ptr::addr_of_mut,
    slice::from_raw_parts_mut,
    sync::OnceLock,
};

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
        external: Box::new(proc) as _,
        names: OnceLock::new(),
        config: offsets,
        options,
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
    names: OnceLock<NamePool>,
    external: Box<dyn External>,
    options: DumperOptions,
    config: Config,
    base: usize,
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

trait External: Send + Sync + 'static {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()>;
}

impl dyn External {
    fn read<T>(&self, address: usize) -> Result<T> {
        let mut temp: T = unsafe { zeroed() };
        let buf = unsafe { from_raw_parts_mut(addr_of_mut!(temp).cast::<u8>(), size_of::<T>()) };
        self.read_buf(address, buf)?;

        Ok(temp)
    }
}

impl External for OwnedProcess {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        OwnedProcess::read_buf(self, address, buf)
            .map(|_| ())
            .map_err(Into::into)
    }
}
