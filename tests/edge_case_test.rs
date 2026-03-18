//! Integration-level edge case tests for the European Data Format (EDF/EDF+) crate.
//!
//! These tests exercise edge cases identified in `doc/edge-cases/index.md`,
//! focusing on cross-module interactions and full round-trip behavior
//! that cannot be tested within a single module's unit tests.
//!
//! Each test is tagged with the edge case number it addresses.

use european_data_format::annotation::{encode_tals, parse_tals};
use european_data_format::edf_file::{EdfDataRecord, EdfSignal};
use european_data_format::header::{EdfHeader, EdfSignalHeader};
use european_data_format::{io_edf, io_json, io_xml, EdfAnnotation, EdfFile};

// ── Helper Functions ─────────────────────────────────────────────────

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

fn make_annotation_header(samples: usize) -> EdfSignalHeader {
    EdfSignalHeader {
        label: "EDF Annotations".into(),
        transducer_type: String::new(),
        physical_dimension: String::new(),
        physical_minimum: -1.0,
        physical_maximum: 1.0,
        digital_minimum: -32768,
        digital_maximum: 32767,
        prefiltering: String::new(),
        samples_per_record: samples,
        reserved: String::new(),
    }
}

/// Build a minimal valid EDF+ file for testing.
fn make_minimal_edf(
    reserved: &str,
    num_records: usize,
    duration: f64,
    signals: Vec<(EdfSignalHeader, Vec<Vec<i16>>)>,
    annotations: Vec<EdfAnnotation>,
) -> EdfFile {
    let ann_header = make_annotation_header(30);
    let mut all_signal_headers: Vec<EdfSignalHeader> =
        signals.iter().map(|(h, _)| h.clone()).collect();
    all_signal_headers.push(ann_header);

    let signals_count = all_signal_headers.len();
    let header_bytes = 256 + signals_count * 256;

    let edf_signals: Vec<EdfSignal> = signals
        .into_iter()
        .map(|(header, records)| EdfSignal {
            header,
            records: records
                .into_iter()
                .map(|sample| EdfDataRecord { sample })
                .collect(),
        })
        .collect();

    EdfFile {
        header: EdfHeader {
            version: "0".into(),
            patient_identification: "X X X X".into(),
            recording_identification: "Startdate X X X X".into(),
            start_date: "01.01.00".into(),
            start_time: "00.00.00".into(),
            header_bytes,
            reserved: reserved.into(),
            data_records_count: num_records as i64,
            data_record_duration: duration,
            signals_count,
            signal_headers: all_signal_headers,
        },
        signals: edf_signals,
        annotations,
    }
}

// ── Edge Case 1: Contiguous Data Limitations ─────────────────────────

#[test]
fn test_edge_case_1_contiguous_sequential_timing() {
    // Edge Case 1: In a contiguous (EDF+C) recording, all data records
    // are consecutive in time. This test creates a 5-record contiguous
    // file and verifies that the sequential timing is preserved through
    // a full EDF→JSON→EDF round-trip.
    let num_records = 5;
    let duration = 1.0;

    let records: Vec<Vec<i16>> = (0..num_records)
        .map(|r| vec![(r * 10) as i16, (r * 10 + 1) as i16])
        .collect();

    let mut annotations = Vec::new();
    for r in 0..num_records {
        annotations.push(EdfAnnotation {
            onset: r as f64 * duration,
            duration: None,
            texts: vec![],
        });
    }

    let edf = make_minimal_edf(
        "EDF+C",
        num_records,
        duration,
        vec![(make_ordinary_header("EEG", 2), records)],
        annotations,
    );

    // Round-trip: EDF → JSON → EDF
    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();

    let mut edf_bytes = Vec::new();
    io_edf::write_edf(&from_json, &mut edf_bytes).unwrap();
    let mut cursor = std::io::Cursor::new(&edf_bytes);
    let round_tripped = io_edf::read_edf(&mut cursor).unwrap();

    // Verify contiguous timing: each record's onset = record_index * duration
    for (i, ann) in round_tripped
        .annotations
        .iter()
        .filter(|a| a.texts.is_empty())
        .enumerate()
    {
        assert_eq!(
            ann.onset,
            i as f64 * duration,
            "Record {i} onset should be {}, got {}",
            i as f64 * duration,
            ann.onset
        );
    }
}

// ── Edge Case 2: Discontinuous Data (EDF+D) ─────────────────────────

