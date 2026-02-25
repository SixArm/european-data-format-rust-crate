//! EDF+ annotation and TAL (Time-stamped Annotation List) parsing.
//!
//! An EDF+ file may contain one or more "EDF Annotations" signals whose
//! bytes encode Time-stamped Annotation Lists (TALs). Each TAL has:
//!
//! - An **onset** time (seconds relative to file start), preceded by `+` or `-`.
//! - An optional **duration** in seconds, preceded by byte `0x15` (NAK).
//! - Zero or more **annotation texts**, each followed by byte `0x14` (DC4).
//! - A terminating `0x00` byte.
//!
//! The first TAL in each data record must have an empty annotation text
//! that serves as the time-keeping annotation for that data record.
//!
//! # TAL Format
//!
//! ```text
//! +Onset\x15Duration\x14Annotation1\x14Annotation2\x14\x00
//! ```
//!
//! # Examples
//!
//! ```
//! use european_data_format::EdfAnnotation;
//!
//! let ann = EdfAnnotation {
//!     onset: 180.0,
//!     duration: None,
//!     texts: vec!["Lights off".into(), "Close door".into()],
//! };
//! assert_eq!(ann.onset, 180.0);
//! assert_eq!(ann.texts.len(), 2);
//! ```

use serde::{Deserialize, Serialize};

use crate::error::EdfError;

/// Byte value 0x14 (DC4): separates annotations within a TAL.
const SEPARATOR: u8 = 0x14;

/// Byte value 0x15 (NAK): separates onset from duration within a TAL.
const DURATION_SEP: u8 = 0x15;

/// Byte value 0x00 (NUL): terminates a TAL.
const TERMINATOR: u8 = 0x00;

/// A single EDF+ annotation extracted from a TAL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfAnnotation {
    /// Onset time in seconds relative to the file start.
    ///
    /// Positive values follow the file start; negative values precede it.
    pub onset: f64,

    /// Optional duration of the annotated event in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,

    /// The annotation texts. May be empty for time-keeping TALs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<String>,
}

/// Parse all TALs from the raw bytes of an "EDF Annotations" signal
/// in a single data record.
///
/// The bytes may contain multiple TALs concatenated, each terminated
/// by a `0x00` byte. Unused trailing bytes are also `0x00`.
///
/// # Errors
///
/// Returns [`EdfError::InvalidAnnotation`] if a TAL cannot be parsed.
///
/// # Examples
///
/// ```
/// use european_data_format::annotation::parse_tals;
///
/// let bytes = b"+0\x14\x14\x00";
/// let tals = parse_tals(bytes).unwrap();
/// assert_eq!(tals.len(), 1);
/// assert_eq!(tals[0].onset, 0.0);
/// assert!(tals[0].texts.is_empty());
/// ```
pub fn parse_tals(bytes: &[u8]) -> Result<Vec<EdfAnnotation>, EdfError> {
    let mut annotations = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        // Skip NUL padding at the end
        if bytes[pos] == TERMINATOR {
            pos += 1;
            continue;
        }

        // Find the end of this TAL (the NUL terminator)
        let tal_end = bytes[pos..]
            .iter()
            .position(|&b| b == TERMINATOR)
            .map(|i| pos + i)
            .unwrap_or(bytes.len());

        let tal_bytes = &bytes[pos..tal_end];
        if !tal_bytes.is_empty() {
            let ann = parse_single_tal(tal_bytes)?;
            annotations.push(ann);
        }

        pos = tal_end + 1; // skip the NUL terminator
    }

    Ok(annotations)
}

/// Parse a single TAL (without the trailing NUL byte).
fn parse_single_tal(bytes: &[u8]) -> Result<EdfAnnotation, EdfError> {
    // Split on 0x14 (SEPARATOR) to get time stamp and annotations
    let parts: Vec<&[u8]> = split_on(bytes, SEPARATOR);

    if parts.is_empty() {
        return Err(EdfError::InvalidAnnotation {
            message: "empty TAL".into(),
        });
    }

    // First part is the time stamp: Onset or Onset\x15Duration
    let timestamp_bytes = parts[0];
    let (onset, duration) = parse_timestamp(timestamp_bytes)?;

    // Remaining non-empty parts are annotation texts
    let texts: Vec<String> = parts[1..]
        .iter()
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).to_string())
        .collect();

    Ok(EdfAnnotation {
        onset,
        duration,
        texts,
    })
}

