//! EDF/EDF+ binary file reading and writing.
//!
//! This module provides functions to read an EDF/EDF+ binary file into
//! an [`EdfFile`] structure and to write an [`EdfFile`] back to EDF binary.
//!
//! # EDF Binary Format
//!
//! An EDF file consists of:
//! 1. A 256-byte fixed header
//! 2. `signals_count * 256` bytes of per-signal header fields
//! 3. Data records, each containing `samples_per_record * 2` bytes per signal
//!
//! Samples are stored as little-endian 16-bit two's complement integers.
//!
//! # Examples
//!
//! ```no_run
//! use european_data_format::io_edf;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("recording.edf").unwrap();
//! let mut reader = BufReader::new(file);
//! let edf = io_edf::read_edf(&mut reader).unwrap();
//! println!("Patient: {}", edf.header.patient_identification);
//! ```

use std::io::{Read, Write};

use crate::annotation::{encode_tals, parse_tals, EdfAnnotation};
use crate::edf_file::{EdfDataRecord, EdfFile, EdfSignal};
use crate::error::EdfError;
use crate::header::EdfHeader;

/// Read an EDF/EDF+ file from a reader.
///
/// Parses the header, reads all data records, separates ordinary signals
/// from "EDF Annotations" signals, and parses annotations from the
/// annotation signals.
///
/// # Errors
///
/// Returns [`EdfError`] on I/O errors, invalid header fields, or
/// malformed data records.
pub fn read_edf(reader: &mut impl Read) -> Result<EdfFile, EdfError> {
    let header = EdfHeader::read_from(reader)?;

    let num_records = header.data_records_count;

    if num_records < 0 {
        return Err(EdfError::InvalidHeader {
            field: "data_records_count".into(),
            message: "data_records_count is -1 (file was not properly closed)".into(),
        });
    }
    let num_records = num_records as usize;

    // Identify annotation vs ordinary signals
    let is_annotation: Vec<bool> = header.signal_headers.iter().map(|s| s.is_annotation()).collect();

    // Initialize storage for ordinary signals
    let mut ordinary_records: Vec<Vec<EdfDataRecord>> = Vec::new();
    let mut ordinary_indices: Vec<usize> = Vec::new();
    for (i, _sh) in header.signal_headers.iter().enumerate() {
        if !is_annotation[i] {
            ordinary_indices.push(i);
            ordinary_records.push(Vec::with_capacity(num_records));
        }
    }

    // Read data records
    let mut all_annotations: Vec<EdfAnnotation> = Vec::new();

    for record_idx in 0..num_records {
        for (sig_idx, sh) in header.signal_headers.iter().enumerate() {
            let num_bytes = sh.samples_per_record * 2;
            let mut buf = vec![0u8; num_bytes];
            reader.read_exact(&mut buf).map_err(|e| {
                EdfError::InvalidDataRecord {
                    index: record_idx,
                    message: format!("failed to read signal {sig_idx}: {e}"),
                }
            })?;

            if is_annotation[sig_idx] {
                let mut annotations = parse_tals(&buf)?;
                all_annotations.append(&mut annotations);
            } else {
                let samples: Vec<i16> = buf
                    .chunks_exact(2)
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();

                let ord_idx = ordinary_indices.iter().position(|&i| i == sig_idx).unwrap();
                ordinary_records[ord_idx].push(EdfDataRecord { sample: samples });
            }
        }
    }

    // Build EdfSignal structs for ordinary signals
    let signals: Vec<EdfSignal> = ordinary_indices
        .iter()
        .zip(ordinary_records.into_iter())
        .map(|(&idx, records)| EdfSignal {
            header: header.signal_headers[idx].clone(),
            records,
        })
        .collect();

    Ok(EdfFile {
        header,
        signals,
        annotations: all_annotations,
    })
}

/// Write an EDF/EDF+ file to a writer.
///
/// Reconstructs the binary EDF format from an [`EdfFile`], including
/// encoding annotations back into "EDF Annotations" signal bytes.
///
/// # Errors
///
/// Returns [`EdfError::Io`] on write failure.
pub fn write_edf(edf: &EdfFile, writer: &mut impl Write) -> Result<(), EdfError> {
    edf.header.write_to(writer)?;

    let num_records = edf.header.data_records_count as usize;

    let annotation_indices: Vec<usize> = edf
        .header
        .signal_headers
        .iter()
        .enumerate()
        .filter(|(_, sh)| sh.is_annotation())
        .map(|(i, _)| i)
        .collect();

    let ordinary_indices: Vec<usize> = edf
        .header
        .signal_headers
        .iter()
        .enumerate()
        .filter(|(_, sh)| !sh.is_annotation())
        .map(|(i, _)| i)
        .collect();

    // Map from header signal index to position in edf.signals
    let mut ord_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (pos, &hdr_idx) in ordinary_indices.iter().enumerate() {
        ord_map.insert(hdr_idx, pos);
    }

    let annotations_per_record = group_annotations_by_record(
        &edf.annotations,
        num_records,
        edf.header.data_record_duration,
    );

    for record_idx in 0..num_records {
        for (sig_idx, sh) in edf.header.signal_headers.iter().enumerate() {
            if sh.is_annotation() {
                let ann_sig_idx = annotation_indices
                    .iter()
                    .position(|&i| i == sig_idx)
                    .unwrap_or(0);
                let empty = Vec::new();
                let anns = if ann_sig_idx == 0 {
                    &annotations_per_record[record_idx]
                } else {
                    &empty
                };
                let total_bytes = sh.samples_per_record * 2;
                let encoded = encode_tals(anns, total_bytes);
                writer.write_all(&encoded)?;
            } else {
                let ord_pos = ord_map[&sig_idx];
                let samples = &edf.signals[ord_pos].records[record_idx].sample;
                for &s in samples {
                    writer.write_all(&s.to_le_bytes())?;
                }
            }
        }
    }

    Ok(())
}

