use crate::elf::symbols::{ChangeType, DiffResult, DiffTotals, Symbol, SymbolDiffReport};
use cpp_demangle;
use csv::WriterBuilder;
use eyre::Result;
use std::collections::HashMap;

pub fn demangle_name(name: &str) -> String {
    match cpp_demangle::Symbol::new(name.as_bytes()) {
        Ok(symbol) => symbol.demangle().unwrap_or_else(|_| name.to_string()),
        Err(_) => name.to_string(),
    }
}

/// Compares two vectors of `Symbol` structs and generates a CSV report of the differences.
///
/// The output CSV string includes a header row: "Change,Type,Diff,Symbol,Base Size,Size".
/// Each subsequent row represents a symbol that has been added, removed, or changed in size.
/// Symbols with no size difference are not included in the symbol list.
/// The rows are sorted in ascending order based on the 'Diff' column.
///
/// A final "TOTAL" row is appended to summarize the total size difference, base size, and new size
/// across all symbols from both input lists, including those not shown in the diff list.
///
/// # Arguments
///
/// * `from_symbols`: A vector of Symbols representing the baseline.
/// * `to_symbols`: A vector of Symbols representing the version to compare.
///
/// # Returns
///
/// A `Result` containing the CSV formatted string or an error.
pub fn compare_symbols(from_symbols: Vec<Symbol>, to_symbols: Vec<Symbol>) -> SymbolDiffReport {
    let from_map: HashMap<String, Symbol> = from_symbols
        .into_iter()
        .map(|s| (s.name.clone(), s))
        .collect();
    let to_map: HashMap<String, Symbol> = to_symbols
        .into_iter()
        .map(|s| (s.name.clone(), s))
        .collect();

    let mut diff_results: Vec<DiffResult> = Vec::new();
    let mut all_keys: Vec<&String> = from_map.keys().collect();
    for key in to_map.keys() {
        if !from_map.contains_key(key) {
            all_keys.push(key);
        }
    }
    all_keys.sort();
    all_keys.dedup();

    let mut total_diff: i64 = 0;
    let mut total_base_size: usize = 0;
    let mut total_size: usize = 0;

    for key in all_keys {
        let from_sym = from_map.get(key);
        let to_sym = to_map.get(key);

        let size1 = from_sym.map(|s| s.size).unwrap_or(0);
        let size2 = to_sym.map(|s| s.size).unwrap_or(0);
        let diff = size2 as i64 - size1 as i64;

        total_base_size += size1;
        total_size += size2;
        total_diff += diff;

        if diff != 0 {
            let change_type = match (from_sym, to_sym) {
                (Some(_), Some(_)) => ChangeType::Changed,
                (None, Some(_)) => ChangeType::Added,
                (Some(_), None) => ChangeType::Removed,
                (None, None) => unreachable!(),
            };

            let symbol = from_sym.or(to_sym).unwrap();

            diff_results.push(DiffResult {
                change_type,
                symbol_kind: symbol.kind,
                symbol_name: symbol.demangled.clone(),
                diff,
                base_size: size1,
                size: size2,
            });
        }
    }

    diff_results.sort_by(|a, b| a.diff.cmp(&b.diff));

    SymbolDiffReport {
        diffs: diff_results,
        totals: DiffTotals {
            base_size: total_base_size,
            size: total_size,
            diff: total_diff,
        },
    }
}

pub fn generate_diff_csv(report: &SymbolDiffReport) -> Result<String> {
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["Change", "Type", "Diff", "Symbol", "Base Size", "Size"])?;

    for result in &report.diffs {
        wtr.write_record(&[
            result.change_type.to_string(),
            result.symbol_kind.to_string(),
            result.diff.to_string(),
            result.symbol_name.clone(),
            result.base_size.to_string(),
            result.size.to_string(),
        ])?;
    }

    // Add TOTAL row
    wtr.write_record(&[
        "TOTAL".to_string(),
        "".to_string(),
        format!("{:+}", report.totals.diff),
        "".to_string(),
        report.totals.base_size.to_string(),
        report.totals.size.to_string(),
    ])?;

    wtr.flush()?;
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