#[test]
fn test_edge_case_2_discontinuous_time_gaps_preserved() {
    // Edge Case 2: EDF+D files have time gaps between records. Here we
    // create a file with 3 records at t=0, t=30, t=120 (representing
    // an intermittent recording). The time gaps must be preserved
    // through EDF→XML→EDF round-trip.
    let onsets = [0.0, 30.0, 120.0];
    let mut annotations = Vec::new();
    for &onset in &onsets {
        annotations.push(EdfAnnotation {
            onset,
            duration: None,
            texts: vec![],
        });
    }
    annotations.push(EdfAnnotation {
        onset: 0.0,
        duration: None,
        texts: vec!["Start".into()],
    });

    let records: Vec<Vec<i16>> = (0..3).map(|r| vec![(r * 100) as i16]).collect();

    let edf = make_minimal_edf(
        "EDF+D",
        3,
        1.0,
        vec![(make_ordinary_header("EEG", 1), records)],
        annotations,
    );

    // Round-trip: EDF → XML → EDF
    let xml = io_xml::to_xml(&edf).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();

    let mut edf_bytes = Vec::new();
    io_edf::write_edf(&from_xml, &mut edf_bytes).unwrap();
    let mut cursor = std::io::Cursor::new(&edf_bytes);
    let round_tripped = io_edf::read_edf(&mut cursor).unwrap();

    // Verify time gaps are preserved
    let timekeeping: Vec<&EdfAnnotation> = round_tripped
        .annotations
        .iter()
        .filter(|a| a.texts.is_empty())
        .collect();
    assert_eq!(timekeeping.len(), 3);
    assert_eq!(timekeeping[0].onset, 0.0);
    assert_eq!(timekeeping[1].onset, 30.0);
    assert_eq!(timekeeping[2].onset, 120.0);
}

// ── Edge Case 5: 16-Bit Data Type Boundaries ────────────────────────

#[test]
fn test_edge_case_5_extreme_sample_values_all_formats() {
    // Edge Case 5: EDF uses 16-bit signed integers. Test that extreme
    // values (-32768, 32767, 0) survive round-trip through ALL three
    // formats: EDF binary, JSON, and XML.
    let extreme_samples = vec![i16::MIN, i16::MAX, 0, -1, 1, i16::MIN + 1, i16::MAX - 1];
    let num_samples = extreme_samples.len();

    let edf = make_minimal_edf(
        "EDF+C",
        1,
        1.0,
        vec![(make_ordinary_header("Test", num_samples), vec![extreme_samples.clone()])],
        vec![EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        }],
    );

    // Test EDF round-trip
    let mut edf_buf = Vec::new();
    io_edf::write_edf(&edf, &mut edf_buf).unwrap();
    let mut cursor = std::io::Cursor::new(&edf_buf);
    let from_edf = io_edf::read_edf(&mut cursor).unwrap();
    assert_eq!(from_edf.signals[0].records[0].sample, extreme_samples);

    // Test JSON round-trip
    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();
    assert_eq!(from_json.signals[0].records[0].sample, extreme_samples);

    // Test XML round-trip
    let xml = io_xml::to_xml(&edf).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();
    assert_eq!(from_xml.signals[0].records[0].sample, extreme_samples);
}

// ── Edge Case 6: ASCII Header Constraints ────────────────────────────

#[test]
fn test_edge_case_6_ascii_header_fields_preserved_through_all_formats() {
    // Edge Case 6: EDF headers must be ASCII. This test creates a file
    // with all printable ASCII characters in header fields and verifies
    // they survive round-trip through EDF, JSON, and XML.
    let patient_id = "PAT-001 M 15-MAR-1980 John_Doe";
    let recording_id = "Startdate 15-MAR-2020 LAB01 DR.SMITH Equipment_v2";

    let edf = make_minimal_edf(
        "EDF+C",
        1,
        1.0,
        vec![(make_ordinary_header("EEG Fp1-Ref", 2), vec![vec![100, -100]])],
        vec![EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        }],
    );

    // Override patient/recording IDs
    let mut edf = edf;
    edf.header.patient_identification = patient_id.into();
    edf.header.recording_identification = recording_id.into();

    // EDF round-trip
    let mut buf = Vec::new();
    io_edf::write_edf(&edf, &mut buf).unwrap();
    let mut cursor = std::io::Cursor::new(&buf);
    let from_edf = io_edf::read_edf(&mut cursor).unwrap();
    assert_eq!(from_edf.header.patient_identification, patient_id);
    assert_eq!(from_edf.header.recording_identification, recording_id);

    // JSON round-trip
    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();
    assert_eq!(from_json.header.patient_identification, patient_id);

    // XML round-trip
    let xml = io_xml::to_xml(&edf).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();
    assert_eq!(from_xml.header.patient_identification, patient_id);
}

