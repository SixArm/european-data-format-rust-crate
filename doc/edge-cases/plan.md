# Plan: Fix Edge Cases in EDF/EDF+ Crate

## Overview

The `doc/edge-cases/index.md` document identifies 13 categories of EDF/EDF+ edge
cases. This plan maps each to specific gaps in the current codebase and proposes
concrete fixes.

## Current State

- All 23 tests pass (13 integration + 10 doc-tests)
- Core read/write/convert works for the Motor Nerve Conduction example
- Byte-perfect round-trips (EDF->JSON->EDF, EDF->XML->EDF) work
- No validation layer exists — the code trusts all input

---

## Edge Case 1: Contiguous Data Limitations (EDF)

**Problem**: Original EDF files must represent contiguous, uninterrupted recordings.
The code doesn't distinguish between EDF and EDF+ modes.

**Current code**: `reserved` field is stored as a string but never inspected for
mode detection during reading.

**Fix**:
- Add `EdfHeader::is_edf_plus()` method (checks if reserved starts with "EDF+C" or "EDF+D")
- Add `EdfHeader::is_discontinuous()` method (checks for "EDF+D")
- In `read_edf`, warn/error if a non-EDF+ file contains annotation signals
- Add validation in `EdfHeader::validate()` that non-EDF+ files don't claim annotations

**Files**: `src/header.rs`, `src/io_edf.rs`

---

## Edge Case 2: Discontinuous Data (EDF+D)

**Problem**: EDF+D files use timekeeping TALs to represent time gaps. The code
partially handles this (group_annotations_by_record uses timekeeping onsets) but
doesn't validate that EDF+D files have proper timekeeping annotations.

**Fix**:
- Validate that every data record in an EDF+D file has at least one timekeeping
  TAL (empty-text annotation) as the first TAL
- In `write_edf`, ensure timekeeping TALs are generated for EDF+D even when
  the user doesn't supply them
- Add test for discontinuous recordings with non-sequential onset times

**Files**: `src/io_edf.rs`, `src/annotation.rs`

---

## Edge Case 3: Sample Rate Restrictions

**Problem**: Samples within a data record must have equal intervals. The interval
between the last sample of one record and the first of the next may differ.

**Current code**: No validation of sample counts or intervals.

**Fix**:
- Add `EdfHeader::validate()` method that checks:
  - `samples_per_record > 0` for all signals
  - Sample count consistency between signal header and actual data
- In `read_edf`, verify actual sample count matches header

**Files**: `src/header.rs`, `src/io_edf.rs`

---

## Edge Case 4: Signal Recording Limits (61,440-byte max)

**Problem**: EDF+ limits data record size to 61,440 bytes.

**Current code**: No size validation.

**Fix**:
- Add validation in `EdfHeader::validate()`:
  `sum(samples_per_record * 2) <= 61440` for EDF+ files
- Return `EdfError::InvalidHeader` if exceeded
- Add test with data record at/above the limit

**Files**: `src/header.rs`, `src/error.rs`

---

## Edge Case 5: Data Type Handling (16-bit integers)

**Problem**: Original EDF uses 16-bit signed integers only. EDF+ can support
floating-point but older software may not handle it.

**Current code**: Correctly uses `i16` for all samples. No floating-point support.

**Fix**:
- This is working correctly for EDF and standard EDF+
- Add a note in documentation that floating-point EDF+ is not yet supported
- No code change needed for now; file an issue for future work

**Files**: None (documentation only in rustdoc)

---

## Edge Case 6: ASCII Header Constraints

**Problem**: All header fields must be ASCII (bytes 32-126). Non-ASCII causes
parsing issues. Current code uses `String::from_utf8_lossy` which silently
replaces invalid bytes.

**Fix**:
- Add `validate_ascii(bytes: &[u8], field: &str)` helper in `header.rs`
- Call it during `EdfHeader::read_from` for all string header fields
- Return `EdfError::InvalidHeader` with a clear message on non-ASCII bytes
- Add `EdfHeader::validate()` check for non-ASCII in string fields
- Add test with non-ASCII bytes in header

**Files**: `src/header.rs`, `src/error.rs`

---

## Edge Case 7: Length-Restricted Fields

**Problem**: Signal labels are limited to 16 characters, header fields to their
spec widths. Truncation can lose data silently.

**Current code**: `format_field` truncates silently.

**Fix**:
- Add `EdfHeader::validate()` checks that field values fit in their spec widths:
  - label: 16 bytes
  - transducer_type: 80 bytes
  - physical_dimension: 8 bytes
  - prefiltering: 80 bytes
  - reserved (signal): 32 bytes
  - patient_identification: 80 bytes
  - recording_identification: 80 bytes
- Return `EdfError::InvalidHeader` or `EdfError::InvalidSignalHeader` if exceeded
- Add a `validate_before_write` step in `write_edf` that warns about truncation

**Files**: `src/header.rs`, `src/io_edf.rs`

---

## Edge Case 8: Calibration Discrepancies

**Problem**: If digital min/max and physical min/max are incorrect, scaling is wrong.

