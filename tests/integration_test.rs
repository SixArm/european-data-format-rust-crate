//! Integration tests for the European Data Format (EDF/EDF+) crate.
//!
//! These tests verify round-trip conversion between EDF binary, JSON,
//! and XML formats using the Motor Nerve Conduction example from the
//! EDF+ specification.

use std::fs::File;
use std::io::BufReader;

use european_data_format::{io_edf, io_json, io_xml, EdfFile};

/// Path to the example EDF file.
const EXAMPLE_EDF: &str = "examples/example.edf";
/// Path to the example JSON file.
const EXAMPLE_JSON: &str = "examples/example.json";
/// Path to the example XML file.
const EXAMPLE_XML: &str = "examples/example.xml";

/// Read the example EDF file and return the parsed EdfFile.
fn read_example_edf() -> EdfFile {
    let file = File::open(EXAMPLE_EDF).expect("open example.edf");
    let mut reader = BufReader::new(file);
    io_edf::read_edf(&mut reader).expect("parse example.edf")
}

/// Read the example JSON file and return the parsed EdfFile.
fn read_example_json() -> EdfFile {
    let contents = std::fs::read_to_string(EXAMPLE_JSON).expect("read example.json");
    io_json::from_json(&contents).expect("parse example.json")
}

/// Read the example XML file and return the parsed EdfFile.
fn read_example_xml() -> EdfFile {
    let contents = std::fs::read_to_string(EXAMPLE_XML).expect("read example.xml");
    io_xml::from_xml(&contents).expect("parse example.xml")
}

// ── Header verification ────────────────────────────────────────────

#[test]
fn test_edf_header_fields() {
    let edf = read_example_edf();

    assert_eq!(edf.header.version, "0");
    assert_eq!(
        edf.header.patient_identification,
        "MCH-0234567 F 02-MAY-1951 Haagse_Harry"
    );
    assert_eq!(
        edf.header.recording_identification,
        "Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve."
    );
    assert_eq!(edf.header.start_date, "17.04.01");
    assert_eq!(edf.header.start_time, "11.25.00");
    assert_eq!(edf.header.header_bytes, 768);
    assert_eq!(edf.header.reserved, "EDF+D");
    assert_eq!(edf.header.data_records_count, 2);
    assert_eq!(edf.header.data_record_duration, 0.05);
    assert_eq!(edf.header.signals_count, 2);
}

#[test]
fn test_edf_signal_headers() {
    let edf = read_example_edf();

    // First signal: R APB
    let sh0 = &edf.header.signal_headers[0];
    assert_eq!(sh0.label, "R APB");
    assert_eq!(sh0.transducer_type, "AgAgCl electrodes");
    assert_eq!(sh0.physical_dimension, "mV");
    assert_eq!(sh0.physical_minimum, -100.0);
    assert_eq!(sh0.physical_maximum, 100.0);
    assert_eq!(sh0.digital_minimum, -2048);
    assert_eq!(sh0.digital_maximum, 2047);
    assert_eq!(sh0.prefiltering, "HP:3Hz LP:20kHz");
    assert_eq!(sh0.samples_per_record, 1000);

    // Second signal: EDF Annotations
    let sh1 = &edf.header.signal_headers[1];
    assert_eq!(sh1.label, "EDF Annotations");
    assert_eq!(sh1.digital_minimum, -32768);
    assert_eq!(sh1.digital_maximum, 32767);
    assert_eq!(sh1.samples_per_record, 60);
}

#[test]
fn test_edf_signal_data() {
    let edf = read_example_edf();

    // One ordinary signal (R APB), annotation signal is excluded
    assert_eq!(edf.signals.len(), 1);
    assert_eq!(edf.signals[0].header.label, "R APB");

    // Two data records, each with 1000 samples
    assert_eq!(edf.signals[0].records.len(), 2);
    assert_eq!(edf.signals[0].records[0].sample.len(), 1000);
    assert_eq!(edf.signals[0].records[1].sample.len(), 1000);
}

#[test]
fn test_edf_annotations() {
    let edf = read_example_edf();

    // 4 annotations: 2 time-keeping + 2 content
    assert_eq!(edf.annotations.len(), 4);

    // First time-keeping annotation (onset 0)
    assert_eq!(edf.annotations[0].onset, 0.0);
    assert!(edf.annotations[0].texts.is_empty());

    // First content annotation (wrist stimulation)
    assert_eq!(edf.annotations[1].onset, 0.0);
    assert!(edf.annotations[1].texts[0].contains("wrist"));
    assert!(edf.annotations[1].texts[1].contains("3.8ms"));

    // Second time-keeping annotation (onset 10)
    assert_eq!(edf.annotations[2].onset, 10.0);
    assert!(edf.annotations[2].texts.is_empty());

    // Second content annotation (elbow stimulation)
    assert_eq!(edf.annotations[3].onset, 10.0);
    assert!(edf.annotations[3].texts[0].contains("elbow"));
    assert!(edf.annotations[3].texts[1].contains("55.0m/s"));
}

// ── EDF → JSON round-trip ──────────────────────────────────────────

#[test]
fn test_edf_to_json_matches_example() {
    let edf = read_example_edf();
    let json_from_edf = io_json::to_json(&edf).expect("serialize to JSON");

    let example_json = std::fs::read_to_string(EXAMPLE_JSON).expect("read example.json");

    // Parse both to compare structurally (ignoring whitespace differences)
    let from_edf: serde_json::Value =
        serde_json::from_str(&json_from_edf).expect("parse EDF-derived JSON");
    let from_file: serde_json::Value =
        serde_json::from_str(&example_json).expect("parse example JSON");

    assert_eq!(from_edf, from_file);
}

