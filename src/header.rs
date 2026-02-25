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
}
