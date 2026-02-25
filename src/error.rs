//! Error types for EDF/EDF+ parsing and serialization.
//!
//! This module defines [`EdfError`], an enum covering all error conditions
//! that can arise when reading, writing, or converting EDF/EDF+ files.
//!
//! # Examples
//!
//! ```
//! use european_data_format::EdfError;
//!
//! let err = EdfError::InvalidHeader {
//!     field: "version".into(),
//!     message: "expected '0'".into(),
//! };
//! assert!(err.to_string().contains("version"));
//! ```

/// Errors that can occur during EDF/EDF+ operations.
#[derive(Debug, thiserror::Error)]
pub enum EdfError {
    /// An I/O error occurred while reading or writing a file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A fixed header field contains an invalid value.
    #[error("Invalid header: {field}: {message}")]
    InvalidHeader {
        /// The name of the header field.
        field: String,
        /// A description of what went wrong.
        message: String,
    },

    /// A per-signal header field contains an invalid value.
    #[error("Invalid signal header at index {index}: {field}: {message}")]
    InvalidSignalHeader {
        /// Zero-based index of the signal.
        index: usize,
        /// The name of the signal header field.
        field: String,
        /// A description of what went wrong.
        message: String,
    },

    /// A data record contains invalid or unexpected data.
    #[error("Invalid data record at index {index}: {message}")]
    InvalidDataRecord {
        /// Zero-based index of the data record.
        index: usize,
        /// A description of what went wrong.
        message: String,
    },

    /// An annotation or TAL could not be parsed.
    #[error("Invalid annotation: {message}")]
    InvalidAnnotation {
        /// A description of what went wrong.
        message: String,
    },

    /// A generic parse error for field values.
    #[error("Parse error: {message}")]
    Parse {
        /// A description of what went wrong.
        message: String,
    },

    /// An error from XML deserialization.
    #[error("XML deserialization error: {0}")]
    XmlDe(#[from] quick_xml::DeError),

    /// An error from XML serialization.
    #[error("XML serialization error: {0}")]
    XmlSe(#[from] quick_xml::SeError),

    /// An error from the JSON serialization/deserialization layer.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = EdfError::Io(io_err);
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_invalid_header_display() {
        let err = EdfError::InvalidHeader {
            field: "version".into(),
            message: "expected '0'".into(),
        };
        assert!(err.to_string().contains("version"));
        assert!(err.to_string().contains("expected '0'"));
    }

    #[test]
    fn test_invalid_signal_header_display() {
        let err = EdfError::InvalidSignalHeader {
            index: 2,
            field: "label".into(),
            message: "empty".into(),
        };
        let s = err.to_string();
        assert!(s.contains("index 2"));
        assert!(s.contains("label"));
    }

    #[test]
    fn test_invalid_data_record_display() {
        let err = EdfError::InvalidDataRecord {
            index: 5,
            message: "truncated".into(),
        };
        assert!(err.to_string().contains("index 5"));
    }

    #[test]
    fn test_invalid_annotation_display() {
        let err = EdfError::InvalidAnnotation {
            message: "missing onset".into(),
        };
        assert!(err.to_string().contains("missing onset"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = EdfError::Parse {
            message: "not a number".into(),
        };
        assert!(err.to_string().contains("not a number"));
    }

    #[test]
    fn test_json_error_from() {
        let json_err = serde_json::from_str::<i32>("abc").unwrap_err();
        let err = EdfError::Json(json_err);
        assert!(err.to_string().contains("JSON"));
    }
}