// ── Edge Case 7: Length-Restricted Fields ────────────────────────────

#[test]
fn test_edge_case_7_signal_label_boundary_lengths() {
    // Edge Case 7: Signal labels are limited to 16 characters. Test with
    // labels of exactly 16, 15, and 1 character to verify that the
    // boundary conditions don't cause data corruption during round-trip.
    let labels = vec!["1234567890123456", "Short", "X"];

    for label in labels {
        let header = make_ordinary_header(label, 2);
        let edf = make_minimal_edf(
            "EDF+C",
            1,
            1.0,
            vec![(header, vec![vec![10, 20]])],
            vec![EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            }],
        );

        let mut buf = Vec::new();
        io_edf::write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(&buf);
        let parsed = io_edf::read_edf(&mut cursor).unwrap();

        // Labels should be preserved (up to 16 chars)
        if label.len() <= 16 {
            assert_eq!(parsed.signals[0].header.label, label);
        }
    }
}

// ── Edge Case 8: Calibration Round-Trip ──────────────────────────────

#[test]
fn test_edge_case_8_calibration_values_preserved_through_json() {
    // Edge Case 8: Digital and physical min/max values define the scaling.
    // Non-integer physical values (e.g., -100.5) and extreme digital ranges
    // must be preserved through JSON round-trip without precision loss.
    let header = EdfSignalHeader {
        label: "ECG".into(),
        transducer_type: "AgAgCl electrode".into(),
        physical_dimension: "mV".into(),
        physical_minimum: -3.2768, // non-round value
        physical_maximum: 3.2767,
        digital_minimum: -32768,
        digital_maximum: 32767,
        prefiltering: "HP:0.05Hz LP:150Hz N:50Hz".into(),
        samples_per_record: 2,
        reserved: String::new(),
    };

    let edf = make_minimal_edf(
        "EDF+C",
        1,
        1.0,
        vec![(header.clone(), vec![vec![1000, -1000]])],
        vec![EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        }],
    );

    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();

    // Calibration values must survive exactly
    assert_eq!(from_json.signals[0].header.physical_minimum, -3.2768);
    assert_eq!(from_json.signals[0].header.physical_maximum, 3.2767);
    assert_eq!(from_json.signals[0].header.digital_minimum, -32768);
    assert_eq!(from_json.signals[0].header.digital_maximum, 32767);
}

// ── Edge Case 9: Reserved Field Variants ─────────────────────────────

#[test]
fn test_edge_case_9_reserved_field_variants_round_trip() {
    // Edge Case 9: The reserved field differs between EDF and EDF+.
    // Test that various reserved field values survive round-trip correctly.
    let variants = ["EDF+C", "EDF+D", ""];

    for reserved in variants {
        let edf = make_minimal_edf(
            reserved,
            1,
            1.0,
            vec![(make_ordinary_header("EEG", 2), vec![vec![10, 20]])],
            vec![EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            }],
        );

        let mut buf = Vec::new();
        io_edf::write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(&buf);
        let parsed = io_edf::read_edf(&mut cursor).unwrap();

        assert_eq!(parsed.header.reserved, reserved);
    }
}

// ── Edge Case 10: Annotation Duration Semantics ──────────────────────

#[test]
fn test_edge_case_10_duration_none_vs_zero_through_all_formats() {
    // Edge Case 10: duration=None (not specified) vs duration=Some(0.0)
    // (explicitly instantaneous) have different clinical semantics.
    // Both must be preserved through EDF, JSON, and XML round-trips.
    let annotations = vec![
        EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        },
        EdfAnnotation {
            onset: 0.0,
            duration: Some(0.0), // explicitly zero
            texts: vec!["Instant event".into()],
        },
        EdfAnnotation {
            onset: 0.0,
            duration: Some(30.0), // 30-second epoch
            texts: vec!["Sleep stage W".into()],
        },
        EdfAnnotation {
            onset: 0.0,
            duration: None, // no duration
            texts: vec!["Point marker".into()],
        },
    ];

    let edf = make_minimal_edf(
        "EDF+C",
        1,
        1.0,
        vec![(make_ordinary_header("EEG", 2), vec![vec![0, 0]])],
        annotations.clone(),
    );

    // JSON round-trip
    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();
    assert_eq!(from_json.annotations[1].duration, Some(0.0));
    assert_eq!(from_json.annotations[2].duration, Some(30.0));
    assert_eq!(from_json.annotations[3].duration, None);

    // XML round-trip
    let xml = io_xml::to_xml(&edf).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();
    assert_eq!(from_xml.annotations[1].duration, Some(0.0));
    assert_eq!(from_xml.annotations[2].duration, Some(30.0));
    assert_eq!(from_xml.annotations[3].duration, None);
}

