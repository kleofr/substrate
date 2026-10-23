use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Reads a file from disk either completely or within a specified 1-indexed line range.
///
/// Returns an `io::Error` if:
/// - The file does not exist or cannot be read.
/// - The given path is a directory.
/// - `start_line` is greater than `end_line`.
pub fn read_file<P: AsRef<Path>>(
    path: P,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> io::Result<String> {
    let path = path.as_ref();
    if path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path '{}' is a directory", path.display()),
        ));
    }

    if let (Some(start), Some(end)) = (start_line, end_line) {
        if start > end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("start_line ({start}) cannot be greater than end_line ({end})"),
            ));
        }
    }

    match (start_line, end_line) {
        (None, None) => fs::read_to_string(path),
        _ => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);

            let start = start_line.unwrap_or(1).max(1);
            let mut result = Vec::new();

            for (idx, line_res) in reader.lines().enumerate() {
                let current_line = idx + 1; // 1-indexed
                if current_line < start {
                    continue;
                }
                if let Some(end) = end_line {
                    if current_line > end {
                        break;
                    }
                }
                result.push(line_res?);
            }

            Ok(result.join("\n"))
        }
    }
}

/// Executes the read_file tool using an argument map.
///
/// Expects:
/// - `"path"`: `Value::String` (required)
/// - `"start_line"`: `Value::Number` (optional, 1-indexed)
/// - `"end_line"`: `Value::Number` (optional, 1-indexed)
pub fn execute_from_args(args: &HashMap<String, Value>) -> io::Result<String> {
    let path = substrate_utils::parse_args::get_str(args, "path")?;
    let start_line = substrate_utils::parse_args::get_opt_usize(args, "start_line")?;
    let end_line = substrate_utils::parse_args::get_opt_usize(args, "end_line")?;

    read_file(path, start_line, end_line)
}

/// Parses JSON arguments and executes the read_file tool.
pub fn execute(json_args: &str) -> io::Result<String> {
    let args = substrate_utils::parse_args::parse_args(json_args)?;
    execute_from_args(&args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_temp_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).expect("file creation failed");
        file.write_all(content.as_bytes()).expect("write failed");
        (dir, file_path)
    }

    #[test]
    fn reads_full_file() {
        let content = "line 1\nline 2\nline 3";
        let (_dir, file_path) = create_temp_file(content);

        let output = read_file(&file_path, None, None).expect("read failed");
        assert_eq!(output, content);
    }

    #[test]
    fn reads_selected_lines_range() {
        let content = "line 1\nline 2\nline 3\nline 4";
        let (_dir, file_path) = create_temp_file(content);

        let output = read_file(&file_path, Some(2), Some(3)).expect("read failed");
        assert_eq!(output, "line 2\nline 3");
    }

    #[test]
    fn reads_with_only_start_line() {
        let content = "line 1\nline 2\nline 3";
        let (_dir, file_path) = create_temp_file(content);

        let output = read_file(&file_path, Some(2), None).expect("read failed");
        assert_eq!(output, "line 2\nline 3");
    }

    #[test]
    fn rejects_start_greater_than_end() {
        let content = "line 1\nline 2";
        let (_dir, file_path) = create_temp_file(content);

        let res = read_file(&file_path, Some(3), Some(1));
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn rejects_directory_path() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        let res = read_file(dir.path(), None, None);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn executes_from_json_string() {
        let content = "first\nsecond\nthird";
        let (_dir, file_path) = create_temp_file(content);
        let path_str = file_path.to_str().expect("valid path");

        let json = format!(r#"{{"path": {:?}, "start_line": 1, "end_line": 2}}"#, path_str);
        let output = execute(&json).expect("execution failed");
        assert_eq!(output, "first\nsecond");
    }
}
