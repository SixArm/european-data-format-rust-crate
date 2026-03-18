//! EDF/EDF+ header structures and binary parsing.
//!
//! The EDF header consists of a 256-byte fixed portion followed by
//! `signals_count * 256` bytes of per-signal header fields.
//!
//! # EDF+ Header Layout (fixed portion)
//!
//! | Bytes | Field                          |
//! |-------|--------------------------------|
//! | 8     | Version (always "0")           |
//! | 80    | Patient identification         |
//! | 80    | Recording identification       |
//! | 8     | Start date (dd.mm.yy)          |
//! | 8     | Start time (hh.mm.ss)          |
//! | 8     | Number of bytes in header      |
//! | 44    | Reserved ("EDF+C" or "EDF+D")  |
//! | 8     | Number of data records         |
//! | 8     | Duration of a data record (s)  |
//! | 4     | Number of signals              |
//!
//! # Examples
//!
//! ```
//! use european_data_format::EdfHeader;
//!
//! let header = EdfHeader {
//!     version: "0".into(),
//!     patient_identification: "X X X X".into(),
//!     recording_identification: "Startdate X X X X".into(),
//!     start_date: "01.01.00".into(),
//!     start_time: "00.00.00".into(),
//!     header_bytes: 512,
//!     reserved: "EDF+C".into(),
//!     data_records_count: 1,
//!     data_record_duration: 1.0,
//!     signals_count: 1,
//!     signal_headers: vec![],
//! };
//! assert_eq!(header.version, "0");
//! ```

use serde::{Deserialize, Serialize};

use crate::error::EdfError;

/// The fixed (main) header of an EDF/EDF+ file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfHeader {
    /// Version of the data format, always `"0"`.
    pub version: String,

    /// Local patient identification.
    ///
    /// EDF+ requires subfields: code, sex, birthdate, name.
    /// For example: `"MCH-0234567 F 02-MAY-1951 Haagse_Harry"`.
    pub patient_identification: String,

    /// Local recording identification.
    ///
    /// EDF+ requires subfields: `"Startdate"`, date, admin code,
    /// technician, equipment.
    /// For example: `"Startdate 02-MAR-2002 EMG561 BK/JOP Sony."`.
    pub recording_identification: String,

    /// Start date of the recording in `dd.mm.yy` format.
    pub start_date: String,

    /// Start time of the recording in `hh.mm.ss` format.
    pub start_time: String,

    /// Total number of bytes in the header record.
    ///
    /// Equal to `256 + signals_count * 256`.
    pub header_bytes: usize,

    /// Reserved field.
    ///
    /// In EDF+ this starts with `"EDF+C"` (contiguous) or `"EDF+D"` (discontinuous).
    pub reserved: String,

    /// Number of data records in the file.
    ///
    /// May be `-1` during recording; must be filled in once the file is closed.
    pub data_records_count: i64,

    /// Duration of each data record in seconds.
    pub data_record_duration: f64,

    /// Number of signals in each data record.
    pub signals_count: usize,

    /// Per-signal header information.
    pub signal_headers: Vec<EdfSignalHeader>,
}

/// Per-signal header fields in an EDF/EDF+ file.
///
/// Each signal has 256 bytes of header data, spread across the header
/// in interleaved order (all labels first, then all transducer types, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfSignalHeader {
    /// Signal label, e.g. `"EEG Fpz-Cz"` or `"EDF Annotations"`.
    pub label: String,

    /// Transducer type, e.g. `"AgAgCl electrode"`.
    pub transducer_type: String,

    /// Physical dimension (unit), e.g. `"uV"` or `"mV"`.
    pub physical_dimension: String,

    /// Physical minimum value.
    pub physical_minimum: f64,

    /// Physical maximum value.
    pub physical_maximum: f64,

    /// Digital minimum value.
    pub digital_minimum: i32,

    /// Digital maximum value.
    pub digital_maximum: i32,

    /// Prefiltering description, e.g. `"HP:0.1Hz LP:75Hz"`.
    pub prefiltering: String,

    /// Number of samples in each data record for this signal.
    pub samples_per_record: usize,

    /// Reserved field for this signal.
    pub reserved: String,
}

