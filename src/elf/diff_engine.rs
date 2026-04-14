use crate::elf::parsers::{GoblinParser, NativeParser, NmParser};
use crate::elf::symbol_diff;
use crate::elf::symbols::{ElfParser, Symbol, SymbolDiffReport};
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
    diff_engine: &DiffEngine,
) -> Result<SymbolDiffReport> {
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

    let report = match diff_engine {
        DiffEngine::Nm => {
            let parser = NmParser::default();
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            symbol_diff::compare_symbols(from_symbols, to_symbols)
        }
        DiffEngine::Native => {
            let parser = NativeParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            symbol_diff::compare_symbols(from_symbols, to_symbols)
        }
        DiffEngine::Goblin => {
            let parser = GoblinParser;
            let from_symbols = parser.get_symbols(from_path)?;
            let to_symbols = parser.get_symbols(to_path)?;
            symbol_diff::compare_symbols(from_symbols, to_symbols)
        }
    };
    Ok(report)
}

/// Runs the symbol size analysis for a single artifact file.
pub fn run_single(
    path: &Path,
    diff_engine: &DiffEngine,
) -> Result<Vec<Symbol>> {
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

    Ok(symbols)
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
