use crate::elf::symbols::ElfParser;
use crate::elf::parsers::{GoblinParser, NativeParser, NmParser};
use crate::elf::symbol_diff;
use crate::output::{ViewerTool, pipe_to_viewer, build_viewer_chain, CommandChain};
use eyre::{Result, eyre};
use log::{debug, info};
use std::path::Path;
use std::process::Command;



/// The diff engine to use for comparison.
#[derive(Debug, PartialEq)]
pub enum DiffEngine {
    Script,
    Nm,
    Native,
    Goblin,
}

impl std::str::FromStr for DiffEngine {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "script" => Ok(Self::Script),
            "nm" => Ok(Self::Nm),
            "native" => Ok(Self::Native),
            "goblin" => Ok(Self::Goblin),
            other => Err(eyre!(
                "Unknown diff engine '{}'. Valid options: script, nm, native, goblin",
                other
            )),
        }
    }
}



/// Runs the comparison between two artifact files based on the selected engine.
pub fn run_diff(
    from_path: &Path,
    to_path: &Path,
    workdir: &Path,
    diff_engine: &DiffEngine,
    extra_args: &[String],
    viewer: &ViewerTool,
) -> Result<()> {
    if !from_path.exists() {
        return Err(eyre!("From file not found: {}", from_path.display()));
    }
    if !to_path.exists() {
        return Err(eyre!("To file not found: {}", to_path.display()));
    }

    info!(
        "Comparing {} and {} using {:?}",
        from_path.display(),
        to_path.display(),
        diff_engine
    );

    match diff_engine {
        DiffEngine::Script => run_script_diff(from_path, to_path, workdir, extra_args, viewer)?,
        DiffEngine::Nm => {
            let parser = NmParser::default();
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let csv_data = symbol_diff::generate_diff_csv(from_symbols, to_symbols)?;
            pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;
        }
        DiffEngine::Native => {
            let parser = NativeParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let csv_data = symbol_diff::generate_diff_csv(from_symbols, to_symbols)?;
            pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;
        }
        DiffEngine::Goblin => {
            let parser = GoblinParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let csv_data = symbol_diff::generate_diff_csv(from_symbols, to_symbols)?;
            pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;
        }
    }
    Ok(())
}

/// Runs the symbol size analysis for a single artifact file.
pub fn run_single(
    path: &Path,
    workdir: &Path,
    diff_engine: &DiffEngine,
    viewer: &ViewerTool,
) -> Result<()> {
    if !path.exists() {
        return Err(eyre!("File not found: {}", path.display()));
    }

    info!("Analyzing {} using {:?}", path.display(), diff_engine);

    let symbols = match diff_engine {
        DiffEngine::Script => {
            return Err(eyre!("Script engine does not support single file analysis"));
        }
        DiffEngine::Nm => {
            let parser = NmParser::default();
            parser.get_symbols(path)?
        }
        DiffEngine::Native => {
            let parser = NativeParser;
            parser.get_symbols(path)?
        }
        DiffEngine::Goblin => {
            let parser = GoblinParser;
            parser.get_symbols(path)?
        }
    };

    let csv_data = symbol_diff::generate_symbols_csv(symbols)?;
    pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;

    Ok(())
}



/// Executes the size difference script to compare the two artifact files.
///
/// Uses `uv run` to execute `scripts/tools/binary_elf_size_diff.py`.
fn run_script_diff(
    from_path: &Path,
    to_path: &Path,
    workdir: &Path,
    extra_args: &[String],
    viewer: &ViewerTool,
) -> Result<()> {
    let mut diff_cmd = Command::new("uv");
    diff_cmd
        .args(["run", "scripts/tools/binary_elf_size_diff.py"])
        .current_dir(workdir);

    // Build the full diff command (including output format and positional paths)
    // before constructing the chain so the chain's first command is immutable
    // once created.
    let chain = if extra_args.is_empty() {
        build_viewer_chain(diff_cmd, from_path, to_path, workdir, viewer)
    } else {
        diff_cmd.args(extra_args).arg(to_path).arg(from_path);
        CommandChain::new(diff_cmd)
    };
    debug!("Executing: {:?}", chain);
    chain.execute()
}



#[cfg(test)]
mod tests {
    use super::*;


    // ── DiffEngine parsing ────────────────────────────────────────────────────

    #[test]
    fn test_parse_diff_engine_known() {
        assert_eq!("script".parse::<DiffEngine>().unwrap(), DiffEngine::Script);
        assert_eq!("nm".parse::<DiffEngine>().unwrap(), DiffEngine::Nm);
        assert_eq!("native".parse::<DiffEngine>().unwrap(), DiffEngine::Native);
        assert_eq!("goblin".parse::<DiffEngine>().unwrap(), DiffEngine::Goblin);
    }

    #[test]
    fn test_parse_diff_engine_unknown() {
        assert!("".parse::<DiffEngine>().is_err());
        assert!("default".parse::<DiffEngine>().is_err());
        assert!("Native".parse::<DiffEngine>().is_err());
    }

}