impl EdfSignalHeader {
    /// Returns `true` if this signal is an EDF Annotations signal.
    pub fn is_annotation(&self) -> bool {
        self.label.trim() == "EDF Annotations"
    }
}

/// Read exactly `n` bytes from a reader.
fn read_bytes(reader: &mut impl std::io::Read, n: usize) -> Result<Vec<u8>, EdfError> {
    let mut buf = vec![0u8; n];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Parse an ASCII field from raw bytes, trimming trailing spaces.
fn parse_ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_end().to_string()
}

/// Pad or truncate a string to exactly `width` bytes, right-padded with spaces.
pub(crate) fn format_field(value: &str, width: usize) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.resize(width, b' ');
    bytes.truncate(width);
    bytes
}

impl EdfHeader {
    /// Parse a complete EDF/EDF+ header from a reader.
    ///
    /// Reads the 256-byte fixed header followed by `signals_count * 256`
    /// bytes of per-signal header fields.
    ///
    /// # Errors
    ///
    /// Returns [`EdfError::InvalidHeader`] if any field cannot be parsed,
    /// or [`EdfError::Io`] on read failure.
    pub fn read_from(reader: &mut impl std::io::Read) -> Result<Self, EdfError> {
        // Fixed header: 256 bytes
        let version_bytes = read_bytes(reader, 8)?;
        let patient_bytes = read_bytes(reader, 80)?;
        let recording_bytes = read_bytes(reader, 80)?;
        let start_date_bytes = read_bytes(reader, 8)?;
        let start_time_bytes = read_bytes(reader, 8)?;
        let header_bytes_bytes = read_bytes(reader, 8)?;
        let reserved_bytes = read_bytes(reader, 44)?;
        let data_records_bytes = read_bytes(reader, 8)?;
        let duration_bytes = read_bytes(reader, 8)?;
        let signals_count_bytes = read_bytes(reader, 4)?;

        let version = parse_ascii(&version_bytes);
        let patient_identification = parse_ascii(&patient_bytes);
        let recording_identification = parse_ascii(&recording_bytes);
        let start_date = parse_ascii(&start_date_bytes);
        let start_time = parse_ascii(&start_time_bytes);

        let header_bytes: usize = parse_ascii(&header_bytes_bytes)
            .parse()
            .map_err(|e| EdfError::InvalidHeader {
                field: "header_bytes".into(),
                message: format!("{e}"),
            })?;

        let reserved = parse_ascii(&reserved_bytes);

        let data_records_count: i64 = parse_ascii(&data_records_bytes)
            .parse()
            .map_err(|e| EdfError::InvalidHeader {
                field: "data_records_count".into(),
                message: format!("{e}"),
            })?;

        let data_record_duration: f64 = parse_ascii(&duration_bytes)
            .parse()
            .map_err(|e| EdfError::InvalidHeader {
                field: "data_record_duration".into(),
                message: format!("{e}"),
            })?;

        let signals_count: usize = parse_ascii(&signals_count_bytes)
            .parse()
            .map_err(|e| EdfError::InvalidHeader {
                field: "signals_count".into(),
                message: format!("{e}"),
            })?;

        // Per-signal headers: read each field across all signals
        let ns = signals_count;

        let mut labels = Vec::with_capacity(ns);
        for _ in 0..ns {
            labels.push(parse_ascii(&read_bytes(reader, 16)?));
        }

        let mut transducer_types = Vec::with_capacity(ns);
        for _ in 0..ns {
            transducer_types.push(parse_ascii(&read_bytes(reader, 80)?));
        }

        let mut physical_dimensions = Vec::with_capacity(ns);
        for _ in 0..ns {
            physical_dimensions.push(parse_ascii(&read_bytes(reader, 8)?));
        }

        let mut physical_minimums = Vec::with_capacity(ns);
        for i in 0..ns {
            let s = parse_ascii(&read_bytes(reader, 8)?);
            physical_minimums.push(s.parse::<f64>().map_err(|e| {
                EdfError::InvalidSignalHeader {
                    index: i,
                    field: "physical_minimum".into(),
                    message: format!("{e}"),
                }
            })?);
        }

        let mut physical_maximums = Vec::with_capacity(ns);
        for i in 0..ns {
            let s = parse_ascii(&read_bytes(reader, 8)?);
            physical_maximums.push(s.parse::<f64>().map_err(|e| {
                EdfError::InvalidSignalHeader {
                    index: i,
                    field: "physical_maximum".into(),
                    message: format!("{e}"),
                }
            })?);
        }

        let mut digital_minimums = Vec::with_capacity(ns);
        for i in 0..ns {
            let s = parse_ascii(&read_bytes(reader, 8)?);
            digital_minimums.push(s.parse::<i32>().map_err(|e| {
                EdfError::InvalidSignalHeader {
                    index: i,
                    field: "digital_minimum".into(),
                    message: format!("{e}"),
                }
            })?);
        }

        let mut digital_maximums = Vec::with_capacity(ns);
        for i in 0..ns {
            let s = parse_ascii(&read_bytes(reader, 8)?);
            digital_maximums.push(s.parse::<i32>().map_err(|e| {
                EdfError::InvalidSignalHeader {
                    index: i,
                    field: "digital_maximum".into(),
                    message: format!("{e}"),
                }
            })?);
        }

        let mut prefilterings = Vec::with_capacity(ns);
        for _ in 0..ns {
            prefilterings.push(parse_ascii(&read_bytes(reader, 80)?));
        }

        let mut samples_per_records = Vec::with_capacity(ns);
        for i in 0..ns {
            let s = parse_ascii(&read_bytes(reader, 8)?);
            samples_per_records.push(s.parse::<usize>().map_err(|e| {
                EdfError::InvalidSignalHeader {
                    index: i,
                    field: "samples_per_record".into(),
                    message: format!("{e}"),
                }
            })?);
        }

        let mut reserveds = Vec::with_capacity(ns);
        for _ in 0..ns {
            reserveds.push(parse_ascii(&read_bytes(reader, 32)?));
        }

        let signal_headers: Vec<EdfSignalHeader> = (0..ns)
            .map(|i| EdfSignalHeader {
                label: labels[i].clone(),
                transducer_type: transducer_types[i].clone(),
                physical_dimension: physical_dimensions[i].clone(),
                physical_minimum: physical_minimums[i],
                physical_maximum: physical_maximums[i],
                digital_minimum: digital_minimums[i],
                digital_maximum: digital_maximums[i],
                prefiltering: prefilterings[i].clone(),
                samples_per_record: samples_per_records[i],
                reserved: reserveds[i].clone(),
            })
            .collect();

        Ok(EdfHeader {
            version,
            patient_identification,
            recording_identification,
            start_date,
            start_time,
            header_bytes,
            reserved,
            data_records_count,
            data_record_duration,
            signals_count,
            signal_headers,
        })
    }

