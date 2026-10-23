use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

/// Parses JSON arguments from any string-like input (`&str`, `String`, etc.) into a `HashMap<String, Value>`.
///
/// Returns an `io::Error` with `ErrorKind::InvalidData` if the input is not valid JSON
/// or if the top-level JSON value is not an object.
pub fn parse_args<S: AsRef<str>>(json: S) -> io::Result<HashMap<String, Value>> {
    parse_args_from_slice(json.as_ref().as_bytes())
}

/// Parses JSON arguments from a byte slice into a `HashMap<String, Value>`.
pub fn parse_args_from_slice(bytes: &[u8]) -> io::Result<HashMap<String, Value>> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    parse_args_from_value(value)
}

/// Parses JSON arguments from any `io::Read` stream into a `HashMap<String, Value>`.
///
/// Manages both I/O errors occurring during stream consumption and JSON decoding errors.
pub fn parse_args_from_reader<R: Read>(reader: R) -> io::Result<HashMap<String, Value>> {
    let value: Value = serde_json::from_reader(reader).map_err(|err| {
        if err.is_io() {
            // Preserves the underlying I/O error condition
            io::Error::new(io::ErrorKind::Other, err)
        } else {
            io::Error::new(io::ErrorKind::InvalidData, err)
        }
    })?;
    parse_args_from_value(value)
}

/// Reads a JSON file from disk and parses it into a `HashMap<String, Value>`.
///
/// Handles file opening/reading I/O errors and argument parsing errors.
pub fn parse_args_from_file<P: AsRef<Path>>(path: P) -> io::Result<HashMap<String, Value>> {
    let file = File::open(path)?;
    parse_args_from_reader(BufReader::new(file))
}

/// Converts a decoded `serde_json::Value` into a `HashMap<String, Value>`.
///
/// Enforces that tool argument payloads must be JSON objects (key-value maps).
pub fn parse_args_from_value(value: Value) -> io::Result<HashMap<String, Value>> {
    match value {
        Value::Object(map) => Ok(map.into_iter().collect()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "tool arguments must be a JSON object",
        )),
    }
}

/// Extracts a required string reference from the arguments map.
pub fn get_str<'a>(args: &'a HashMap<String, Value>, key: &str) -> io::Result<&'a str> {
    match args.get(key) {
        Some(Value::String(s)) => Ok(s.as_str()),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be a string"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required argument '{key}'"),
        )),
    }
}

/// Extracts an optional string reference from the arguments map.
pub fn get_opt_str<'a>(args: &'a HashMap<String, Value>, key: &str) -> io::Result<Option<&'a str>> {
    match args.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be a string"),
        )),
    }
}

/// Extracts a required unsigned integer (usize) from the arguments map.
pub fn get_usize(args: &HashMap<String, Value>, key: &str) -> io::Result<usize> {
    match args.get(key) {
        Some(Value::Number(n)) => {
            let raw = n.as_u64().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("argument '{key}' must be an unsigned integer"),
                )
            })?;
            usize::try_from(raw).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("argument '{key}' exceeds platform pointer size: {e}"),
                )
            })
        }
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be an unsigned integer"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required argument '{key}'"),
        )),
    }
}

/// Extracts an optional unsigned integer (usize) from the arguments map.
pub fn get_opt_usize(args: &HashMap<String, Value>, key: &str) -> io::Result<Option<usize>> {
    match args.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Number(n)) => {
            let raw = n.as_u64().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("argument '{key}' must be an unsigned integer"),
                )
            })?;
            let val = usize::try_from(raw).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("argument '{key}' exceeds platform pointer size: {e}"),
                )
            })?;
            Ok(Some(val))
        }
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be an unsigned integer"),
        )),
    }
}

/// Extracts a required boolean from the arguments map.
pub fn get_bool(args: &HashMap<String, Value>, key: &str) -> io::Result<bool> {
    match args.get(key) {
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be a boolean"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required argument '{key}'"),
        )),
    }
}

/// Extracts an optional boolean from the arguments map.
pub fn get_opt_bool(args: &HashMap<String, Value>, key: &str) -> io::Result<Option<bool>> {
    match args.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be a boolean"),
        )),
    }
}

/// Extracts a required signed 64-bit integer from the arguments map.
pub fn get_i64(args: &HashMap<String, Value>, key: &str) -> io::Result<i64> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_i64().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("argument '{key}' must be a signed 64-bit integer"),
            )
        }),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be an integer"),
        )),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required argument '{key}'"),
        )),
    }
}

/// Extracts an optional signed 64-bit integer from the arguments map.
pub fn get_opt_i64(args: &HashMap<String, Value>, key: &str) -> io::Result<Option<i64>> {
    match args.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Number(n)) => {
            let val = n.as_i64().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("argument '{key}' must be a signed 64-bit integer"),
                )
            })?;
            Ok(Some(val))
        }
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("argument '{key}' must be an integer"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_valid_json_object_string() {
        let json = r#"{"cmd": "cargo test", "timeout": 30, "flag": true}"#;
        let args = parse_args(json).expect("should parse valid object");

        assert_eq!(args.get("cmd"), Some(&Value::String("cargo test".into())));
        assert_eq!(args.get("timeout"), Some(&Value::Number(30.into())));
        assert_eq!(args.get("flag"), Some(&Value::Bool(true)));
    }

    #[test]
    fn parses_empty_object() {
        let args = parse_args("{}").expect("empty object should be valid");
        assert!(args.is_empty());
    }

    #[test]
    fn rejects_non_object_json() {
        let res = parse_args("[1, 2, 3]");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::InvalidData);

        let res = parse_args("\"plain string\"");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_malformed_json() {
        let res = parse_args(r#"{"unclosed": "#);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn parses_from_reader() {
        let cursor = Cursor::new(r#"{"path": "/tmp/test.txt"}"#.as_bytes());
        let args = parse_args_from_reader(cursor).expect("should parse from reader");
        assert_eq!(
            args.get("path"),
            Some(&Value::String("/tmp/test.txt".into()))
        );
    }

    struct FailingReader;
    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "read failed"))
        }
    }

    #[test]
    fn handles_reader_io_error() {
        let res = parse_args_from_reader(FailingReader);
        assert!(res.is_err());
    }

    #[test]
    fn helper_getters_extract_typed_values() {
        let json = r#"{"path": "src/main.rs", "start_line": 10, "is_active": true}"#;
        let args = parse_args(json).expect("should parse");

        assert_eq!(get_str(&args, "path").unwrap(), "src/main.rs");
        assert_eq!(get_opt_usize(&args, "start_line").unwrap(), Some(10));
        assert_eq!(get_opt_usize(&args, "end_line").unwrap(), None);
        assert_eq!(get_bool(&args, "is_active").unwrap(), true);
    }

    #[test]
    fn helper_getters_validate_types() {
        let json = r#"{"path": 123, "start_line": "not_a_number"}"#;
        let args = parse_args(json).expect("should parse");

        let path_err = get_str(&args, "path").unwrap_err();
        assert_eq!(path_err.kind(), io::ErrorKind::InvalidInput);

        let line_err = get_opt_usize(&args, "start_line").unwrap_err();
        assert_eq!(line_err.kind(), io::ErrorKind::InvalidInput);

        let missing_err = get_str(&args, "missing").unwrap_err();
        assert_eq!(missing_err.kind(), io::ErrorKind::InvalidInput);
    }
}

