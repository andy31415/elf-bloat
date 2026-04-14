use clap::{Parser, ValueEnum};
use color_eyre::eyre::Result;
use elf_bloat::elf::diff_engine::{self, DiffEngine};
use elf_bloat::output::{ViewerTool, display_diff, display_symbols};
use env_logger::Env;
use std::path::PathBuf;

/// A CLI tool for comparing ELF file symbol sizes.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The file to analyze or the comparison file if --compare-base is used.
    file: String,

    /// Baseline build file path for comparison.
    #[arg(long)]
    compare_base: Option<String>,

    /// Viewer tool to pipe CSV output to.
    /// Options: default, vd, visidata, csvlens, none, custom:<name>
    #[arg(long, default_value = "default")]
    viewer: String,

    /// Diff engine to use for comparison.
    /// Options: nm, native, goblin
    #[arg(long, default_value = "native")]
    diff_engine: String,

    /// Set the logging level.
    #[arg(short, long, default_value_t = LogLevel::Info, ignore_case = true)]
    log_level: LogLevel,
}

/// Log verbosity levels accepted by `--log-level`.
#[derive(ValueEnum, Debug, Clone, Default)]
#[value(rename_all = "lowercase")]
enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        f.write_str(s)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or(cli.log_level.to_string()))
        .init();
    color_eyre::install()?;

    let viewer: ViewerTool = cli.viewer.parse()?;
    let diff_engine: DiffEngine = cli.diff_engine.parse()?;

    let workdir = std::env::current_dir()?;

    if let Some(base) = cli.compare_base {
        let report = diff_engine::run_diff(
            &PathBuf::from(&base),
            &PathBuf::from(&cli.file),
            &diff_engine,
        )?;
        display_diff(&report, &workdir, &viewer)?;
    } else {
        let symbols = diff_engine::run_single(&PathBuf::from(&cli.file), &diff_engine)?;
        display_symbols(symbols, &workdir, &viewer)?;
    }

    Ok(())
}
