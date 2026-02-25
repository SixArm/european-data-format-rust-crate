# Plan to implement European Data Format (EDF/EDF+) Rust crate

## Context

Create a European Data Format (EDF/EDF+) Rust crate that can read/write EDF
binary files and convert to/from XML and JSON.

## Phase 1: Project Setup & Dependencies

### Cargo.toml

Rust edition = "2024".

Add crates:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
quick-xml = { version = "0.37", features = ["serialize"] }
clap = { version = "4", features = ["derive"] }
thiserror = "2"

[dev-dependencies]
assertables = "9.8.6" # Assert macros for better testing
cargo-diet = "1.2.7" # Make your crate lean
cargo-deny = "0.19.0" # Lint dependencies
cargo-dist = "0.30.4" # Distribution builder for release engineering
cargo-release = "1.0.0" # Release automation
cargo-semver-checks = "0.46.0" # Scan for semantic version errors
regex = "1.12.3" # Regular expressions parser, compiler, and executer
rustdoc-md = "0.2.0" # Convert Rust documentation JSON into clean, organized Markdown files.
```

### File structure

```txt
src/
  lib.rs          — re-exports modules
  error.rs        — EdfError enum via thiserror
  header.rs       — EdfHeader, EdfSignalHeader structs + EDF binary parsing
  data_record.rs  — EdfDataRecord struct + binary parsing
  annotation.rs   — TAL and annotation parsing (EDF+)
  edf_file.rs     — Top-level EdfFile struct (header + data records)
  io_edf.rs       — Read/write EDF binary files
  io_xml.rs       — Read/write XML via quick-xml + serde
  io_json.rs      — Read/write JSON via serde_json
  main.rs         — CLI via clap
tests/
  integration_test.rs — Round-trip tests
examples/
  example.edf     — Motor Nerve Conduction example from spec
  example.xml     — Same data in XML
  example.json    — Same data in JSON
```

## Phase 2: Data Structures

### EdfFile (edf_file.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdfFile {
    pub header: EdfHeader,
    pub signals: Vec<EdfSignal>,
    pub annotations: Vec<EdfAnnotation>,
}
```

### EdfHeader (header.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdfHeader {
    pub version: String,                    // 8 bytes, always "0"
    pub patient_identification: String,     // 80 bytes
    pub recording_identification: String,   // 80 bytes
    pub start_date: String,                 // 8 bytes, dd.mm.yy
    pub start_time: String,                 // 8 bytes, hh.mm.ss
    pub header_bytes: usize,               // 8 bytes
    pub reserved: String,                   // 44 bytes ("EDF+C" or "EDF+D")
    pub data_records_count: i64,           // 8 bytes (-1 if unknown)
    pub data_record_duration: f64,         // 8 bytes, seconds
    pub signals_count: usize,              // 4 bytes
    pub signal_headers: Vec<EdfSignalHeader>,
}
```

### EdfSignalHeader (header.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdfSignalHeader {
    pub label: String,                  // 16 bytes
    pub transducer_type: String,        // 80 bytes
    pub physical_dimension: String,     // 8 bytes
    pub physical_minimum: f64,          // 8 bytes
    pub physical_maximum: f64,          // 8 bytes
    pub digital_minimum: i32,           // 8 bytes
    pub digital_maximum: i32,           // 8 bytes
    pub prefiltering: String,           // 80 bytes
    pub samples_per_record: usize,      // 8 bytes
    pub reserved: String,               // 32 bytes
}
```

### EdfSignal (edf_file.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdfSignal {
    pub header: EdfSignalHeader,
    pub samples: Vec<Vec<i16>>,  // samples[record_index][sample_index]
}
```

### EdfAnnotation (annotation.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdfAnnotation {
    pub onset: f64,
    pub duration: Option<f64>,
    pub texts: Vec<String>,
}
```

## Phase 3: Error Handling (error.rs)