/// Parse the timestamp portion of a TAL: `Onset` or `Onset\x15Duration`.
fn parse_timestamp(bytes: &[u8]) -> Result<(f64, Option<f64>), EdfError> {
    // Split on 0x15 (DURATION_SEP)
    if let Some(sep_pos) = bytes.iter().position(|&b| b == DURATION_SEP) {
        let onset_str = std::str::from_utf8(&bytes[..sep_pos]).map_err(|_| {
            EdfError::InvalidAnnotation {
                message: "onset is not valid UTF-8".into(),
            }
        })?;
        let duration_str = std::str::from_utf8(&bytes[sep_pos + 1..]).map_err(|_| {
            EdfError::InvalidAnnotation {
                message: "duration is not valid UTF-8".into(),
            }
        })?;

        let onset = parse_onset(onset_str)?;
        let duration: f64 =
            duration_str
                .parse()
                .map_err(|e| EdfError::InvalidAnnotation {
                    message: format!("invalid duration '{duration_str}': {e}"),
                })?;

        Ok((onset, Some(duration)))
    } else {
        let onset_str =
            std::str::from_utf8(bytes).map_err(|_| EdfError::InvalidAnnotation {
                message: "onset is not valid UTF-8".into(),
            })?;
        let onset = parse_onset(onset_str)?;
        Ok((onset, None))
    }
}

/// Parse the onset value (must start with `+` or `-`).
fn parse_onset(s: &str) -> Result<f64, EdfError> {
    if s.is_empty() || (!s.starts_with('+') && !s.starts_with('-')) {
        return Err(EdfError::InvalidAnnotation {
            message: format!("onset must start with '+' or '-', got '{s}'"),
        });
    }
    s.parse().map_err(|e| EdfError::InvalidAnnotation {
        message: format!("invalid onset '{s}': {e}"),
    })
}

/// Split a byte slice on a delimiter byte, returning all parts.
fn split_on(bytes: &[u8], delimiter: u8) -> Vec<&[u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == delimiter {
            parts.push(&bytes[start..i]);
            start = i + 1;
        }
    }
    if start <= bytes.len() {
        parts.push(&bytes[start..]);
    }
    parts
}

/// Encode annotations into the raw bytes for an "EDF Annotations" signal
/// within a single data record.
///
/// The `total_bytes` parameter specifies the total number of bytes
/// available for annotations in this data record (`samples_per_record * 2`).
/// The output is padded with NUL bytes to fill the available space.
///
/// # Examples
///
/// ```
/// use european_data_format::annotation::{encode_tals, parse_tals};
/// use european_data_format::EdfAnnotation;
///
/// let annotations = vec![EdfAnnotation {
///     onset: 0.0,
///     duration: None,
///     texts: vec![],
/// }];
/// let bytes = encode_tals(&annotations, 20);
/// let parsed = parse_tals(&bytes).unwrap();
/// assert_eq!(parsed.len(), 1);
/// assert_eq!(parsed[0].onset, 0.0);
/// ```
pub fn encode_tals(annotations: &[EdfAnnotation], total_bytes: usize) -> Vec<u8> {
    let mut buf = Vec::new();

    for ann in annotations {
        // Onset
        if ann.onset >= 0.0 {
            buf.push(b'+');
            buf.extend_from_slice(format_onset(ann.onset).as_bytes());
        } else {
            buf.extend_from_slice(format_onset(ann.onset).as_bytes());
        }

        // Duration
        if let Some(dur) = ann.duration {
            buf.push(DURATION_SEP);
            buf.extend_from_slice(format_onset(dur).as_bytes());
        }

        // First separator (always present after onset/duration)
        buf.push(SEPARATOR);

        // Annotation texts
        for text in &ann.texts {
            buf.extend_from_slice(text.as_bytes());
            buf.push(SEPARATOR);
        }

        // TAL terminator
        buf.push(TERMINATOR);
    }

    // Pad with NUL bytes
    buf.resize(total_bytes, TERMINATOR);
    buf
}

