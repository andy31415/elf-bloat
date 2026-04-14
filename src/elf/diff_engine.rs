use crate::elf::parsers::{GoblinParser, NativeParser, NmParser};
use crate::elf::symbol_diff;
use crate::elf::symbols::ElfParser;
use crate::output::{ViewerTool, pipe_to_viewer};
use eyre::{Result, eyre};
use log::info;
use std::path::Path;

/// The diff engine to use for comparison.
#[derive(Debug, PartialEq)]
pub enum DiffEngine {
    Nm,
    Native,
    Goblin,
}

impl std::str::FromStr for DiffEngine {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "nm" => Ok(Self::Nm),
            "native" => Ok(Self::Native),
            "goblin" => Ok(Self::Goblin),
            other => Err(eyre!(
                "Unknown diff engine '{}'. Valid options: nm, native, goblin",
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
        DiffEngine::Nm => {
            let parser = NmParser::default();
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let report = symbol_diff::compare_symbols(from_symbols, to_symbols);
            let csv_data = symbol_diff::generate_diff_csv(&report)?;
            pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;
        }
        DiffEngine::Native => {
            let parser = NativeParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let report = symbol_diff::compare_symbols(from_symbols, to_symbols);
            let csv_data = symbol_diff::generate_diff_csv(&report)?;
            pipe_to_viewer(csv_data.as_bytes(), workdir, viewer)?;
        }
        DiffEngine::Goblin => {
            let parser = GoblinParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            let report = symbol_diff::compare_symbols(from_symbols, to_symbols);
            let csv_data = symbol_diff::generate_diff_csv(&report)?;
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



#[cfg(test)]
mod tests {
    use super::*;

    // ── DiffEngine parsing ────────────────────────────────────────────────────

    #[test]
    fn test_parse_diff_engine_known() {
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