// ── Edge Case 11: Date Formatting ────────────────────────────────────

#[test]
fn test_edge_case_11_date_time_preserved_through_all_formats() {
    // Edge Case 11: The dd.mm.yy date and hh.mm.ss time formats must be
    // preserved exactly. Even though "00" year is ambiguous (1900 vs 2000),
    // the string representation must not change during round-trip.
    let dates = [
        ("01.01.00", "00.00.00"), // midnight, year 2000 (or 1900)
        ("31.12.99", "23.59.59"), // one second before midnight
        ("17.04.01", "11.25.00"), // from the EDF+ spec example
        ("29.02.04", "12.00.00"), // leap day
    ];

    for (date, time) in dates {
        let mut edf = make_minimal_edf(
            "EDF+C",
            1,
            1.0,
            vec![(make_ordinary_header("EEG", 2), vec![vec![0, 0]])],
            vec![EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec![],
            }],
        );
        edf.header.start_date = date.into();
        edf.header.start_time = time.into();

        // EDF round-trip
        let mut buf = Vec::new();
        io_edf::write_edf(&edf, &mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(&buf);
        let from_edf = io_edf::read_edf(&mut cursor).unwrap();
        assert_eq!(from_edf.header.start_date, date, "date mismatch for {date}");
        assert_eq!(from_edf.header.start_time, time, "time mismatch for {time}");

        // JSON round-trip
        let json = io_json::to_json(&edf).unwrap();
        let from_json = io_json::from_json(&json).unwrap();
        assert_eq!(from_json.header.start_date, date);
        assert_eq!(from_json.header.start_time, time);
    }
}

// ── Edge Case 12: Manually Created Files ─────────────────────────────

#[test]
fn test_edge_case_12_header_size_consistency() {
    // Edge Case 12: The header_bytes field must equal 256 + signals_count * 256.
    // This test creates files with 0, 1, 5, and 10 signals and verifies
    // that the written file size matches the formula exactly.
    for num_signals in [0, 1, 5, 10] {
        let signal_headers: Vec<EdfSignalHeader> = (0..num_signals)
            .map(|i| make_ordinary_header(&format!("Ch{i}"), 2))
            .collect();
        let signals: Vec<EdfSignal> = signal_headers
            .iter()
            .map(|h| EdfSignal {
                header: h.clone(),
                records: vec![],
            })
            .collect();

        let expected_header_bytes = 256 + num_signals * 256;

        let edf = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "X X X X".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: expected_header_bytes,
                reserved: String::new(),
                data_records_count: 0,
                data_record_duration: 0.0,
                signals_count: num_signals,
                signal_headers,
            },
            signals,
            annotations: vec![],
        };

        let mut buf = Vec::new();
        io_edf::write_edf(&edf, &mut buf).unwrap();

        // With 0 data records, file should be exactly the header
        assert_eq!(
            buf.len(),
            expected_header_bytes,
            "File with {num_signals} signals: expected {expected_header_bytes} bytes, got {}",
            buf.len()
        );

        // Read back and verify
        let mut cursor = std::io::Cursor::new(&buf);
        let parsed = io_edf::read_edf(&mut cursor).unwrap();
        assert_eq!(parsed.header.signals_count, num_signals);
        assert_eq!(parsed.header.header_bytes, expected_header_bytes);
    }
}

// ── Edge Case 13: Channel-Oriented Access ────────────────────────────