**Current code**: Values are stored and written back but never validated.

**Fix**:
- Add validation in `EdfHeader::validate()`:
  - `digital_minimum < digital_maximum` (or at least `!=`)
  - `physical_minimum < physical_maximum` (or at least `!=`)
  - For annotation signals: digital_min=-32768, digital_max=32767 per spec
- Add helper `EdfSignalHeader::gain()` and `EdfSignalHeader::offset()` for
  digital-to-physical conversion
- Add test with inverted min/max values

**Files**: `src/header.rs`

---

## Edge Case 9: Reserved Field Discrepancies

**Problem**: The 44-char reserved field has different semantics in EDF vs EDF+.
Old EDF software may misinterpret EDF+ reserved field contents.

**Current code**: Stored as a string, no validation.

**Fix**:
- Add validation in `EdfHeader::validate()`:
  - If reserved starts with "EDF+", must be "EDF+C" or "EDF+D" (optionally
    followed by spaces)
  - If file has annotation signals, reserved must start with "EDF+"
- Add `EdfError::InvalidHeader` variant message for invalid reserved field
- Add test for various reserved field values

**Files**: `src/header.rs`

---

## Edge Case 10: Annotation Duration Edge Cases

**Problem**: Annotations can have durations but point-in-time events should have
`None`. Also, the first TAL in each data record must be a timekeeping annotation
(empty text).

**Current code**: Duration is Optional, timekeeping TALs are handled but not
validated on write.

**Fix**:
- Add validation in `write_edf` that the first annotation per record is a
  timekeeping TAL (empty texts)
- Ensure `group_annotations_by_record` always inserts a timekeeping TAL
  as the first annotation per record (it currently does this)
- Add test for annotation with zero duration vs None duration
- Add test for annotation at exact record boundary

**Files**: `src/io_edf.rs`, `src/annotation.rs`

---

## Edge Case 11: Date Formatting

**Problem**: Start date must follow `dd.mm.yy` format. Y2K issues exist in
original EDF; EDF+ addresses this via recording identification.

**Current code**: Date is stored as a string, no validation.

**Fix**:
- Add date validation in `EdfHeader::validate()`:
  - Format check: `dd.mm.yy` with valid day (01-31), month (01-12), year (00-99)
  - Time check: `hh.mm.ss` with valid ranges
- Add `EdfError::InvalidHeader` for date/time format errors
- Add test with invalid dates (e.g., "32.13.99", "ab.cd.ef")

**Files**: `src/header.rs`

---

## Edge Case 12: Manually Created Files / Whitespace

**Problem**: Manually created EDF files may have wrong whitespace or newline
issues in the header.

**Current code**: `parse_ascii` trims trailing spaces which handles most cases.
But doesn't validate the total header size.

**Fix**:
- Validate total header size: `header_bytes == 256 + signals_count * 256`
- Add stricter header byte count validation in `EdfHeader::read_from`
- Return `EdfError::InvalidHeader` if header size is inconsistent
- Add test for header with wrong byte count

**Files**: `src/header.rs`

---

## Edge Case 13: Channel-Oriented vs Sample-Oriented Data

**Problem**: EDF is sample-oriented (data records contain all signals). Accessing
a single channel requires reading all records.

**Current code**: Data is stored per-signal with records grouped, which is
already a channel-oriented view in memory.

**Fix**:
- Add convenience methods to `EdfSignal`:
  - `all_samples() -> Vec<i16>` — flatten all records into a single vector
  - `physical_samples() -> Vec<f64>` — convert digital samples to physical units
- These are quality-of-life improvements for API consumers

**Files**: `src/edf_file.rs`, `src/header.rs`

---

## Implementation Strategy

### Priority Order

1. **High priority** (data correctness):
   - Edge Case 6: ASCII validation (prevents silent data corruption)
   - Edge Case 8: Calibration validation (prevents wrong scaling)
   - Edge Case 11: Date format validation (prevents invalid files)
   - Edge Case 12: Header size validation (prevents malformed files)

2. **Medium priority** (spec compliance):
   - Edge Case 1: EDF/EDF+ mode detection
   - Edge Case 2: EDF+D timekeeping validation
   - Edge Case 4: Data record size limit
   - Edge Case 7: Field length validation
   - Edge Case 9: Reserved field validation
   - Edge Case 10: Annotation validation

3. **Low priority** (quality of life):
   - Edge Case 3: Sample rate validation
   - Edge Case 5: Documentation update
   - Edge Case 13: Channel access helpers

### Architecture

All validation should go through a single `EdfHeader::validate()` method and
a companion `EdfFile::validate()` method. This keeps validation centralized
and testable.

- `EdfHeader::validate() -> Result<(), EdfError>` — validates header fields
- `EdfFile::validate() -> Result<(), EdfError>` — validates file-level constraints
- Call `validate()` at the end of `read_edf` and at the start of `write_edf`
- Validation errors use existing `EdfError` variants (no new variants needed)

### Testing

For each edge case fix:
1. Add unit tests in the relevant module
2. Add integration tests for round-trip behavior
3. Add negative tests (invalid input → proper error)
