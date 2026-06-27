// Native-only CSV helpers to produce a table AST node.
// Not compiled into wasm builds.

use serde_json::{Value, json};

/// Options for reading CSV into a table AST node.
#[derive(Debug, Clone)]
pub struct CsvReadOptions {
    /// 1-based line number to start reading from (inclusive). Default: 1
    pub start_line: usize,
    /// Whether the CSV contains a header row. If true and `header_line` is
    /// None, the first row at or after `start_line` is used as header.
    pub header: bool,
    /// Optional explicit 1-based line number that contains header. If set,
    /// that line is used as header regardless of start_line.
    pub header_line: Option<usize>,
    /// Optional row-range (1-based, inclusive) relative to the file. If set,
    /// only rows within this range are included (after applying start_line).
    pub row_range: Option<(usize, usize)>,
    /// Optional column-range (1-based, inclusive). If set, only columns in
    /// this range are included.
    pub col_range: Option<(usize, usize)>,
    /// delimiter byte (',' by default)
    pub delimiter: u8,
}

impl Default for CsvReadOptions {
    fn default() -> Self {
        Self {
            start_line: 1,
            header: false,
            header_line: None,
            row_range: None,
            col_range: None,
            delimiter: b',',
        }
    }
}

/// Parse a CSV string and return a `table` AST node as `serde_json::Value`.
/// Behavior:
/// - `start_line` and `header_line` are 1-based file line numbers.
/// - If `header` is true and no `header_line` given, the first record at or
///   after `start_line` is used as header and excluded from `rows`.
pub fn parse_csv_to_table_ast_str(csv_text: &str, opts: &CsvReadOptions) -> Result<Value, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(opts.delimiter)
        .flexible(true)
        .from_reader(csv_text.as_bytes());

    let mut rows_vec: Vec<Vec<String>> = Vec::new();
    for result in rdr.records() {
        match result {
            Ok(rec) => rows_vec.push(rec.iter().map(|s| s.to_string()).collect()),
            Err(e) => return Err(e.to_string()),
        }
    }

    // rows_vec index 0 corresponds to file line 1
    let total_lines = rows_vec.len();
    if opts.start_line == 0 {
        return Err("start_line must be >= 1".into());
    }
    let mut effective_rows: Vec<Vec<String>> = Vec::new();
    for (i, row) in rows_vec.into_iter().enumerate() {
        let lineno = i + 1;
        if lineno < opts.start_line {
            continue;
        }
        if let Some((rstart, rend)) = opts.row_range
            && (lineno < rstart || lineno > rend)
        {
            continue;
        }
        effective_rows.push(row);
    }

    // Determine header
    let mut headers: Vec<Value> = Vec::new();
    let mut body_rows: Vec<Vec<Value>> = Vec::new();

    if opts.header {
        // find header line relative to file
        let header_lineno = if let Some(h) = opts.header_line {
            h
        } else {
            opts.start_line
        };
        let idx = header_lineno.saturating_sub(opts.start_line);
        if header_lineno < opts.start_line || header_lineno > total_lines {
            return Err("header_line out of range".into());
        }
        // effective_rows index for header
        let header_row = &effective_rows
            .get(idx)
            .ok_or_else(|| "header_line not available after start_line".to_string())?;
        // build headers
        for cell in header_row.iter() {
            headers.push(json!({ "t": "_cell", "children": [{ "t": "text", "text": cell }] }));
        }
        // body rows are everything in effective_rows except at position idx
        for (i, row) in effective_rows.into_iter().enumerate() {
            if i == idx {
                continue;
            }
            body_rows.push(
                row.into_iter()
                    .map(|c| json!({ "t": "_cell", "children": [{ "t": "text", "text": c }] }))
                    .collect(),
            );
        }
    } else {
        // no header; all effective_rows become body rows
        for row in effective_rows.into_iter() {
            body_rows.push(
                row.into_iter()
                    .map(|c| json!({ "t": "_cell", "children": [{ "t": "text", "text": c }] }))
                    .collect(),
            );
        }
    }

    // Apply column slicing if requested. First determine max columns
    let mut maxcols = 0usize;
    for r in body_rows.iter() {
        if r.len() > maxcols {
            maxcols = r.len();
        }
    }
    if headers.len() > maxcols {
        maxcols = headers.len();
    }

    // Build align array default "none"
    let mut align = vec![];
    for _ in 0..maxcols {
        align.push(json!("none"));
    }

    // Apply col_range slicing
    let col_start = opts.col_range.map(|(a, _)| a).unwrap_or(1);
    let col_end = opts.col_range.map(|(_, b)| b).unwrap_or(maxcols);
    if col_start == 0 || col_end < col_start {
        return Err("invalid col_range".into());
    }

    // Slice headers
    let sliced_headers: Vec<Value> = if headers.is_empty() {
        vec![]
    } else {
        let mut out = Vec::new();
        for (i, h) in headers.into_iter().enumerate() {
            let colno = i + 1;
            if colno < col_start || colno > col_end {
                continue;
            }
            out.push(h);
        }
        out
    };

    // Slice rows
    let mut sliced_rows: Vec<Value> = Vec::new();
    for row in body_rows.into_iter() {
        let mut out_row: Vec<Value> = Vec::new();
        for (i, cell) in row.into_iter().enumerate() {
            let colno = i + 1;
            if colno < col_start || colno > col_end {
                continue;
            }
            out_row.push(cell);
        }
        // pad to keep rectangular shape
        for _ in out_row.len()..(col_end - col_start + 1) {
            out_row.push(json!({ "t": "_cell", "children": [{ "t": "text", "text": "" }] }));
        }
        sliced_rows.push(Value::Array(out_row));
    }

    let tbl = json!({
        "t": "table",
        "align": Value::Array(align),
        "headers": Value::Array(sliced_headers),
        "rows": Value::Array(sliced_rows)
    });

    Ok(tbl)
}

