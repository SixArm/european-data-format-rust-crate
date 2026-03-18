# European Data Format (EDF/EDF+) Rust Crate

## Overview

The `european-data-format` crate provides a complete Rust implementation for
reading, writing, and converting European Data Format (EDF and EDF+) files. EDF
is the standard non-proprietary format for time-series data in medical fields,
particularly EEG, EMG, ECG, polysomnography (PSG), and evoked potentials.

This crate supports:

- **EDF binary** (`.edf`) — the native binary format specified in the 1992 EDF
  standard and the 2003 EDF+ extension.
- **JSON** (`.json`) — a human-readable representation via `serde_json`.
- **XML** (`.xml`) — a structured representation via `quick-xml` with serde
  support.

All six conversion directions are supported:

```
EDF <-> JSON
EDF <-> XML
JSON <-> XML
```

Round-trip fidelity is guaranteed: EDF -> JSON -> EDF and EDF -> XML -> EDF
produce byte-identical output.

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Architecture](#architecture)
4. [EDF/EDF+ Specification Summary](#edfedf-specification-summary)
5. [Data Structures](#data-structures)
6. [Reading EDF Files](#reading-edf-files)
7. [Writing EDF Files](#writing-edf-files)
8. [JSON Serialization](#json-serialization)
9. [XML Serialization](#xml-serialization)
10. [Command-Line Interface](#command-line-interface)
11. [Annotations and TALs](#annotations-and-tals)
12. [Error Handling](#error-handling)
13. [Testing](#testing)
14. [Edge Cases and Limitations](#edge-cases-and-limitations)
15. [Examples](#examples)
16. [Related Work](#related-work)

---

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
european-data-format = "0.2"
```

Or use cargo:

```sh
cargo add european-data-format
```

### Dependencies

| Crate        | Purpose                              |
| ------------ | ------------------------------------ |
| `serde`      | Serialization/deserialization derive |
| `serde_json` | JSON reading and writing             |
| `quick-xml`  | XML reading and writing              |
| `clap`       | Command-line argument parsing        |
| `thiserror`  | Custom error type derivation         |

---

## Quick Start

### As a Library

```rust
use european_data_format::{io_edf, io_json, io_xml, EdfFile};
use std::fs::File;
use std::io::BufReader;

// Read an EDF file
let file = File::open("recording.edf").unwrap();
let edf = io_edf::read_edf(&mut BufReader::new(file)).unwrap();

// Inspect header fields
println!("Patient: {}", edf.header.patient_identification);
println!("Signals: {}", edf.header.signals_count);
println!("Duration: {}s per record", edf.header.data_record_duration);

// Convert to JSON
let json = io_json::to_json(&edf).unwrap();
println!("{json}");

// Convert to XML
let xml = io_xml::to_xml(&edf).unwrap();
println!("{xml}");
```

### As a CLI Tool

```sh
# EDF to JSON
cargo run -- --input recording.edf --output recording.json

# EDF to XML
cargo run -- --input recording.edf --output recording.xml

# JSON to EDF
cargo run -- --input recording.json --output recording.edf

# XML to JSON
cargo run -- --input recording.xml --output recording.json
```

---

## Architecture

The crate is organized into the following modules:

```
src/
  lib.rs          — Module declarations and public re-exports
  error.rs        — EdfError enum via thiserror
  header.rs       — EdfHeader, EdfSignalHeader structs + EDF binary parsing
  annotation.rs   — TAL and annotation parsing (EDF+)
  edf_file.rs     — Top-level EdfFile, EdfSignal, EdfDataRecord structs
  io_edf.rs       — Read/write EDF binary files
  io_json.rs      — Read/write JSON via serde_json
  io_xml.rs       — Read/write XML via quick-xml + serde
  main.rs         — CLI via clap
tests/
  integration_test.rs  — Round-trip and example file tests
  edge_case_test.rs    — Edge case integration tests
examples/
  generate_examples.rs — Generates example files from spec section 3.7
  example.edf          — Motor Nerve Conduction example (EDF binary)
  example.json         — Same data in JSON
  example.xml          — Same data in XML
```

### Data Flow

```
                    ┌──────────┐
                    │ EDF File │
                    │ (binary) │
                    └────┬─────┘
                         │
              io_edf::read_edf()
                         │
                         ▼
                    ┌──────────┐
                    │ EdfFile  │  ← In-memory representation
                    │ (struct) │
                    └──┬───┬───┘
                       │   │
        io_json::to_json() io_xml::to_xml()
                       │   │
                       ▼   ▼
                 ┌──────┐ ┌─────┐
                 │ JSON │ │ XML │
                 └──────┘ └─────┘
```

All conversions go through the `EdfFile` in-memory representation. This
guarantees that JSON and XML representations are structurally identical and
that round-trips produce consistent results.

---

## EDF/EDF+ Specification Summary

### EDF (1992)

The European Data Format was introduced in 1992 as a standard for EEG and
polysomnography recordings. Key properties:

- **Contiguous recordings only**: data records are consecutive with no gaps.
- **16-bit signed integers**: samples are stored as little-endian two's
  complement 2-byte integers.
- **ASCII headers**: all header fields use printable US-ASCII (bytes 32-126).
- **Fixed header layout**: 256-byte main header + 256 bytes per signal.

### EDF+ (2003)

EDF+ extends EDF with backward-compatible enhancements:

- **Discontinuous recordings** (`EDF+D`): data records can have time gaps,
  with actual start times specified by timekeeping annotations.
- **Contiguous recordings** (`EDF+C`): same as EDF but with annotations.
- **Annotations**: time-stamped text annotations stored in special "EDF
  Annotations" signals using TAL (Time-stamped Annotation List) encoding.
- **Standardized identification**: structured subfields for patient ID (code,
  sex, birthdate, name) and recording ID (Startdate, admin code, technician,
  equipment).
- **UTF-8 in annotations**: annotation texts support full Unicode via UTF-8
  encoding (headers remain ASCII-only).

### Header Layout

#### Fixed Header (256 bytes)

| Bytes | Field                    | Description                                              |
| ----- | ------------------------ | -------------------------------------------------------- |
| 8     | Version                  | Always `"0"` (space-padded to 8 bytes)                   |
| 80    | Patient identification   | EDF+: `code sex birthdate name` (spaces between fields)  |
| 80    | Recording identification | EDF+: `Startdate dd-MMM-yyyy admin tech equipment`       |
| 8     | Start date               | `dd.mm.yy` format. Y2K: 85-99=1985-1999, 00-84=2000-2084 |
| 8     | Start time               | `hh.mm.ss` format (local time at patient location)       |
| 8     | Header bytes             | Total header size: `256 + signals_count * 256`           |
| 44    | Reserved                 | EDF+: starts with `"EDF+C"` or `"EDF+D"`; EDF: spaces    |
| 8     | Data records count       | Number of data records; `-1` during recording            |
| 8     | Data record duration     | Duration per data record in seconds                      |
| 4     | Signals count            | Number of signals (including annotation signals)         |

#### Per-Signal Header (256 bytes per signal, interleaved across signals)

| Bytes | Field              | Description                                           |
| ----- | ------------------ | ----------------------------------------------------- |
| 16    | Label              | Signal name, e.g. `"EEG Fpz-Cz"`, `"EDF Annotations"` |
| 80    | Transducer type    | Sensor description, e.g. `"AgAgCl electrode"`         |
| 8     | Physical dimension | Unit of measurement, e.g. `"uV"`, `"mV"`              |
| 8     | Physical minimum   | Minimum physical value (for scaling)                  |
| 8     | Physical maximum   | Maximum physical value (for scaling)                  |
| 8     | Digital minimum    | Minimum digital value (for scaling)                   |
| 8     | Digital maximum    | Maximum digital value (for scaling)                   |
| 80    | Prefiltering       | Filter description, e.g. `"HP:0.1Hz LP:75Hz N:50Hz"`  |
| 8     | Samples per record | Number of samples per data record for this signal     |
| 32    | Reserved           | Reserved (space-padded)                               |

### Digital-to-Physical Conversion

Samples are stored as raw 16-bit digital values. To convert to physical units:

```
physical = (digital - digital_min) / (digital_max - digital_min)
           * (physical_max - physical_min) + physical_min
```

The gain and offset are:

```
gain   = (physical_max - physical_min) / (digital_max - digital_min)
offset = physical_min - gain * digital_min
```

EDF+ requires `digital_max > digital_min` and `physical_max != physical_min`.

---

## Data Structures

### EdfFile

The top-level structure representing a complete EDF/EDF+ file in memory.

```rust
pub struct EdfFile {
    pub header: EdfHeader,           // File header (including per-signal headers)
    pub signals: Vec<EdfSignal>,     // Ordinary (non-annotation) signal data
    pub annotations: Vec<EdfAnnotation>, // EDF+ annotations (from annotation signals)
}
```

**Important**: `header.signal_headers` includes headers for ALL signals in the
file (including annotation signals), while `signals` contains only ordinary
(non-annotation) signals. This separation allows clean serialization while
preserving the full header for byte-perfect round-trips.

### EdfHeader

The file-level header containing metadata and per-signal headers.

```rust
pub struct EdfHeader {
    pub version: String,                   // Always "0"
    pub patient_identification: String,    // Patient ID (80 bytes in EDF)
    pub recording_identification: String,  // Recording ID (80 bytes in EDF)
    pub start_date: String,                // dd.mm.yy
    pub start_time: String,                // hh.mm.ss
    pub header_bytes: usize,               // 256 + signals_count * 256
    pub reserved: String,                  // "EDF+C", "EDF+D", or empty
    pub data_records_count: i64,           // -1 during recording
    pub data_record_duration: f64,         // Seconds per data record
    pub signals_count: usize,              // Total signal count
    pub signal_headers: Vec<EdfSignalHeader>,
}
```

### EdfSignalHeader

Per-signal metadata describing the signal's properties and calibration.

```rust
pub struct EdfSignalHeader {
    pub label: String,              // 16 bytes: signal name
    pub transducer_type: String,    // 80 bytes: sensor description
    pub physical_dimension: String, // 8 bytes: measurement unit
    pub physical_minimum: f64,      // Physical range minimum
    pub physical_maximum: f64,      // Physical range maximum
    pub digital_minimum: i32,       // Digital range minimum
    pub digital_maximum: i32,       // Digital range maximum
    pub prefiltering: String,       // 80 bytes: filter description
    pub samples_per_record: usize,  // Samples per data record
    pub reserved: String,           // 32 bytes: reserved
}
```

Methods:

- `is_annotation() -> bool` — returns `true` if the label (trimmed) equals
  `"EDF Annotations"`.

### EdfSignal

An ordinary signal's data, grouped by data record.

```rust
pub struct EdfSignal {
    pub header: EdfSignalHeader,       // Signal metadata
    pub records: Vec<EdfDataRecord>,   // Sample data per record
}
```

### EdfDataRecord

A single data record's samples for one signal.

```rust
pub struct EdfDataRecord {
    pub sample: Vec<i16>,  // Raw digital sample values
}
```

### EdfAnnotation

A parsed EDF+ annotation with onset time, optional duration, and text.

```rust
pub struct EdfAnnotation {
    pub onset: f64,               // Seconds relative to file start
    pub duration: Option<f64>,    // Optional duration in seconds
    pub texts: Vec<String>,       // Annotation texts (empty for timekeeping)
}
```

**Timekeeping annotations** have empty `texts` and serve to specify the start
time of each data record. The first annotation in each data record must be a
timekeeping annotation.

---

## Reading EDF Files

### From Binary EDF

```rust
use european_data_format::io_edf;
use std::fs::File;
use std::io::BufReader;

let file = File::open("recording.edf")?;
let mut reader = BufReader::new(file);
let edf = io_edf::read_edf(&mut reader)?;
```

The reader:

1. Parses the 256-byte fixed header.
2. Parses `signals_count * 256` bytes of per-signal headers.
3. Reads `data_records_count` data records.
4. For each record, reads `samples_per_record * 2` bytes per signal.
5. Separates ordinary signals from "EDF Annotations" signals.
6. Parses TALs from annotation signal bytes into `EdfAnnotation` structs.

### From JSON

```rust
use european_data_format::io_json;

let json_str = std::fs::read_to_string("recording.json")?;
let edf = io_json::from_json(&json_str)?;
```

### From XML

```rust
use european_data_format::io_xml;

let xml_str = std::fs::read_to_string("recording.xml")?;
let edf = io_xml::from_xml(&xml_str)?;
```

---

## Writing EDF Files

### To Binary EDF

```rust
use european_data_format::io_edf;
use std::fs::File;
use std::io::BufWriter;

let file = File::create("output.edf")?;
let mut writer = BufWriter::new(file);
io_edf::write_edf(&edf, &mut writer)?;
```

The writer:

1. Writes the header (fixed + per-signal) using space-padded ASCII fields.
2. For each data record, writes ordinary signal samples as little-endian i16.
3. Encodes annotations back into TAL format for "EDF Annotations" signals.
4. Pads annotation bytes with NUL (0x00) to fill the allocated space.

### To JSON

```rust
use european_data_format::io_json;

let json = io_json::to_json(&edf)?;
std::fs::write("output.json", format!("{json}\n"))?;
```

Produces pretty-printed JSON with `serde_json::to_string_pretty`.

### To XML

```rust
use european_data_format::io_xml;

let xml = io_xml::to_xml(&edf)?;
std::fs::write("output.xml", format!("{xml}\n"))?;
```

Produces XML with an `<?xml version="1.0" encoding="UTF-8"?>` declaration
and `<EdfFile>` as the root element.

---

## JSON Serialization

The JSON representation mirrors the `EdfFile` struct directly:

```json
{
  "header": {
    "version": "0",
    "patient_identification": "MCH-0234567 F 02-MAY-1951 Haagse_Harry",
    "recording_identification": "Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve.",
    "start_date": "17.04.01",
    "start_time": "11.25.00",
    "header_bytes": 768,
    "reserved": "EDF+D",
    "data_records_count": 2,
    "data_record_duration": 0.05,
    "signals_count": 2,
    "signal_headers": [ ... ]
  },
  "signals": [
    {
      "header": { ... },
      "records": [
        { "sample": [0, 1, -5, 12, ...] },
        { "sample": [3, -7, 8, 15, ...] }
      ]
    }
  ],
  "annotations": [
    { "onset": 0.0 },
    { "onset": 0.0, "texts": ["Stimulus right wrist..."] },
    { "onset": 10.0 },
    { "onset": 10.0, "texts": ["Stimulus right elbow..."] }
  ]
}
```

**Serde behavior**:

- `duration: None` is omitted from JSON (`skip_serializing_if = "Option::is_none"`).
- `texts: []` is omitted from JSON (`skip_serializing_if = "Vec::is_empty"`).
- Both fields have `#[serde(default)]` so they deserialize correctly when absent.

---

## XML Serialization

The XML representation uses `<EdfFile>` as the root element:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<EdfFile>
  <header>
    <version>0</version>
    <patient_identification>MCH-0234567 F 02-MAY-1951 Haagse_Harry</patient_identification>
    ...
    <signal_headers>
      <EdfSignalHeader>
        <label>R APB</label>
        ...
      </EdfSignalHeader>
    </signal_headers>
  </header>
  <signals>
    <EdfSignal>
      <header>...</header>
      <records>
        <EdfDataRecord>
          <sample>0</sample>
          <sample>1</sample>
          ...
        </EdfDataRecord>
      </records>
    </EdfSignal>
  </signals>
  <annotations>
    <EdfAnnotation>
      <onset>0</onset>
    </EdfAnnotation>
    ...
  </annotations>
</EdfFile>
```

---

## Command-Line Interface

The CLI detects formats from file extensions (`.edf`, `.json`, `.xml`).

### Usage

```
edf --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>    Input file path (.edf, .xml, or .json)
  -o, --output <OUTPUT>  Output file path (.edf, .xml, or .json)
  -h, --help             Print help
  -V, --version          Print version
```

### Supported Conversions

| Input   | Output  | Description                |
| ------- | ------- | -------------------------- |
| `.edf`  | `.json` | Convert EDF binary to JSON |
| `.edf`  | `.xml`  | Convert EDF binary to XML  |
| `.json` | `.edf`  | Convert JSON to EDF binary |
| `.xml`  | `.edf`  | Convert XML to EDF binary  |
| `.json` | `.xml`  | Convert JSON to XML        |
| `.xml`  | `.json` | Convert XML to JSON        |

### Examples

```sh
# Convert EDF to JSON (pretty-printed)
cargo run -- -i examples/example.edf -o /tmp/output.json

# Convert EDF to XML
cargo run -- -i examples/example.edf -o /tmp/output.xml

# Convert JSON back to EDF (byte-perfect round-trip)
cargo run -- -i /tmp/output.json -o /tmp/round_trip.edf

# Convert between JSON and XML
cargo run -- -i /tmp/output.json -o /tmp/output.xml
cargo run -- -i /tmp/output.xml -o /tmp/output.json
```

---

## Annotations and TALs

### TAL Format

EDF+ annotations are encoded as Time-stamped Annotation Lists (TALs) in
"EDF Annotations" signals. Each TAL has the following binary format:

```
+Onset\x15Duration\x14Annotation1\x14Annotation2\x14\x00
```

Where:

- **Onset**: seconds relative to file start, preceded by `+` or `-`.
  Examples: `+0`, `+180.5`, `-0.065`.
- **`\x15`** (byte 21, NAK): separates onset from duration. Omitted if no
  duration is specified.
- **Duration**: seconds (no sign). Examples: `25.5`, `0.2`, `30`.
- **`\x14`** (byte 20, DC4): separates annotations. Always follows onset
  (or duration, if present). Each annotation text is followed by `\x14`.
- **`\x00`** (byte 0, NUL): terminates the TAL.

### Timekeeping TALs

The first TAL in each data record is a **timekeeping annotation** with empty
text. Its onset specifies the actual start time of that data record:

```
+567\x14\x14\x00   (data record starts 567s after file start)
```

For contiguous recordings (EDF+C), timekeeping onsets are sequential:
`+0`, `+1`, `+2`, ... (for 1-second records).

For discontinuous recordings (EDF+D), timekeeping onsets reflect actual times
with gaps: `+0`, `+10`, `+120`, ... (time gaps between stimuli).

### Parsing and Encoding

```rust
use european_data_format::annotation::{parse_tals, encode_tals};
use european_data_format::EdfAnnotation;

// Parse TAL bytes
let bytes = b"+180\x14Lights off\x14Close door\x14\x00";
let annotations = parse_tals(bytes).unwrap();
assert_eq!(annotations[0].onset, 180.0);
assert_eq!(annotations[0].texts, vec!["Lights off", "Close door"]);

// Encode annotations to TAL bytes
let encoded = encode_tals(&annotations, 120); // 120 bytes total
assert_eq!(encoded.len(), 120); // NUL-padded to specified length
```

### Duration Semantics

- `duration: None` — no duration specified (unspecified).
- `duration: Some(0.0)` — explicitly instantaneous event (point in time).
- `duration: Some(30.0)` — event lasting 30 seconds (e.g., a sleep epoch).

These are clinically distinct. A seizure onset is a point event (None),
while a seizure lasting 45 seconds has `duration: Some(45.0)`.

---

## Error Handling

All errors use the `EdfError` enum (derived via `thiserror`):

```rust
pub enum EdfError {
    Io(std::io::Error),                                    // I/O errors
    InvalidHeader { field: String, message: String },      // Bad header field
    InvalidSignalHeader { index: usize, field: String, message: String }, // Bad signal header
    InvalidDataRecord { index: usize, message: String },   // Bad data record
    InvalidAnnotation { message: String },                 // Bad TAL/annotation
    Parse { message: String },                             // Generic parse error
    XmlDe(quick_xml::DeError),                             // XML deserialization
    XmlSe(quick_xml::SeError),                             // XML serialization
    Json(serde_json::Error),                               // JSON error
}
```

All public functions return `Result<T, EdfError>`. Error messages include
context about which field or record caused the error:

```
Invalid header: data_records_count: invalid digit found in string
Invalid signal header at index 2: physical_minimum: invalid float literal
Invalid data record at index 5: failed to read signal 0: unexpected end of file
Invalid annotation: onset must start with '+' or '-', got '180'
```

---

## Testing

The crate includes comprehensive tests at multiple levels:

### Unit Tests (90 tests across modules)

- **header.rs**: Header parsing, field padding/truncation, format_number
  precision, ASCII constraints, calibration validation, date formatting,
  header size consistency, annotation label detection.
- **annotation.rs**: TAL parsing (timekeeping, content, duration, negative
  onset, multiple TALs), encoding, round-trips, precision, UTF-8 text.
- **io_edf.rs**: Binary round-trip, contiguous/discontinuous recordings,
  16-bit sample boundaries, little-endian byte order, multiple signals,
  annotation-only files, truncated input handling.
- **io_json.rs**: JSON round-trip, field presence, invalid input.
- **io_xml.rs**: XML round-trip, element presence, invalid input.
- **edf_file.rs**: Struct construction, signal samples.
- **error.rs**: Error display formatting.

### Integration Tests (27 tests)

- **integration_test.rs** (13 tests): Header verification, signal data,
  annotation parsing, EDF->JSON/XML comparison with example files, JSON->EDF
  and XML->EDF round-trips, byte-perfect EDF->JSON->EDF and EDF->XML->EDF
  round-trips, JSON<->XML structural equivalence, CLI smoke tests.
- **edge_case_test.rs** (14 tests): Cross-format round-trips for all 13 edge
  case categories, extreme sample values across all formats, discontinuous
  time gap preservation, header size consistency, multi-channel data isolation,
  full EDF->JSON->XML->EDF pipeline.

### Doc Tests (10 tests)

All public API examples in rustdoc comments are compiled and executed.

### Running Tests

```sh
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_test
cargo test --test edge_case_test

# Run only doc tests
cargo test --doc

# Run a specific test by name
cargo test test_round_trip_edf_json_edf
```

### Test Verification Requirements

Per the project specification, the following must all pass:

- **EDF -> JSON -> EDF**: byte-perfect round-trip
- **EDF -> XML -> EDF**: byte-perfect round-trip
- **JSON -> XML**: structurally identical (`from_json == from_xml`)
- **CLI**: all 6 conversion directions work
- **Documentation**: `cargo doc` builds with zero warnings

---

## Edge Cases and Limitations

See [edge-cases/index.md](edge-cases/index.md) for detailed coverage.

### Currently Handled

| Edge Case                  | Status  | Description                                            |
| -------------------------- | ------- | ------------------------------------------------------ |
| Contiguous data (EDF)      | Handled | EDF+C files round-trip with sequential timing          |
| Discontinuous data (EDF+D) | Handled | Time gaps preserved through all formats                |
| 16-bit sample storage      | Handled | Full i16 range (-32768 to 32767) round-trips correctly |
| Little-endian byte order   | Handled | Correct LE encoding verified                           |
| ASCII header fields        | Partial | `from_utf8_lossy` replaces non-ASCII silently          |
| Length-restricted fields   | Handled | Padding and truncation work correctly                  |
| Annotation TAL parsing     | Handled | All TAL variants parse and encode correctly            |
| Duration semantics         | Handled | None vs Some(0.0) preserved in all formats             |
| Date format (dd.mm.yy)     | Handled | String preserved through round-trips                   |
| Reserved field (EDF+C/D)   | Handled | All variants round-trip correctly                      |
| data_records_count = -1    | Handled | Rejected with clear error message                      |
| Multiple signals           | Handled | Correct interleaving and separation                    |
| Truncated input            | Handled | Clear error messages on short reads                    |

### Known Limitations

| Limitation                 | Description                                                    |
| -------------------------- | -------------------------------------------------------------- |
| No ASCII validation        | Non-ASCII bytes in headers are silently replaced, not rejected |
| No calibration validation  | Inverted or zero digital ranges are not detected               |
| No date validation         | Invalid dates (e.g., "32.13.99") are not rejected              |
| No header size validation  | Inconsistent `header_bytes` values are not detected            |
| No data record size limit  | The 61,440-byte EDF+ limit is not enforced                     |
| No floating-point EDF+     | Extended floating-point data format is not supported           |
| No field length validation | Oversized fields are silently truncated on write               |

These are tracked as future work in the project's task list.

---

## Examples

### Motor Nerve Conduction (from EDF+ spec section 3.7)

The included example files represent a right Median Nerve conduction velocity
study, as described in the EDF+ specification:

- **Patient**: MCH-0234567 F 02-MAY-1951 Haagse_Harry
- **Recording**: Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve.
- **Signals**: "R APB" (1000 samples/record, 50ms window) + "EDF Annotations"
  (60 samples/record)
- **Data records**: 2 (wrist stimulation + elbow stimulation), EDF+D
  (discontinuous)
- **Duration**: 0.050 seconds per data record

Record 1 (onset t=0): Wrist stimulation — 0.2ms pulse, 8.2mA, at 6.5cm from
recording site. Response: 7.2mV at 3.8ms latency.

Record 2 (onset t=10): Elbow stimulation — 0.2ms pulse, 15.3mA, at 28.5cm
from recording site. Response: 7.2mV at 7.8ms latency. Conduction velocity:
55.0 m/s.

### Generating Example Files

```sh
cargo run --example generate_examples
```

This produces `examples/example.edf`, `examples/example.json`, and
`examples/example.xml` from the Motor Nerve Conduction example.

### Programmatic Example

```rust
use european_data_format::*;
use european_data_format::edf_file::{EdfDataRecord, EdfSignal};

// Create an EDF+ file from scratch
let edf = EdfFile {
    header: EdfHeader {
        version: "0".into(),
        patient_identification: "PAT-001 M 15-MAR-1980 John_Doe".into(),
        recording_identification: "Startdate 15-MAR-2020 LAB01 DR.SMITH Equipment_v2".into(),
        start_date: "15.03.20".into(),
        start_time: "14.30.00".into(),
        header_bytes: 768, // 256 + 2 * 256
        reserved: "EDF+C".into(),
        data_records_count: 1,
        data_record_duration: 1.0,
        signals_count: 2,
        signal_headers: vec![
            EdfSignalHeader {
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
                samples_per_record: 30,
                reserved: String::new(),
            },
        ],
    },
    signals: vec![EdfSignal {
        header: EdfSignalHeader {
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
        },
        records: vec![EdfDataRecord {
            sample: vec![0i16; 256], // flat line for demo
        }],
    }],
    annotations: vec![
        EdfAnnotation { onset: 0.0, duration: None, texts: vec![] },
        EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec!["Recording start".into()],
        },
    ],
};

// Write to all three formats
let mut edf_bytes = Vec::new();
io_edf::write_edf(&edf, &mut edf_bytes).unwrap();

let json = io_json::to_json(&edf).unwrap();
let xml = io_xml::to_xml(&edf).unwrap();
```

---

## Related Work

### Libraries

| Library                                              | Language | Description                    |
| ---------------------------------------------------- | -------- | ------------------------------ |
| [EDFlib](https://gitlab.com/Teuniz/EDFlib)           | C/C++    | Read/write EDF+ and BDF+ files |
| [EDFlib-Java](https://gitlab.com/Teuniz/EDFlib-Java) | Java     | Read/write EDF+ and BDF+ files |
| [PyEDFlib](https://github.com/holgern/pyedflib)      | Python   | Read/write EDF/EDF+/BDF files  |
| [edfplus](https://github.com/2986002971/edfplus)     | Rust     | Read/write EDF+ files          |

### Specifications

- [EDF+ specification](https://www.edfplus.info/specs/edfplus.html) — full
  protocol specification including header layout, TAL encoding, and examples.
- [EDF standard texts](http://www.edfplus.info/specs/edftexts.html) — standard
  labels, transducer types, and polarity rules for clinical signals.
- [Original EDF paper](<https://doi.org/10.1016/0013-4694(92)90009-7>) — Kemp et
  al., "A simple format for exchange of digitized polygraphic recordings,"
  Electroencephalography and Clinical Neurophysiology, 1992.