/// Group annotations into per-record buckets.
fn group_annotations_by_record(
    annotations: &[EdfAnnotation],
    num_records: usize,
    record_duration: f64,
) -> Vec<Vec<EdfAnnotation>> {
    let mut per_record: Vec<Vec<EdfAnnotation>> = Vec::with_capacity(num_records);

    let timekeeping: Vec<&EdfAnnotation> = annotations
        .iter()
        .filter(|a| a.texts.is_empty())
        .collect();
    let content: Vec<&EdfAnnotation> = annotations
        .iter()
        .filter(|a| !a.texts.is_empty())
        .collect();

    for record_idx in 0..num_records {
        let mut record_anns: Vec<EdfAnnotation> = Vec::new();

        if let Some(tk) = timekeeping.get(record_idx) {
            record_anns.push((*tk).clone());
        } else {
            let onset = record_idx as f64 * record_duration;
            record_anns.push(EdfAnnotation {
                onset,
                duration: None,
                texts: vec![],
            });
        }

        let record_start = if let Some(tk) = timekeeping.get(record_idx) {
            tk.onset
        } else {
            record_idx as f64 * record_duration
        };
        let record_end = if record_idx + 1 < num_records {
            if let Some(tk) = timekeeping.get(record_idx + 1) {
                tk.onset
            } else {
                (record_idx + 1) as f64 * record_duration
            }
        } else {
            f64::INFINITY
        };

        for ann in &content {
            if ann.onset >= record_start && ann.onset < record_end {
                record_anns.push((*ann).clone());
            }
        }

        per_record.push(record_anns);
    }

    per_record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::EdfSignalHeader;

    fn make_annotation_header(samples_per_record: usize) -> EdfSignalHeader {
        EdfSignalHeader {
            label: "EDF Annotations".into(),
            transducer_type: String::new(),
            physical_dimension: String::new(),
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            digital_minimum: -32768,
            digital_maximum: 32767,
            prefiltering: String::new(),
            samples_per_record,
            reserved: String::new(),
        }
    }

    fn make_ordinary_header(label: &str, samples: usize) -> EdfSignalHeader {
        EdfSignalHeader {
            label: label.into(),
            transducer_type: String::new(),
            physical_dimension: "uV".into(),
            physical_minimum: -100.0,
            physical_maximum: 100.0,
            digital_minimum: -2048,
            digital_maximum: 2047,
            prefiltering: String::new(),
            samples_per_record: samples,
            reserved: String::new(),
        }
    }

    #[test]
    fn test_round_trip_simple() {
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+C".into(),
                data_records_count: 1,
                data_record_duration: 1.0,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EEG", 4),
                    make_annotation_header(10),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EEG", 4),
                records: vec![EdfDataRecord { sample: vec![100, -200, 300, -400] }],
            }],
            annotations: vec![EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            }],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.header.version, "0");
        assert_eq!(parsed.signals.len(), 1);
        assert_eq!(parsed.signals[0].records[0].sample, vec![100, -200, 300, -400]);
        assert_eq!(parsed.annotations.len(), 1);
        assert_eq!(parsed.annotations[0].onset, 0.0);
    }

    #[test]
    fn test_group_annotations_by_record() {
        let annotations = vec![
            EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            },
            EdfAnnotation {
                onset: 0.5,
                duration: None,
                texts: vec!["Event A".into()],
            },
            EdfAnnotation {
                onset: 1.0,
                duration: None,
                texts: vec![],
            },
            EdfAnnotation {
                onset: 1.5,
                duration: Some(0.5),
                texts: vec!["Event B".into()],
            },
        ];

        let grouped = group_annotations_by_record(&annotations, 2, 1.0);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].len(), 2);
        assert!(grouped[0][0].texts.is_empty());
        assert_eq!(grouped[0][1].texts, vec!["Event A"]);
        assert_eq!(grouped[1].len(), 2);
        assert!(grouped[1][0].texts.is_empty());
        assert_eq!(grouped[1][1].texts, vec!["Event B"]);
    }
}