/// Read a CSV file and produce a `table` AST node.
pub fn read_csv_as_table_ast(
    path: &std::path::Path,
    opts: &CsvReadOptions,
) -> Result<Value, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_csv_to_table_ast_str(&s, opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn simple_csv_no_header() {
        let csv = "1,2,3\n4,5,6\n";
        let opts = CsvReadOptions::default();
        let v = parse_csv_to_table_ast_str(csv, &opts).expect("parse failed");
        assert_eq!(v["t"], "table");
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].as_array().unwrap()[0]["children"][0]["text"], "1");
    }

    #[test]
    fn csv_with_header_and_slice_cols() {
        let csv = "Name,Value,Unit\nGain,12,dB\nLoss,3,dB\n";
        let mut opts = CsvReadOptions::default();
        opts.header = true;
        opts.col_range = Some((2, 3));
        let v = parse_csv_to_table_ast_str(csv, &opts).expect("parse failed");
        assert_eq!(v["headers"].as_array().unwrap().len(), 2);
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows[0].as_array().unwrap()[0]["children"][0]["text"], "12");
    }

    #[test]
    fn read_csv_file_roundtrip() {
        let csv = "h1,h2\na,1\nb,2\n";
        let opts = CsvReadOptions {
            header: true,
            start_line: 1,
            ..Default::default()
        };
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_csv.csv");
        fs::write(&path, csv).expect("write failed");
        let v = read_csv_as_table_ast(&path, &opts).expect("read failed");
        assert_eq!(
            v["headers"].as_array().unwrap()[0]["children"][0]["text"],
            "h1"
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn start_line_zero_errors() {
        let csv = "a,b\n1,2\n";
        let mut opts = CsvReadOptions::default();
        opts.start_line = 0;
        let res = parse_csv_to_table_ast_str(csv, &opts);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("start_line must be >= 1"));
    }

    #[test]
    fn header_line_out_of_range_errors() {
        let csv = "a,b\n1,2\n";
        let mut opts = CsvReadOptions::default();
        opts.header = true;
        opts.header_line = Some(10);
        let res = parse_csv_to_table_ast_str(csv, &opts);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("header_line out of range"));
    }

    #[test]
    fn header_line_not_available_after_start_errors() {
        let csv = "h1,h2\na,1\nb,2\n";
        let mut opts = CsvReadOptions::default();
        opts.header = true;
        opts.start_line = 2; // skips header
        opts.header_line = Some(1);
        let res = parse_csv_to_table_ast_str(csv, &opts);
        assert!(res.is_err());
        // parser returns "header_line out of range" in this scenario
        assert!(res.err().unwrap().contains("header_line out of range"));
    }

    #[test]
    fn invalid_col_range_errors() {
        let csv = "a,b\n1,2\n";
        let mut opts = CsvReadOptions::default();
        opts.col_range = Some((2, 1));
        let res = parse_csv_to_table_ast_str(csv, &opts);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("invalid col_range"));
    }

    #[test]
    fn row_range_filters_rows() {
        let csv = "a,b,c\n1,2,3\n4,5,6\n7,8,9\n";
        let mut opts = CsvReadOptions::default();
        opts.row_range = Some((2, 3)); // include only lines 2 and 3
        let v = parse_csv_to_table_ast_str(csv, &opts).expect("parse failed");
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].as_array().unwrap()[0]["children"][0]["text"], "1");
    }

    #[test]
    fn padding_short_rows_produces_empty_cells() {
        let csv = "a,b,c\n1,2\n3,4,5\n";
        let mut opts = CsvReadOptions::default();
        opts.row_range = None;
        // flexible parser allows uneven records; padding logic will pad shorter rows
        let v = parse_csv_to_table_ast_str(csv, &opts).expect("parse failed");
        let rows = v["rows"].as_array().unwrap();
        // second data row (index 1) had only 2 cols; after padding it should have 3
        assert_eq!(rows[1].as_array().unwrap().len(), 3);
        // padded cell text should be empty
        assert_eq!(rows[1].as_array().unwrap()[2]["children"][0]["text"], "");
    }

    #[test]
    fn custom_delimiter_parses_semicolon() {
        let csv = "a;b;c\n1;2;3\n";
        let mut opts = CsvReadOptions::default();
        opts.delimiter = b';';
        let v = parse_csv_to_table_ast_str(csv, &opts).expect("parse failed");
        let rows = v["rows"].as_array().unwrap();
        // first row is header-like row when header=false
        assert_eq!(rows[0].as_array().unwrap()[1]["children"][0]["text"], "b");
    }

    #[test]
    fn read_csv_missing_file_errs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_csv_999.csv");
        let opts = CsvReadOptions::default();
        let res = read_csv_as_table_ast(&path, &opts);
        assert!(res.is_err());
    }
}