```rust
#[derive(Debug, thiserror::Error)]
pub enum EdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid header: {field}: {message}")]
    InvalidHeader { field: String, message: String },
    #[error("Invalid signal header at index {index}: {field}: {message}")]
    InvalidSignalHeader { index: usize, field: String, message: String },
    #[error("Invalid data record at index {index}: {message}")]
    InvalidDataRecord { index: usize, message: String },
    #[error("Invalid annotation: {message}")]
    InvalidAnnotation { message: String },
    #[error("Parse error: {message}")]
    Parse { message: String },
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

### Phase 4: EDF Binary I/O (io_edf.rs)

Reading

1. Read 256 bytes → parse fixed header fields (trim ASCII spaces)
2. Parse signals_count → read signals_count \* 256 bytes for signal headers
3. Read per-signal fields in EDF order (all labels, then all transducers, etc.)
4. For each data record: read samples_per_record \* 2 bytes per signal as little-endian i16
5. Detect "EDF Annotations" signals by label, parse TALs from annotation signals
6. Separate annotation signals from ordinary signals in the returned EdfFile

Writing

1. Format header fields, pad with ASCII spaces to exact field widths
2. Write per-signal headers in EDF interleaved order
3. Write data records: ordinary signal samples as little-endian i16
4. Encode annotations back into "EDF Annotations" signal bytes

Phase 5: XML & JSON I/O (io_xml.rs, io_json.rs)

XML (io_xml.rs)

- pub fn to_xml(edf: &EdfFile) -> Result<String, EdfError> — uses quick_xml::se::to_string
- pub fn from_xml(xml: &str) -> Result<EdfFile, EdfError> — uses quick_xml::de::from_str
- Root element: <EdfFile>

JSON (io_json.rs)

- pub fn to_json(edf: &EdfFile) -> Result<String, EdfError> — uses serde_json::to_string_pretty
- pub fn from_json(json: &str) -> Result<EdfFile, EdfError> — uses serde_json::from_str

### Phase 6: CLI (main.rs)

```rust
#[derive(Parser)]
#[command(name = "edf", about = "European Data Format (EDF/EDF+) converter")]
struct Cli {
    /// Input file path (.edf, .xml, or .json)
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path (.edf, .xml, or .json)
    #[arg(short, long)]
    output: PathBuf,
}
```

Format detection by file extension. Read input → convert to EdfFile → write output.

### Phase 7: Example Files

Use the Motor Nerve Conduction example from the EDF+ spec (section 3.7 of README.md):

- Patient: MCH-0234567 F 02-MAY-1951 Haagse_Harry
- Recording: Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve.
- 2 signals: "R APB" (1000 samples/record) + "EDF Annotations" (60 samples/record)
- 2 data records (wrist + elbow stimulation)
- Duration: 0.050s per record, EDF+D (discontinuous)

Generate examples/example.edf programmatically from a helper binary, then derive examples/example.xml and examples/example.json from it.

### Phase 8: Testing

Unit Tests (in each module)

- Header parsing: valid/invalid fields, boundary values, trimming
- Signal header parsing: per-field validation
- Annotation/TAL parsing: various TAL formats from spec examples
- Error conditions: truncated files, invalid values, division-by-zero checks

Integration Tests (tests/integration_test.rs)

- Read examples/example.edf → verify header/signal fields match expected values
- example.edf → XML → compare with examples/example.xml
- example.edf → JSON → compare with examples/example.json
- examples/example.xml → EdfFile → EDF bytes → compare with examples/example.edf
- examples/example.json → EdfFile → EDF bytes → compare with examples/example.edf
- Round-trip: EDF → JSON → EDF (bytes match)
- Round-trip: EDF → XML → EDF (bytes match)

Implementation Order

1. Cargo.toml — add dependencies
2. error.rs — error types (everything else depends on this)
3. header.rs — EdfHeader + EdfSignalHeader structs with serde derives
4. annotation.rs — EdfAnnotation struct + TAL parsing
5. edf_file.rs — EdfFile + EdfSignal structs
6. io_edf.rs — EDF binary read/write (core functionality)
7. io_json.rs — JSON read/write
8. io_xml.rs — XML read/write
9. lib.rs — module declarations and re-exports
10. main.rs — CLI with clap
11. Example files — generate example.edf, derive .xml/.json
12. Unit tests — in each module
13. Integration tests — round-trip verification

Verification

- cargo build — compiles without errors
- cargo test — all unit + integration tests pass
- cargo run -- --input examples/example.edf --output /tmp/test.json — converts EDF to JSON
- cargo run -- --input examples/example.edf --output /tmp/test.xml — converts EDF to XML
- cargo doc --open — verify rustdoc documentation renders correctly