#[test]
fn test_edf_to_xml_matches_example() {
    let edf = read_example_edf();
    let xml_from_edf = io_xml::to_xml(&edf).expect("serialize to XML");

    let example_xml = std::fs::read_to_string(EXAMPLE_XML).expect("read example.xml");

    // Compare the XML strings (both have the same declaration prefix)
    // Trim trailing newlines for comparison
    assert_eq!(xml_from_edf.trim(), example_xml.trim());
}

// ── JSON → EDF round-trip ──────────────────────────────────────────

#[test]
fn test_json_to_edf_binary_matches() {
    let edf_from_json = read_example_json();

    // Write to EDF binary
    let mut edf_bytes = Vec::new();
    io_edf::write_edf(&edf_from_json, &mut edf_bytes).expect("write EDF from JSON");

    // Read back
    let mut cursor = std::io::Cursor::new(&edf_bytes);
    let round_tripped = io_edf::read_edf(&mut cursor).expect("re-read EDF");

    // Compare structurally
    assert_eq!(edf_from_json.header.version, round_tripped.header.version);
    assert_eq!(
        edf_from_json.header.patient_identification,
        round_tripped.header.patient_identification
    );
    assert_eq!(edf_from_json.signals.len(), round_tripped.signals.len());
    assert_eq!(
        edf_from_json.signals[0].records[0].sample,
        round_tripped.signals[0].records[0].sample
    );
}

// ── XML → EDF round-trip ───────────────────────────────────────────

#[test]
fn test_xml_to_edf_binary_matches() {
    let edf_from_xml = read_example_xml();

    // Write to EDF binary
    let mut edf_bytes = Vec::new();
    io_edf::write_edf(&edf_from_xml, &mut edf_bytes).expect("write EDF from XML");

    // Read back
    let mut cursor = std::io::Cursor::new(&edf_bytes);
    let round_tripped = io_edf::read_edf(&mut cursor).expect("re-read EDF");

    assert_eq!(edf_from_xml.header.version, round_tripped.header.version);
    assert_eq!(edf_from_xml.signals.len(), round_tripped.signals.len());
    assert_eq!(
        edf_from_xml.signals[0].records[0].sample,
        round_tripped.signals[0].records[0].sample
    );
}

// ── Full round-trip: EDF → JSON → EDF ──────────────────────────────

#[test]
fn test_round_trip_edf_json_edf() {
    let original_bytes = std::fs::read(EXAMPLE_EDF).expect("read example.edf");

    // EDF → EdfFile
    let mut cursor = std::io::Cursor::new(&original_bytes);
    let edf = io_edf::read_edf(&mut cursor).expect("parse EDF");

    // EdfFile → JSON → EdfFile
    let json = io_json::to_json(&edf).expect("to JSON");
    let from_json = io_json::from_json(&json).expect("from JSON");

    // EdfFile → EDF bytes
    let mut round_tripped_bytes = Vec::new();
    io_edf::write_edf(&from_json, &mut round_tripped_bytes).expect("write EDF");

    assert_eq!(original_bytes, round_tripped_bytes);
}

// ── Full round-trip: EDF → XML → EDF ───────────────────────────────

#[test]
fn test_round_trip_edf_xml_edf() {
    let original_bytes = std::fs::read(EXAMPLE_EDF).expect("read example.edf");

    // EDF → EdfFile
    let mut cursor = std::io::Cursor::new(&original_bytes);
    let edf = io_edf::read_edf(&mut cursor).expect("parse EDF");

    // EdfFile → XML → EdfFile
    let xml = io_xml::to_xml(&edf).expect("to XML");
    let from_xml = io_xml::from_xml(&xml).expect("from XML");

    // EdfFile → EDF bytes
    let mut round_tripped_bytes = Vec::new();
    io_edf::write_edf(&from_xml, &mut round_tripped_bytes).expect("write EDF");

    assert_eq!(original_bytes, round_tripped_bytes);
}

// ── JSON ↔ XML cross-format ────────────────────────────────────────

#[test]
fn test_json_and_xml_represent_same_data() {
    let from_json = read_example_json();
    let from_xml = read_example_xml();

    assert_eq!(from_json, from_xml);
}

// ── CLI smoke test ─────────────────────────────────────────────────

#[test]
fn test_cli_edf_to_json() {
    let output_path = "/tmp/edf_test_output.json";
    let status = std::process::Command::new("cargo")
        .args(["run", "--", "--input", EXAMPLE_EDF, "--output", output_path])
        .status()
        .expect("run CLI");
    assert!(status.success());

    // Verify the output is valid JSON representing the same data
    let json_str = std::fs::read_to_string(output_path).expect("read output JSON");
    let edf: EdfFile = io_json::from_json(&json_str).expect("parse output JSON");
    assert_eq!(edf.header.version, "0");
    assert_eq!(edf.header.data_records_count, 2);

    std::fs::remove_file(output_path).ok();
}

#[test]
fn test_cli_edf_to_xml() {
    let output_path = "/tmp/edf_test_output.xml";
    let status = std::process::Command::new("cargo")
        .args(["run", "--", "--input", EXAMPLE_EDF, "--output", output_path])
        .status()
        .expect("run CLI");
    assert!(status.success());

    let xml_str = std::fs::read_to_string(output_path).expect("read output XML");
    let edf: EdfFile = io_xml::from_xml(&xml_str).expect("parse output XML");
    assert_eq!(edf.header.version, "0");

    std::fs::remove_file(output_path).ok();
}
