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

    // ── Edge Case Tests: Contiguous Data (Edge Case 1) ───────────────

    #[test]
    fn test_edge_case_edf_plus_contiguous_round_trip() {
        // EDF+C (contiguous) means data records are consecutive with no time gaps.
        // The timekeeping TAL onsets should be sequential:
        // record 0 → onset 0.0, record 1 → onset 1.0, etc.
        // This test verifies that a contiguous EDF+ file round-trips correctly.
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+C".into(), // contiguous
                data_records_count: 3,
                data_record_duration: 1.0,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EEG", 4),
                    make_annotation_header(15),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EEG", 4),
                records: vec![
                    EdfDataRecord { sample: vec![10, 20, 30, 40] },
                    EdfDataRecord { sample: vec![50, 60, 70, 80] },
                    EdfDataRecord { sample: vec![90, 100, 110, 120] },
                ],
            }],
            annotations: vec![
                // Timekeeping TALs for each record
                EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 1.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 2.0, duration: None, texts: vec![] },
            ],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        // All 3 timekeeping annotations should be preserved with correct onsets
        assert_eq!(parsed.annotations.len(), 3);
        assert_eq!(parsed.annotations[0].onset, 0.0);
        assert_eq!(parsed.annotations[1].onset, 1.0);
        assert_eq!(parsed.annotations[2].onset, 2.0);

        // Signal data should be preserved exactly
        assert_eq!(parsed.signals[0].records[0].sample, vec![10, 20, 30, 40]);
        assert_eq!(parsed.signals[0].records[2].sample, vec![90, 100, 110, 120]);
    }

    // ── Edge Case Tests: Discontinuous Data (Edge Case 2) ────────────

    #[test]
    fn test_edge_case_edf_plus_discontinuous_with_time_gap() {
        // EDF+D (discontinuous) allows time gaps between data records. The
        // timekeeping TAL in each record specifies the actual start time.
        // Here, record 0 starts at t=0, record 1 starts at t=10 (9-second gap).
        // This is common in event-triggered recordings like EMG nerve conduction.
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+D".into(), // discontinuous
                data_records_count: 2,
                data_record_duration: 0.05,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EMG", 4),
                    make_annotation_header(15),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EMG", 4),
                records: vec![
                    EdfDataRecord { sample: vec![100, -200, 300, -400] },
                    EdfDataRecord { sample: vec![150, -250, 350, -450] },
                ],
            }],
            annotations: vec![
                // Record 0: onset at t=0 (wrist stimulation)
                EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 0.0, duration: None, texts: vec!["Wrist stimulus".into()] },
                // Record 1: onset at t=10 (elbow stimulation, 10 seconds later)
                EdfAnnotation { onset: 10.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 10.0, duration: None, texts: vec!["Elbow stimulus".into()] },
            ],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        // Verify the time gap is preserved: record 0 at t=0, record 1 at t=10
        assert_eq!(parsed.annotations.len(), 4);
        assert_eq!(parsed.annotations[0].onset, 0.0);
        assert_eq!(parsed.annotations[2].onset, 10.0);

        // Content annotations should be preserved
        assert_eq!(parsed.annotations[1].texts, vec!["Wrist stimulus"]);
        assert_eq!(parsed.annotations[3].texts, vec!["Elbow stimulus"]);
    }

    // ── Edge Case Tests: Data Type Handling (Edge Case 5) ────────────

    #[test]
    fn test_edge_case_16bit_sample_range_boundaries() {
        // EDF stores samples as 16-bit signed integers (little-endian).
        // Valid range: -32768 to 32767. This test verifies that extreme
        // values at both ends of the i16 range survive round-trip correctly.
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
                    make_ordinary_header("Test", 5),
                    make_annotation_header(10),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("Test", 5),
                records: vec![EdfDataRecord {
                    // Test boundary values: min, max, zero, and near-boundaries
                    sample: vec![i16::MIN, i16::MAX, 0, i16::MIN + 1, i16::MAX - 1],
                }],
            }],
            annotations: vec![EdfAnnotation { onset: 0.0, duration: None, texts: vec![] }],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.signals[0].records[0].sample, vec![-32768, 32767, 0, -32767, 32766]);
    }

    #[test]
    fn test_edge_case_little_endian_byte_order() {
        // EDF spec mandates little-endian byte order for sample values.
        // For example, the value 256 (0x0100) should be stored as [0x00, 0x01].
        // This test manually constructs bytes and verifies correct parsing.
        let ann_header = make_annotation_header(10);
        let sig_header = EdfSignalHeader {
            label: "Test".into(),
            transducer_type: String::new(),
            physical_dimension: "uV".into(),
            physical_minimum: -100.0,
            physical_maximum: 100.0,
            digital_minimum: -2048,
            digital_maximum: 2047,
            prefiltering: String::new(),
            samples_per_record: 2,
            reserved: String::new(),
        };

        let header = EdfHeader {
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
            signal_headers: vec![sig_header.clone(), ann_header],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();

        // Manually write sample data in little-endian:
        // Sample 1: 256 = 0x0100 → bytes [0x00, 0x01]
        // Sample 2: -1 = 0xFFFF → bytes [0xFF, 0xFF]
        buf.extend_from_slice(&[0x00, 0x01]); // 256 in LE
        buf.extend_from_slice(&[0xFF, 0xFF]); // -1 in LE

        // Write annotation signal (timekeeping TAL)
        let tal_bytes = crate::annotation::encode_tals(
            &[EdfAnnotation { onset: 0.0, duration: None, texts: vec![] }],
            20,
        );
        buf.extend_from_slice(&tal_bytes);

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.signals[0].records[0].sample[0], 256);
        assert_eq!(parsed.signals[0].records[0].sample[1], -1);
    }

    // ── Edge Case Tests: Multiple Data Records (Edge Cases 1, 3) ─────

    #[test]
    fn test_edge_case_many_data_records_round_trip() {
        // Test with a larger number of data records to verify that the
        // read/write loop handles indexing correctly across many iterations.
        // This catches off-by-one errors in record counting.
        let num_records = 20;
        let samples_per_record = 4;

        let records: Vec<EdfDataRecord> = (0..num_records)
            .map(|r| EdfDataRecord {
                sample: (0..samples_per_record).map(|s| (r * 100 + s) as i16).collect(),
            })
            .collect();

        let timekeeping: Vec<EdfAnnotation> = (0..num_records)
            .map(|r| EdfAnnotation {
                onset: r as f64,
                duration: None,
                texts: vec![],
            })
            .collect();

        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+C".into(),
                data_records_count: num_records as i64,
                data_record_duration: 1.0,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EEG", samples_per_record),
                    make_annotation_header(10),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EEG", samples_per_record),
                records,
            }],
            annotations: timekeeping,
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.signals[0].records.len(), num_records);
        // Verify first and last record data
        assert_eq!(parsed.signals[0].records[0].sample, vec![0, 1, 2, 3]);
        assert_eq!(parsed.signals[0].records[19].sample, vec![1900, 1901, 1902, 1903]);
    }

    // ── Edge Case Tests: Multiple Signals (Edge Case 4) ──────────────

    #[test]
    fn test_edge_case_multiple_ordinary_signals() {
        // An EDF file can have many ordinary signals (e.g., 19 EEG channels
        // in a 10-20 montage, plus EMG, EOG, ECG). This test verifies that
        // multiple ordinary signals are correctly separated and preserved
        // through a round-trip, with each signal's data in the right place.
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 256 + 4 * 256, // 4 signals
                reserved: "EDF+C".into(),
                data_records_count: 1,
                data_record_duration: 1.0,
                signals_count: 4,
                signal_headers: vec![
                    make_ordinary_header("EEG Fp1", 3),
                    make_ordinary_header("EEG Fp2", 3),
                    make_ordinary_header("EMG chin", 3),
                    make_annotation_header(10),
                ],
            },
            signals: vec![
                EdfSignal {
                    header: make_ordinary_header("EEG Fp1", 3),
                    records: vec![EdfDataRecord { sample: vec![100, 200, 300] }],
                },
                EdfSignal {
                    header: make_ordinary_header("EEG Fp2", 3),
                    records: vec![EdfDataRecord { sample: vec![400, 500, 600] }],
                },
                EdfSignal {
                    header: make_ordinary_header("EMG chin", 3),
                    records: vec![EdfDataRecord { sample: vec![700, 800, 900] }],
                },
            ],
            annotations: vec![EdfAnnotation { onset: 0.0, duration: None, texts: vec![] }],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        // Verify each signal's data is in the correct position
        assert_eq!(parsed.signals.len(), 3);
        assert_eq!(parsed.signals[0].header.label, "EEG Fp1");
        assert_eq!(parsed.signals[0].records[0].sample, vec![100, 200, 300]);
        assert_eq!(parsed.signals[1].header.label, "EEG Fp2");
        assert_eq!(parsed.signals[1].records[0].sample, vec![400, 500, 600]);
        assert_eq!(parsed.signals[2].header.label, "EMG chin");
        assert_eq!(parsed.signals[2].records[0].sample, vec![700, 800, 900]);
    }

    // ── Edge Case Tests: Annotation-Only File ────────────────────────

    #[test]
    fn test_edge_case_file_with_only_annotation_signal() {
        // An EDF+ file can contain only an annotation signal and no ordinary
        // signals. This is uncommon but valid — it would be a pure event log
        // with no waveform data (e.g., nurse notes, medication log).
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 512,
                reserved: "EDF+C".into(),
                data_records_count: 1,
                data_record_duration: 1.0,
                signals_count: 1,
                signal_headers: vec![make_annotation_header(30)],
            },
            signals: vec![], // no ordinary signals
            annotations: vec![
                EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 0.0, duration: None, texts: vec!["Recording start".into()] },
            ],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert!(parsed.signals.is_empty());
        assert_eq!(parsed.annotations.len(), 2);
        assert_eq!(parsed.annotations[1].texts, vec!["Recording start"]);
    }

    // ── Edge Case Tests: Truncated Input ─────────────────────────────

    #[test]
    fn test_edge_case_truncated_header_returns_error() {
        // Edge Case 12: A file that is too short to contain a complete header
        // (less than 256 bytes) should produce a clear I/O error, not a panic.
        let truncated = vec![0u8; 100]; // only 100 bytes, need 256 minimum
        let mut cursor = std::io::Cursor::new(truncated);
        let result = read_edf(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_case_truncated_data_record_returns_error() {
        // A file with a valid header but truncated data records should
        // produce an error, not silently return partial data.
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+C".into(),
                data_records_count: 2, // claims 2 records
                data_record_duration: 1.0,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EEG", 4),
                    make_annotation_header(10),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EEG", 4),
                records: vec![
                    EdfDataRecord { sample: vec![1, 2, 3, 4] },
                    EdfDataRecord { sample: vec![5, 6, 7, 8] },
                ],
            }],
            annotations: vec![
                EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
                EdfAnnotation { onset: 1.0, duration: None, texts: vec![] },
            ],
        };

        let mut full_buf = Vec::new();
        write_edf(&edf, &mut full_buf).unwrap();

        // Truncate the buffer to remove the second data record
        let truncated_len = full_buf.len() - 10; // remove last 10 bytes
        let truncated = full_buf[..truncated_len].to_vec();

        let mut cursor = std::io::Cursor::new(truncated);
        let result = read_edf(&mut cursor);
        assert!(result.is_err(), "truncated data should produce an error");
    }

    // ── Edge Case Tests: data_records_count = -1 (Edge Case 12) ─────

    #[test]
    fn test_edge_case_data_records_count_negative_one_rejected() {
        // The EDF spec allows data_records_count = -1 to indicate a file
        // that hasn't been closed properly (still recording). Our reader
        // should reject such files because we can't determine how much
        // data to read.
        let header = EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes: 512,
            reserved: "EDF+C".into(),
            data_records_count: -1,
            data_record_duration: 1.0,
            signals_count: 1,
            signal_headers: vec![make_ordinary_header("EEG", 4)],
        };

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_edf(&mut cursor);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("-1") || err_msg.contains("not properly closed"),
            "error should mention -1 or unclosed file: {err_msg}");
    }

    // ── Edge Case Tests: Annotation at Record Boundary (Edge Case 10) ─

    #[test]
    fn test_edge_case_annotation_at_exact_record_boundary() {
        // An annotation with onset exactly at a record boundary should be
        // placed in the second record (onset >= record_start, onset < record_end).
        // For 1-second records: record 0 = [0, 1), record 1 = [1, 2).
        // An annotation at onset=1.0 should go in record 1.
        let annotations = vec![
            EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
            EdfAnnotation { onset: 1.0, duration: None, texts: vec![] },
            EdfAnnotation { onset: 1.0, duration: None, texts: vec!["At boundary".into()] },
        ];

        let grouped = group_annotations_by_record(&annotations, 2, 1.0);

        // Record 0: only timekeeping
        assert_eq!(grouped[0].len(), 1);
        assert!(grouped[0][0].texts.is_empty());

        // Record 1: timekeeping + boundary annotation
        assert_eq!(grouped[1].len(), 2);
        assert!(grouped[1][0].texts.is_empty());
        assert_eq!(grouped[1][1].texts, vec!["At boundary"]);
    }

    // ── Edge Case Tests: Zero Data Records ───────────────────────────

    #[test]
    fn test_edge_case_zero_data_records() {
        // A file with header_bytes indicating data but data_records_count=0
        // is valid — it's a header-only file. This can happen when recording
        // is set up but never started.
        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 768,
                reserved: "EDF+C".into(),
                data_records_count: 0,
                data_record_duration: 1.0,
                signals_count: 2,
                signal_headers: vec![
                    make_ordinary_header("EEG", 4),
                    make_annotation_header(10),
                ],
            },
            signals: vec![EdfSignal {
                header: make_ordinary_header("EEG", 4),
                records: vec![], // no data records
            }],
            annotations: vec![],
        };

        let mut buf = Vec::new();
        write_edf(&edf, &mut buf).unwrap();

        // The file should be exactly header_bytes long (no data section)
        assert_eq!(buf.len(), 768);

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.signals[0].records.len(), 0);
        assert!(parsed.annotations.is_empty());
    }
}