#[test]
fn test_edge_case_13_multi_channel_data_isolation() {
    // Edge Case 13: EDF stores data as sample-oriented (interleaved channels
    // within each data record). After parsing, each channel's data should
    // be fully isolated. Modifying one channel's data should not affect
    // another. This test verifies that the parsed representation correctly
    // separates channel data.
    let num_channels = 4;
    let samples_per_record = 3;

    let signals: Vec<(EdfSignalHeader, Vec<Vec<i16>>)> = (0..num_channels)
        .map(|ch| {
            let header = make_ordinary_header(&format!("Ch{ch}"), samples_per_record);
            let data = vec![vec![(ch * 1000) as i16, (ch * 1000 + 1) as i16, (ch * 1000 + 2) as i16]];
            (header, data)
        })
        .collect();

    let edf = make_minimal_edf(
        "EDF+C",
        1,
        1.0,
        signals,
        vec![EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        }],
    );

    let mut buf = Vec::new();
    io_edf::write_edf(&edf, &mut buf).unwrap();
    let mut cursor = std::io::Cursor::new(&buf);
    let parsed = io_edf::read_edf(&mut cursor).unwrap();

    // Verify each channel's data is independent and correct
    assert_eq!(parsed.signals.len(), num_channels);
    for ch in 0..num_channels {
        let expected: Vec<i16> = vec![
            (ch * 1000) as i16,
            (ch * 1000 + 1) as i16,
            (ch * 1000 + 2) as i16,
        ];
        assert_eq!(
            parsed.signals[ch].records[0].sample, expected,
            "Channel {ch} data mismatch"
        );
    }
}

// ── Cross-Format Consistency ─────────────────────────────────────────

#[test]
fn test_edge_case_json_xml_structural_equivalence() {
    // Verify that JSON and XML representations produce structurally identical
    // EdfFile instances. This ensures that neither format introduces artifacts
    // or loses information relative to the other.
    let edf = make_minimal_edf(
        "EDF+D",
        2,
        0.5,
        vec![
            (make_ordinary_header("EEG Fp1", 4), vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]]),
            (make_ordinary_header("EMG", 2), vec![vec![100, -100], vec![200, -200]]),
        ],
        vec![
            EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
            EdfAnnotation { onset: 0.0, duration: Some(0.5), texts: vec!["Event A".into()] },
            EdfAnnotation { onset: 10.0, duration: None, texts: vec![] },
            EdfAnnotation { onset: 10.5, duration: None, texts: vec!["Event B".into(), "Note".into()] },
        ],
    );

    let json = io_json::to_json(&edf).unwrap();
    let xml = io_xml::to_xml(&edf).unwrap();

    let from_json = io_json::from_json(&json).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();

    assert_eq!(from_json, from_xml, "JSON and XML should produce identical EdfFile instances");
}

#[test]
fn test_edge_case_full_round_trip_edf_json_xml_edf() {
    // The ultimate round-trip test: EDF → JSON → XML → EDF.
    // The final EDF binary should be byte-identical to a fresh write
    // from the same in-memory structure.
    let edf = make_minimal_edf(
        "EDF+C",
        2,
        1.0,
        vec![(make_ordinary_header("EEG", 3), vec![vec![10, -20, 30], vec![40, -50, 60]])],
        vec![
            EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
            EdfAnnotation { onset: 0.5, duration: None, texts: vec!["Marker".into()] },
            EdfAnnotation { onset: 1.0, duration: None, texts: vec![] },
        ],
    );

    // Write original EDF
    let mut original_bytes = Vec::new();
    io_edf::write_edf(&edf, &mut original_bytes).unwrap();

    // EDF → JSON
    let json = io_json::to_json(&edf).unwrap();
    let from_json = io_json::from_json(&json).unwrap();

    // JSON → XML
    let xml = io_xml::to_xml(&from_json).unwrap();
    let from_xml = io_xml::from_xml(&xml).unwrap();

    // XML → EDF
    let mut final_bytes = Vec::new();
    io_edf::write_edf(&from_xml, &mut final_bytes).unwrap();

    assert_eq!(
        original_bytes, final_bytes,
        "EDF → JSON → XML → EDF should produce byte-identical output"
    );
}

// ── TAL Encoding Edge Cases ──────────────────────────────────────────

#[test]
fn test_edge_case_annotation_signal_bytes_fit_in_allocated_space() {
    // The annotation signal has samples_per_record * 2 bytes available.
    // If the encoded TALs exceed this space, data is silently truncated
    // by encode_tals. This test verifies that typical annotation volumes
    // fit within the allocated space.
    let annotations: Vec<EdfAnnotation> = (0..5)
        .map(|i| EdfAnnotation {
            onset: i as f64 * 10.0,
            duration: Some(1.0),
            texts: vec![format!("Event number {i}")],
        })
        .collect();

    let total_bytes = 60 * 2; // 60 samples_per_record * 2 bytes
    let encoded = encode_tals(&annotations, total_bytes);
    assert_eq!(encoded.len(), total_bytes);

    // Verify all annotations are recoverable (not truncated)
    let parsed = parse_tals(&encoded).unwrap();
    assert_eq!(parsed.len(), 5, "all 5 annotations should fit in {total_bytes} bytes");
}