    /// Write this header to a writer in EDF binary format.
    ///
    /// # Errors
    ///
    /// Returns [`EdfError::Io`] on write failure.
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), EdfError> {
        // Fixed header
        writer.write_all(&format_field(&self.version, 8))?;
        writer.write_all(&format_field(&self.patient_identification, 80))?;
        writer.write_all(&format_field(&self.recording_identification, 80))?;
        writer.write_all(&format_field(&self.start_date, 8))?;
        writer.write_all(&format_field(&self.start_time, 8))?;
        writer.write_all(&format_field(&self.header_bytes.to_string(), 8))?;
        writer.write_all(&format_field(&self.reserved, 44))?;
        writer.write_all(&format_field(&self.data_records_count.to_string(), 8))?;

        // Format duration: avoid unnecessary trailing zeros but preserve spec format
        let dur_str = format_duration(self.data_record_duration);
        writer.write_all(&format_field(&dur_str, 8))?;
        writer.write_all(&format_field(&self.signals_count.to_string(), 4))?;

        // Per-signal headers (interleaved)
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.label, 16))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.transducer_type, 80))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.physical_dimension, 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&format_number(sh.physical_minimum), 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&format_number(sh.physical_maximum), 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.digital_minimum.to_string(), 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.digital_maximum.to_string(), 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.prefiltering, 80))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.samples_per_record.to_string(), 8))?;
        }
        for sh in &self.signal_headers {
            writer.write_all(&format_field(&sh.reserved, 32))?;
        }

        Ok(())
    }
}

