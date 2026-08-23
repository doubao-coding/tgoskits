use std::{
    env, fs,
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

use quote::quote;

const BUILD_INFO_NAME: &str = "build_info.rs";
const DEFAULT_CPU_CAPACITY: usize = 16;
const DEFAULT_TASK_STACK_SIZE: usize = 0x40000;

fn main() -> Result<()> {
    println!("cargo:rerun-if-env-changed=SMP");
    println!("cargo:rerun-if-env-changed=RT_CPUMASK");

    if cfg!(feature = "host-test") {
        let linker = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host-test.ld");
        println!("cargo:rerun-if-changed={}", linker.display());
        // This crate keeps its scheduler tests in the library target rather
        // than a standalone integration-test target.
        println!("cargo:rustc-link-arg=-T{}", linker.display());
    }

    let config = TaskConfig::load()?;
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join(BUILD_INFO_NAME), build_info_source(config))
}

fn build_info_source(config: TaskConfig) -> String {
    let cpu_capacity = config.cpu_capacity;
    let task_stack_size = config.task_stack_size;
    let rt_indices = config.rt_cpumask.iter().map(|index| quote!(#index));

    quote! {
        pub const CPU_CAPACITY: usize = #cpu_capacity;
        pub const DEFAULT_TASK_STACK_SIZE: usize = #task_stack_size;
        /// CPU indices reserved for realtime tasks under `sched-rt-fifo`. Empty
        /// means no isolation: all CPUs run all tasks. When non-empty, default
        /// tasks avoid these CPUs and `rt_cpu_mask()` selects exactly them.
        pub const RT_CPUMASK: &[usize] = &[#(#rt_indices),*];
    }
    .to_string()
}

#[derive(Clone, Copy)]
struct TaskConfig {
    cpu_capacity: usize,
    task_stack_size: usize,
    rt_cpumask: &'static [usize],
}

impl TaskConfig {
    fn load() -> Result<Self> {
        let mut config = Self {
            cpu_capacity: DEFAULT_CPU_CAPACITY,
            task_stack_size: DEFAULT_TASK_STACK_SIZE,
            rt_cpumask: &[],
        };

        if let Ok(smp) = env::var("SMP") {
            config.cpu_capacity = parse_usize(&smp)
                .map_err(|err| invalid_data(format!("failed to parse SMP value `{smp}`: {err}")))?;
        }

        if let Ok(value) = env::var("RT_CPUMASK") {
            let parsed = value
                .split(',')
                .filter(|part| !part.trim().is_empty())
                .map(parse_usize)
                .collect::<std::result::Result<Vec<usize>, _>>()
                .map_err(|err| {
                    invalid_data(format!("failed to parse RT_CPUMASK `{value}`: {err}"))
                })?;
            for index in &parsed {
                if *index >= config.cpu_capacity {
                    return Err(invalid_data(format!(
                        "RT_CPUMASK entry {index} is out of range (CPU_CAPACITY={})",
                        config.cpu_capacity
                    )));
                }
            }
            config.rt_cpumask = parsed.leak();
        }

        Ok(config)
    }
}

fn parse_usize(value: &str) -> std::result::Result<usize, std::num::ParseIntError> {
    let value = value.replace('_', "");
    if let Some(hex) = value.strip_prefix("0x") {
        usize::from_str_radix(hex, 16)
    } else {
        value.parse()
    }
}

fn invalid_data(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semantic_source(source: &str) -> String {
        source
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect()
    }

    #[test]
    fn build_info_source_generates_task_constants() {
        assert_eq!(
            semantic_source(&build_info_source(TaskConfig {
                cpu_capacity: DEFAULT_CPU_CAPACITY,
                task_stack_size: DEFAULT_TASK_STACK_SIZE,
                rt_cpumask: &[],
            })),
            semantic_source(
                "pub const CPU_CAPACITY: usize = 16usize; pub const DEFAULT_TASK_STACK_SIZE: \
                 usize = 262144usize; pub const RT_CPUMASK: &[usize] = &[];"
            )
        );
    }
}