/// Format a number for onset/duration, avoiding unnecessary trailing zeros.
fn format_onset(value: f64) -> String {
    if value == value.trunc() {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timekeeping_tal() {
        // +0\x14\x14\x00 — time-keeping annotation at onset 0
        let bytes = b"+0\x14\x14\x00";
        let tals = parse_tals(bytes).unwrap();
        assert_eq!(tals.len(), 1);
        assert_eq!(tals[0].onset, 0.0);
        assert_eq!(tals[0].duration, None);
        assert!(tals[0].texts.is_empty());
    }

    #[test]
    fn test_parse_tal_with_annotations() {
        // +180\x14Lights off\x14Close door\x14\x00
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"+180");
        bytes.push(SEPARATOR);
        bytes.extend_from_slice(b"Lights off");
        bytes.push(SEPARATOR);
        bytes.extend_from_slice(b"Close door");
        bytes.push(SEPARATOR);
        bytes.push(TERMINATOR);

        let tals = parse_tals(&bytes).unwrap();
        assert_eq!(tals.len(), 1);
        assert_eq!(tals[0].onset, 180.0);
        assert_eq!(tals[0].duration, None);
        assert_eq!(tals[0].texts, vec!["Lights off", "Close door"]);
    }

    #[test]
    fn test_parse_tal_with_duration() {
        // +1800.2\x1525.5\x14Apnea\x14\x00
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"+1800.2");
        bytes.push(DURATION_SEP);
        bytes.extend_from_slice(b"25.5");
        bytes.push(SEPARATOR);
        bytes.extend_from_slice(b"Apnea");
        bytes.push(SEPARATOR);
        bytes.push(TERMINATOR);

        let tals = parse_tals(&bytes).unwrap();
        assert_eq!(tals.len(), 1);
        assert_eq!(tals[0].onset, 1800.2);
        assert_eq!(tals[0].duration, Some(25.5));
        assert_eq!(tals[0].texts, vec!["Apnea"]);
    }

    #[test]
    fn test_parse_negative_onset() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"-0.065");
        bytes.push(SEPARATOR);
        bytes.extend_from_slice(b"Pre-stimulus beep 1000Hz");
        bytes.push(SEPARATOR);
        bytes.push(TERMINATOR);

        let tals = parse_tals(&bytes).unwrap();
        assert_eq!(tals.len(), 1);
        assert_eq!(tals[0].onset, -0.065);
    }

    #[test]
    fn test_parse_multiple_tals() {
        let mut bytes = Vec::new();
        // First TAL
        bytes.extend_from_slice(b"+0");
        bytes.push(SEPARATOR);
        bytes.push(SEPARATOR);
        bytes.push(TERMINATOR);
        // Second TAL
        bytes.extend_from_slice(b"+10");
        bytes.push(SEPARATOR);
        bytes.extend_from_slice(b"Event");
        bytes.push(SEPARATOR);
        bytes.push(TERMINATOR);
        // Padding
        bytes.push(TERMINATOR);
        bytes.push(TERMINATOR);

        let tals = parse_tals(&bytes).unwrap();
        assert_eq!(tals.len(), 2);
        assert_eq!(tals[0].onset, 0.0);
        assert!(tals[0].texts.is_empty());
        assert_eq!(tals[1].onset, 10.0);
        assert_eq!(tals[1].texts, vec!["Event"]);
    }

    #[test]
    fn test_parse_invalid_onset() {
        let bytes = b"180\x14\x14\x00"; // missing + or -
        let result = parse_tals(bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_tals_round_trip() {
        let annotations = vec![
            EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            },
            EdfAnnotation {
                onset: 10.0,
                duration: Some(5.0),
                texts: vec!["Test event".into()],
            },
        ];

        let encoded = encode_tals(&annotations, 100);
        let parsed = parse_tals(&encoded).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].onset, 0.0);
        assert!(parsed[0].texts.is_empty());
        assert_eq!(parsed[1].onset, 10.0);
        assert_eq!(parsed[1].duration, Some(5.0));
        assert_eq!(parsed[1].texts, vec!["Test event"]);
    }

    #[test]
    fn test_encode_padding() {
        let annotations = vec![EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        }];
        let encoded = encode_tals(&annotations, 20);
        assert_eq!(encoded.len(), 20);
        // Trailing bytes should be NUL
        assert!(encoded[5..].iter().all(|&b| b == 0));
    }
}