/// Generates a CSV report of symbol sizes for a single ELF file.
pub fn generate_symbols_csv(symbols: Vec<Symbol>) -> Result<String> {
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["Type", "Size", "Symbol"])?;

    let mut total_size: usize = 0;
    let mut sorted_symbols = symbols;
    sorted_symbols.sort_by(|a, b| b.size.cmp(&a.size));

    for sym in &sorted_symbols {
        total_size += sym.size;
        wtr.write_record(&[
            sym.kind.to_string(),
            sym.size.to_string(),
            sym.demangled.clone(),
        ])?;
    }

    wtr.write_record(&["TOTAL".to_string(), total_size.to_string(), "".to_string()])?;

    wtr.flush()?;
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::symbols::SymbolKind;

    fn create_sym(name: &str, kind: SymbolKind, size: usize) -> Symbol {
        Symbol {
            name: name.to_string(),
            demangled: name.to_string(),
            kind,
            size,
            address: None,
        }
    }

    #[test]
    fn test_demangle_name() {
        assert_eq!(
            demangle_name("_ZN6System5Layer4InitEv"),
            "System::Layer::Init()"
        );
        assert_eq!(demangle_name("not_mangled"), "not_mangled");
    }

    #[test]
    fn test_generate_symbols_csv() {
        let symbols = vec![
            create_sym("foo", SymbolKind::Code, 100),
            create_sym("bar", SymbolKind::Data, 50),
        ];
        let csv = generate_symbols_csv(symbols).unwrap();
        let expected = "Type,Size,Symbol
Code,100,foo
Data,50,bar
TOTAL,150,\n";
        assert_eq!(csv, expected);
    }

    #[test]
    fn test_generate_diff_csv_empty() {
        let report = compare_symbols(vec![], vec![]);
        let csv = generate_diff_csv(&report).unwrap();
        let expected = "Change,Type,Diff,Symbol,Base Size,Size
TOTAL,,+0,,0,0
";
        assert_eq!(csv, expected);
    }

    #[test]
    fn test_generate_diff_csv_added_removed() {
        let from = vec![create_sym("foo", SymbolKind::Code, 100)];
        let to = vec![create_sym("bar", SymbolKind::Data, 50)];
        let report = compare_symbols(from, to);
        let csv = generate_diff_csv(&report).unwrap();
        let expected = "Change,Type,Diff,Symbol,Base Size,Size
REMOVED,Code,-100,foo,100,0
ADDED,Data,50,bar,0,50
TOTAL,,-50,,100,50
";
        let expected_lines: Vec<&str> = expected
            .trim()
            .split(
                "
",
            )
            .collect();
        let mut csv_lines: Vec<&str> = csv
            .trim()
            .split(
                "
",
            )
            .collect();

        assert_eq!(csv_lines.remove(0), expected_lines[0]); // Header
        assert_eq!(csv_lines.pop(), expected_lines.last().copied()); // TOTAL

        csv_lines.sort();
        let mut expected_data_lines = expected_lines[1..expected_lines.len() - 1].to_vec();
        expected_data_lines.sort();
        assert_eq!(csv_lines, expected_data_lines);
    }

    #[test]
    fn test_generate_diff_csv_changed() {
        let from = vec![create_sym("foo", SymbolKind::Code, 100)];
        let to = vec![create_sym("foo", SymbolKind::Code, 150)];
        let report = compare_symbols(from, to);
        let csv = generate_diff_csv(&report).unwrap();
        let expected = "Change,Type,Diff,Symbol,Base Size,Size
CHANGED,Code,50,foo,100,150
TOTAL,,+50,,100,150
";
        assert_eq!(csv, expected);
    }

    #[test]
    fn test_generate_diff_csv_no_diff() {
        let from = vec![create_sym("foo", SymbolKind::Code, 100)];
        let to = vec![create_sym("foo", SymbolKind::Code, 100)];
        let report = compare_symbols(from, to);
        let csv = generate_diff_csv(&report).unwrap();
        let expected = "Change,Type,Diff,Symbol,Base Size,Size
TOTAL,,+0,,100,100
";
        assert_eq!(csv, expected);
    }

    #[test]
    fn test_compare_symbols_sorting() {
        let from = vec![
            create_sym("a", SymbolKind::Code, 100),
            create_sym("b", SymbolKind::Code, 100),
        ];
        let to = vec![
            create_sym("a", SymbolKind::Code, 50),  // Diff -50
            create_sym("b", SymbolKind::Code, 110), // Diff +10
            create_sym("c", SymbolKind::Data, 20),  // Diff +20
        ];
        let report = compare_symbols(from, to);

        assert_eq!(report.diffs.len(), 3);
        assert_eq!(report.diffs[0].symbol_name, "a");
        assert_eq!(report.diffs[0].diff, -50);
        assert_eq!(report.diffs[1].symbol_name, "b");
        assert_eq!(report.diffs[1].diff, 10);
        assert_eq!(report.diffs[2].symbol_name, "c");
        assert_eq!(report.diffs[2].diff, 20);

        assert_eq!(report.totals.diff, -20);
        assert_eq!(report.totals.base_size, 200);
        assert_eq!(report.totals.size, 180);
    }

    #[test]
    fn test_generate_diff_csv_totals() {
        let from = vec![create_sym("a", SymbolKind::Code, 100)];
        let to = vec![create_sym("b", SymbolKind::Data, 30)];
        let report = compare_symbols(from, to);
        let csv = generate_diff_csv(&report).unwrap();
        // Extract TOTAL line
        let total_line = csv.lines().last().unwrap();
        assert_eq!(total_line, "TOTAL,,-70,,100,30");
    }
}