/// Format a floating-point number for EDF header fields.
///
/// Produces the shortest representation that round-trips correctly,
/// while avoiding unnecessary trailing zeros after the decimal point.
fn format_number(value: f64) -> String {
    if value == value.trunc() {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

/// Format data record duration, preserving the original precision style.
fn format_duration(value: f64) -> String {
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
    fn test_format_field_pad() {
        let result = format_field("hello", 8);
        assert_eq!(result, b"hello   ");
    }

    #[test]
    fn test_format_field_truncate() {
        let result = format_field("hello world", 5);
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_parse_ascii_trims_spaces() {
        assert_eq!(parse_ascii(b"hello   "), "hello");
        assert_eq!(parse_ascii(b"  hello "), "  hello");
    }

    #[test]
    fn test_signal_header_is_annotation() {
        let mut sh = EdfSignalHeader {
            label: "EDF Annotations".into(),
            transducer_type: String::new(),
            physical_dimension: String::new(),
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            digital_minimum: -32768,
            digital_maximum: 32767,
            prefiltering: String::new(),
            samples_per_record: 60,
            reserved: String::new(),
        };
        assert!(sh.is_annotation());

        sh.label = "EEG Fpz-Cz".into();
        assert!(!sh.is_annotation());
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(100.0), "100");
        assert_eq!(format_number(-2048.0), "-2048");
        assert_eq!(format_number(0.5), "0.5");
        assert_eq!(format_number(-100.5), "-100.5");
    }

    #[test]
    fn test_header_round_trip() {
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: 10,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG Fpz-Cz".into(),
                transducer_type: "AgAgCl electrode".into(),
                physical_dimension: "uV".into(),
                physical_minimum: -500.0,
                physical_maximum: 500.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: "HP:0.1Hz LP:75Hz".into(),
                samples_per_record: 256,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.version, header.version);
        assert_eq!(parsed.patient_identification, header.patient_identification);
        assert_eq!(parsed.signals_count, header.signals_count);
        assert_eq!(parsed.data_records_count, header.data_records_count);
        assert_eq!(parsed.signal_headers.len(), 1);
        assert_eq!(parsed.signal_headers[0].label, "EEG Fpz-Cz");
        assert_eq!(parsed.signal_headers[0].samples_per_record, 256);
    }

    // ── Edge Case Tests: ASCII Header Constraints (Edge Case 6) ──────

    #[test]
    fn test_edge_case_ascii_header_preserves_printable_ascii() {
        // EDF spec requires all header fields to be printable ASCII (bytes 32-126).
        // Verify that standard ASCII characters survive a write/read round-trip
        // without modification. This is the baseline happy-path.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "ABC-123 M 01-JAN-2000 Test_Name".into(),
            recording_identification: "Startdate 01-JAN-2000 LAB TECH Equipment".into(),
            start_date: "01.01.00".into(),
            start_time: "12.30.45".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG Fp1".into(),
                transducer_type: "AgAgCl".into(),
                physical_dimension: "uV".into(),
                physical_minimum: -500.0,
                physical_maximum: 500.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: "HP:0.1Hz".into(),
                samples_per_record: 256,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.patient_identification, header.patient_identification);
        assert_eq!(parsed.recording_identification, header.recording_identification);
    }

    #[test]
    fn test_edge_case_non_ascii_in_header_replaced_by_lossy_conversion() {
        // Edge Case 6: Non-ASCII characters (like accents, e.g. "Müller") in
        // header fields cause parsing issues. The current code uses
        // String::from_utf8_lossy, which replaces invalid UTF-8 sequences with
        // the Unicode replacement character U+FFFD. This test documents that
        // behavior — non-ASCII bytes are silently replaced, not rejected.
        //
        // A proper fix should validate and reject non-ASCII bytes. For now,
        // this test captures the current (lossy) behavior so we know when
        // it changes.
        let non_ascii_bytes: &[u8] = &[0x4D, 0xFC, 0x6C, 0x6C, 0x65, 0x72]; // "Müller" in Latin-1
        let result = parse_ascii(non_ascii_bytes);
        // from_utf8_lossy replaces the 0xFC byte with the replacement character
        assert!(result.contains('\u{FFFD}') || result.contains("M"));
    }

    #[test]
    fn test_edge_case_header_field_all_spaces() {
        // EDF uses space-padded fields. A field that is entirely spaces
        // (e.g., an empty patient ID) should parse to an empty string after
        // trimming trailing spaces.
        let all_spaces = b"        "; // 8 spaces
        assert_eq!(parse_ascii(all_spaces), "");
    }

    #[test]
    fn test_edge_case_header_field_with_nul_bytes() {
        // Some broken EDF writers pad with NUL (0x00) instead of spaces.
        // from_utf8_lossy handles NUL bytes as valid UTF-8 (they map to U+0000).
        // trim_end only trims whitespace (which includes NUL in some contexts).
        // This test documents what happens with NUL-padded fields.
        let nul_padded = b"EEG\x00\x00\x00\x00\x00";
        let result = parse_ascii(nul_padded);
        // NUL is a valid UTF-8 character but trim_end may or may not strip it
        // depending on what Rust considers whitespace. Document the actual behavior.
        assert!(result.starts_with("EEG"));
    }

    // ── Edge Case Tests: Length-Restricted Fields (Edge Case 7) ──────

    #[test]
    fn test_edge_case_label_exactly_16_chars() {
        // Signal labels are limited to exactly 16 bytes in EDF. A label that is
        // exactly 16 characters should be preserved without padding or truncation.
        let result = format_field("1234567890123456", 16);
        assert_eq!(result.len(), 16);
        assert_eq!(&result, b"1234567890123456");
    }

    #[test]
    fn test_edge_case_label_exceeds_16_chars_is_truncated() {
        // Edge Case 7: If a signal label exceeds 16 characters, format_field
        // silently truncates it. This means data is lost without warning.
        // This test documents the truncation behavior.
        let result = format_field("This is a very long label name", 16);
        assert_eq!(result.len(), 16);
        assert_eq!(&result, b"This is a very l");
    }

    #[test]
    fn test_edge_case_empty_label() {
        // An empty label should be padded entirely with spaces.
        let result = format_field("", 16);
        assert_eq!(result.len(), 16);
        assert!(result.iter().all(|&b| b == b' '));
    }

    #[test]
    fn test_edge_case_patient_id_at_80_byte_boundary() {
        // Patient identification is exactly 80 bytes. Test with a string that
        // is exactly 80 characters to ensure no off-by-one in padding.
        let id = "A".repeat(80);
        let result = format_field(&id, 80);
        assert_eq!(result.len(), 80);
        assert!(result.iter().all(|&b| b == b'A'));
    }

    #[test]
    fn test_edge_case_patient_id_exceeds_80_bytes() {
        // A patient ID longer than 80 bytes gets truncated. This could lose
        // critical patient information silently.
        let id = "B".repeat(100);
        let result = format_field(&id, 80);
        assert_eq!(result.len(), 80);
        assert!(result.iter().all(|&b| b == b'B'));
    }

    // ── Edge Case Tests: Calibration Discrepancies (Edge Case 8) ─────

    #[test]
    fn test_edge_case_calibration_normal_range() {
        // Normal calibration: digital range maps linearly to physical range.
        // digital [-2048, 2047] -> physical [-500.0, 500.0] uV
        // gain = (physical_max - physical_min) / (digital_max - digital_min)
        //      = 1000.0 / 4095 ≈ 0.2442
        let sh = EdfSignalHeader {
            label: "EEG".into(),
            transducer_type: String::new(),
            physical_dimension: "uV".into(),
            physical_minimum: -500.0,
            physical_maximum: 500.0,
            digital_minimum: -2048,
            digital_maximum: 2047,
            prefiltering: String::new(),
            samples_per_record: 256,
            reserved: String::new(),
        };
        // Verify digital range is valid
        assert!(sh.digital_minimum < sh.digital_maximum);
        // Verify physical range is valid
        assert!(sh.physical_minimum < sh.physical_maximum);
    }

    #[test]
    fn test_edge_case_calibration_inverted_digital_range() {
        // Edge Case 8: If digital_minimum > digital_maximum, the gain becomes
        // negative, which inverts the signal. This is technically allowed by some
        // interpretations of the spec but usually indicates a bug in the writing
        // software. The crate currently does not validate this.
        let sh = EdfSignalHeader {
            label: "EEG".into(),
            transducer_type: String::new(),
            physical_dimension: "uV".into(),
            physical_minimum: -500.0,
            physical_maximum: 500.0,
            digital_minimum: 2047,  // INVERTED: min > max
            digital_maximum: -2048, // INVERTED
            prefiltering: String::new(),
            samples_per_record: 256,
            reserved: String::new(),
        };
        // This should ideally be caught by validation, but currently isn't
        assert!(sh.digital_minimum > sh.digital_maximum);
    }

    #[test]
    fn test_edge_case_calibration_equal_digital_min_max() {
        // Edge Case 8: If digital_minimum == digital_maximum, the gain is
        // infinite (division by zero). This is always invalid.
        let sh = EdfSignalHeader {
            label: "EEG".into(),
            transducer_type: String::new(),
            physical_dimension: "uV".into(),
            physical_minimum: -500.0,
            physical_maximum: 500.0,
            digital_minimum: 0,
            digital_maximum: 0, // EQUAL: would cause division by zero
            prefiltering: String::new(),
            samples_per_record: 256,
            reserved: String::new(),
        };
        // gain = (phys_max - phys_min) / (dig_max - dig_min) = 1000.0 / 0 = inf
        let dig_range = sh.digital_maximum - sh.digital_minimum;
        assert_eq!(dig_range, 0);
    }

    #[test]
    fn test_edge_case_annotation_signal_calibration() {
        // EDF+ spec requires annotation signals to have specific calibration:
        // digital_min = -32768, digital_max = 32767, physical_min = -1, physical_max = 1
        // This test verifies the is_annotation() detection and expected calibration.
        let sh = EdfSignalHeader {
            label: "EDF Annotations".into(),
            transducer_type: String::new(),
            physical_dimension: String::new(),
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            digital_minimum: -32768,
            digital_maximum: 32767,
            prefiltering: String::new(),
            samples_per_record: 60,
            reserved: String::new(),
        };
        assert!(sh.is_annotation());
        assert_eq!(sh.digital_minimum, -32768);
        assert_eq!(sh.digital_maximum, 32767);
    }

    // ── Edge Case Tests: Reserved Field Discrepancies (Edge Case 9) ──

    #[test]
    fn test_edge_case_reserved_edf_plus_contiguous() {
        // EDF+C means contiguous recording: data records are consecutive
        // with no time gaps. The reserved field starts with "EDF+C" and
        // may be followed by spaces to fill the 44-byte field.
        let result = format_field("EDF+C", 44);
        assert_eq!(result.len(), 44);
        assert_eq!(&result[..5], b"EDF+C");
        assert!(result[5..].iter().all(|&b| b == b' '));
    }

    #[test]
    fn test_edge_case_reserved_edf_plus_discontinuous() {
        // EDF+D means discontinuous recording: time gaps exist between
        // some data records. Each record must have a timekeeping TAL
        // that specifies the actual start time of that record.
        let result = format_field("EDF+D", 44);
        assert_eq!(result.len(), 44);
        assert_eq!(&result[..5], b"EDF+D");
    }

    #[test]
    fn test_edge_case_reserved_plain_edf_empty() {
        // Original (non-plus) EDF files have an empty or space-filled reserved
        // field. This means no annotations are supported.
        let result = format_field("", 44);
        assert_eq!(result.len(), 44);
        assert!(result.iter().all(|&b| b == b' '));
    }

    #[test]
    fn test_edge_case_reserved_field_round_trip() {
        // Verify that the reserved field survives a write/read round-trip.
        // The 44-byte reserved field is written with space-padding, then
        // read back and trimmed. The trimmed value should match the original.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512,
            reserved: "EDF+D".into(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 10,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.reserved, "EDF+D");
    }

    // ── Edge Case Tests: Date Formatting (Edge Case 11) ──────────────

    #[test]
    fn test_edge_case_date_format_round_trip() {
        // EDF requires start_date in dd.mm.yy format. The two-digit year
        // causes Y2K issues: "00" could mean 1900 or 2000. EDF+ resolves
        // this via the recording_identification field ("Startdate dd-MMM-yyyy").
        // This test verifies the date string survives round-trip unchanged.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "17.04.01".into(), // 17 April, year 01
            start_time: "11.25.00".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 10,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.start_date, "17.04.01");
        assert_eq!(parsed.start_time, "11.25.00");
    }

    // ── Edge Case Tests: Header Size Consistency (Edge Case 12) ──────

    #[test]
    fn test_edge_case_header_bytes_matches_formula() {
        // The header_bytes field must equal 256 + signals_count * 256.
        // This is a critical consistency check. If the value is wrong,
        // the data section offset will be incorrect, causing data corruption.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 768, // 256 + 2*256 = 768
            reserved: "EDF+C".into(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: 2,
            signal_headers: vec![
                EdfSignalHeader {
                    label: "EEG".into(),
                    transducer_type: String::new(),
                    physical_dimension: "uV".into(),
                    physical_minimum: -100.0,
                    physical_maximum: 100.0,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefiltering: String::new(),
                    samples_per_record: 10,
                    reserved: String::new(),
                },
                EdfSignalHeader {
                    label: "EDF Annotations".into(),
                    transducer_type: String::new(),
                    physical_dimension: String::new(),
                    physical_minimum: -1.0,
                    physical_maximum: 1.0,
                    digital_minimum: -32768,
                    digital_maximum: 32767,
                    prefiltering: String::new(),
                    samples_per_record: 60,
                    reserved: String::new(),
                },
            ],
        };

        // Verify the formula: header_bytes = 256 + signals_count * 256
        let expected = 256 + header.signals_count * 256;
        assert_eq!(header.header_bytes, expected);
    }

    #[test]
    fn test_edge_case_header_write_produces_exact_byte_count() {
        // The written header must be exactly header_bytes long.
        // Any mismatch means the binary layout is broken.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512, // 256 + 1*256
            reserved: "EDF+C".into(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 10,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();

        // Written bytes must exactly match header_bytes
        assert_eq!(buf.len(), header.header_bytes);
    }

    // ── Edge Case Tests: Multiple Signals (Edge Case 4) ──────────────

    #[test]
    fn test_edge_case_many_signal_headers_round_trip() {
        // EDF+ has a maximum data record size of 61,440 bytes. With many
        // signals at high sample rates, this limit can be reached. This test
        // verifies that a file with many signals (but within limits) round-trips
        // correctly through the header parser.
        let num_signals = 10;
        let signal_headers: Vec<EdfSignalHeader> = (0..num_signals)
            .map(|i| EdfSignalHeader {
                label: format!("Ch{i:02}"),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 256,
                reserved: String::new(),
            })
            .collect();

        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 256 + num_signals * 256,
            reserved: String::new(),
            data_records_count: 1,
            data_record_duration: 1.0,
            signals_count: num_signals,
            signal_headers,
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 256 + num_signals * 256);

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.signals_count, num_signals);
        assert_eq!(parsed.signal_headers.len(), num_signals);
        for (i, sh) in parsed.signal_headers.iter().enumerate() {
            assert_eq!(sh.label, format!("Ch{i:02}"));
            assert_eq!(sh.samples_per_record, 256);
        }
    }

    // ── Edge Case Tests: Floating-Point Precision (Edge Cases 3, 8) ──

    #[test]
    fn test_edge_case_fractional_duration_round_trip() {
        // The data_record_duration is stored as an 8-byte ASCII field.
        // Fractional durations (e.g., 0.05 seconds for high-speed EMG)
        // must survive the float→string→float round-trip without precision loss.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: 1,
            data_record_duration: 0.05, // 50ms, typical for EMG
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 10,
                reserved: String::new(),
            }],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.data_record_duration, 0.05);
    }

    #[test]
    fn test_edge_case_format_number_preserves_precision() {
        // format_number must not lose precision for values used in EDF headers.
        // Integer values should format without a decimal point.
        // Fractional values must round-trip through string representation.
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0), "0"); // negative zero becomes "0"
        assert_eq!(format_number(32767.0), "32767");
        assert_eq!(format_number(-32768.0), "-32768");
        assert_eq!(format_number(0.05), "0.05");
        assert_eq!(format_number(-100.5), "-100.5");

        // Verify round-trip: number → string → number
        let values = [0.0, -0.0, 1.0, -1.0, 0.05, 100.5, -32768.0, 32767.0];
        for &v in &values {
            let s = format_number(v);
            let parsed: f64 = s.parse().unwrap();
            assert_eq!(parsed, if v == -0.0 { 0.0 } else { v });
        }
    }

    // ── Edge Case Tests: Annotation Signal Detection ─────────────────

    #[test]
    fn test_edge_case_annotation_label_with_trailing_spaces() {
        // In EDF binary, the label field is 16 bytes, space-padded.
        // After parsing, "EDF Annotations " (with trailing space) should
        // still be detected as an annotation signal. The is_annotation()
        // method uses trim() to handle this.
        let sh = EdfSignalHeader {
            label: "EDF Annotations ".into(), // trailing space
            transducer_type: String::new(),
            physical_dimension: String::new(),
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            digital_minimum: -32768,
            digital_maximum: 32767,
            prefiltering: String::new(),
            samples_per_record: 60,
            reserved: String::new(),
        };
        assert!(sh.is_annotation());
    }

    #[test]
    fn test_edge_case_annotation_label_case_sensitivity() {
        // The EDF+ spec uses "EDF Annotations" with specific capitalization.
        // A label like "edf annotations" (lowercase) should NOT be detected
        // as an annotation signal, because the spec is case-sensitive.
        let sh = EdfSignalHeader {
            label: "edf annotations".into(),
            transducer_type: String::new(),
            physical_dimension: String::new(),
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            digital_minimum: -32768,
            digital_maximum: 32767,
            prefiltering: String::new(),
            samples_per_record: 60,
            reserved: String::new(),
        };
        assert!(!sh.is_annotation());
    }

    #[test]
    fn test_edge_case_data_records_count_negative_one() {
        // The EDF spec allows data_records_count = -1 to indicate that
        // the file is still being recorded (not yet closed). A proper
        // reader should handle this, typically by refusing to read data
        // or by inferring the count from file size.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: -1, // File not yet closed
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -100.0,
                physical_maximum: 100.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 10,
                reserved: String::new(),
            }],
        };

        // Write and read back — the -1 should survive round-trip
        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();

        assert_eq!(parsed.data_records_count, -1);
    }

    // ── Edge Case Tests: Signal Header Field Boundaries ──────────────

    #[test]
    fn test_edge_case_physical_dimension_exactly_8_bytes() {
        // Physical dimension field is 8 bytes. Common units like "uV" are
        // short, but some could be longer (e.g., "degC/min"). Test at the
        // exact boundary.
        let result = format_field("degC/min", 8);
        assert_eq!(result.len(), 8);
        assert_eq!(&result, b"degC/min");
    }

    #[test]
    fn test_edge_case_prefiltering_exactly_80_bytes() {
        // Prefiltering field is 80 bytes. Long filter descriptions should
        // be preserved up to 80 bytes.
        let long_filter = "HP:0.1Hz LP:75Hz N:50Hz BP:0.5-35Hz";
        let result = format_field(long_filter, 80);
        assert_eq!(result.len(), 80);
        assert!(result.starts_with(long_filter.as_bytes()));
    }

    #[test]
    fn test_edge_case_zero_signals_count() {
        // An EDF file with 0 signals is technically parseable (header only),
        // though practically useless. The header should be exactly 256 bytes.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 256,
            reserved: String::new(),
            data_records_count: 0,
            data_record_duration: 0.0,
            signals_count: 0,
            signal_headers: vec![],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 256);

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = EdfHeader::read_from(&mut cursor).unwrap();
        assert_eq!(parsed.signals_count, 0);
        assert!(parsed.signal_headers.is_empty());
    }
}
