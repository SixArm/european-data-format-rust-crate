# Tasks: Fix Edge Cases in EDF/EDF+ Crate

## Phase 1: Validation Infrastructure

- [ ] **Task 1.1**: Add `EdfHeader::is_edf_plus()` and `EdfHeader::is_discontinuous()` methods
  - File: `src/header.rs`
  - Check reserved field for "EDF+C" / "EDF+D"

- [ ] **Task 1.2**: Add `EdfHeader::validate()` method (skeleton)
  - File: `src/header.rs`
  - Returns `Result<(), EdfError>`
  - Called from `read_edf` and `write_edf`

- [ ] **Task 1.3**: Add `EdfFile::validate()` method (skeleton)
  - File: `src/edf_file.rs`
  - Validates file-level constraints (signals match header, etc.)

## Phase 2: High Priority — Data Correctness

- [ ] **Task 2.1**: ASCII validation for header fields (Edge Case 6)
  - File: `src/header.rs`
  - Add `validate_ascii()` helper
  - Validate all string fields contain only ASCII (bytes 32-126)
  - Unit tests for ASCII/non-ASCII header fields

- [ ] **Task 2.2**: Calibration validation (Edge Case 8)
  - File: `src/header.rs`
  - Validate digital_minimum < digital_maximum
  - Validate physical_minimum < physical_maximum (for non-annotation signals)
  - Validate annotation signal calibration (digital_min=-32768, digital_max=32767)
  - Add `EdfSignalHeader::gain()` and `EdfSignalHeader::offset()` helpers
  - Unit tests for valid/invalid calibration values

- [ ] **Task 2.3**: Date/time format validation (Edge Case 11)
  - File: `src/header.rs`
  - Validate start_date format: dd.mm.yy with valid ranges
  - Validate start_time format: hh.mm.ss with valid ranges
  - Unit tests for valid/invalid dates and times

- [ ] **Task 2.4**: Header size validation (Edge Case 12)
  - File: `src/header.rs`
  - Validate header_bytes == 256 + signals_count * 256
  - Unit tests for consistent/inconsistent header sizes

## Phase 3: Medium Priority — Spec Compliance

- [ ] **Task 3.1**: EDF/EDF+ mode detection and validation (Edge Cases 1, 9)
  - File: `src/header.rs`, `src/io_edf.rs`
  - Validate reserved field values ("EDF+C", "EDF+D", or empty/spaces for plain EDF)
  - Validate that annotation signals only exist in EDF+ files
  - Unit tests for mode detection

- [ ] **Task 3.2**: EDF+D discontinuous data validation (Edge Case 2)
  - File: `src/io_edf.rs`, `src/annotation.rs`
  - Validate timekeeping TAL exists as first TAL in each data record for EDF+D
  - Ensure write_edf generates timekeeping TALs for EDF+D
  - Tests for discontinuous recordings with gaps

- [ ] **Task 3.3**: Data record size limit validation (Edge Case 4)
  - File: `src/header.rs`
  - Validate sum(samples_per_record * 2) <= 61440 for EDF+ files
  - Unit tests at boundary (61440 bytes, 61442 bytes)

- [ ] **Task 3.4**: Field length validation (Edge Case 7)
  - File: `src/header.rs`
  - Validate all string fields fit within their EDF spec widths
  - Log/warn on truncation during write
  - Unit tests for oversized labels, transducer types, etc.

- [ ] **Task 3.5**: Annotation edge cases (Edge Case 10)
  - File: `src/annotation.rs`, `src/io_edf.rs`
  - Test zero duration vs None duration
  - Test annotations at exact record boundaries
  - Test empty annotation texts
  - Test multiple annotation signals

## Phase 4: Low Priority — Quality of Life

- [ ] **Task 4.1**: Sample count validation (Edge Case 3)
  - File: `src/io_edf.rs`
  - Validate actual sample count matches samples_per_record in header
  - Validate samples_per_record > 0

- [ ] **Task 4.2**: Channel access helpers (Edge Case 13)
  - File: `src/edf_file.rs`, `src/header.rs`
  - Add `EdfSignal::all_samples() -> Vec<i16>`
  - Add `EdfSignal::physical_samples() -> Vec<f64>`
  - Add `EdfSignalHeader::gain()` and `EdfSignalHeader::offset()`
  - Unit tests for conversions

- [ ] **Task 4.3**: Documentation update (Edge Case 5)
  - Add rustdoc note about floating-point EDF+ not being supported
  - Ensure `cargo doc` builds with zero warnings

## Phase 5: Comprehensive Edge Case Tests

- [ ] **Task 5.1**: Add edge case unit tests in `src/header.rs`
- [ ] **Task 5.2**: Add edge case unit tests in `src/annotation.rs`
- [ ] **Task 5.3**: Add edge case unit tests in `src/io_edf.rs`
- [ ] **Task 5.4**: Add edge case integration tests in `tests/`
- [ ] **Task 5.5**: Add negative tests (malformed input → proper error)
