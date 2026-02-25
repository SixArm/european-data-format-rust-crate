//! Top-level EDF/EDF+ file structure.
//!
//! An [`EdfFile`] combines the parsed header, ordinary signal data,
//! and EDF+ annotations into a single structure that can be serialized
//! to JSON, XML, or written back to EDF binary format.
//!
//! # Examples
//!
//! ```
//! use european_data_format::{EdfFile, EdfSignal, EdfDataRecord, EdfHeader, EdfSignalHeader, EdfAnnotation};
//!
//! let file = EdfFile {
//!     header: EdfHeader {
//!         version: "0".into(),
//!         patient_identification: "X X X X".into(),
//!         recording_identification: "Startdate X X X X".into(),
//!         start_date: "01.01.00".into(),
//!         start_time: "00.00.00".into(),
//!         header_bytes: 512,
//!         reserved: "EDF+C".into(),
//!         data_records_count: 0,
//!         data_record_duration: 1.0,
//!         signals_count: 1,
//!         signal_headers: vec![],
//!     },
//!     signals: vec![],
//!     annotations: vec![],
//! };
//! assert_eq!(file.header.version, "0");
//! ```

use serde::{Deserialize, Serialize};

use crate::annotation::EdfAnnotation;
use crate::header::{EdfHeader, EdfSignalHeader};

/// A complete EDF/EDF+ file in memory.
///
/// Contains the file header, ordinary signal data (samples), and
/// any EDF+ annotations parsed from "EDF Annotations" signals.
///
/// Note: The `header.signal_headers` includes headers for *all* signals
/// in the original file (including annotation signals). The `signals`
/// field contains only the *ordinary* (non-annotation) signals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfFile {
    /// The file header, including per-signal headers for all signals.
    pub header: EdfHeader,

    /// Ordinary (non-annotation) signal data.
    pub signals: Vec<EdfSignal>,

    /// EDF+ annotations extracted from "EDF Annotations" signals.
    pub annotations: Vec<EdfAnnotation>,
}

/// A single data record's worth of samples for one signal.
///
/// This wrapper ensures that nested sample vectors serialize correctly
/// in XML format (each record becomes a distinct `<record>` element).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfDataRecord {
    /// The sample values (raw digital i16 values) for this data record.
    pub sample: Vec<i16>,
}

/// An ordinary signal's data, grouped by data record.
///
/// Each signal has a header describing its properties and a list of
/// data records, each containing the samples for that record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdfSignal {
    /// The header for this signal (copied from the file header).
    pub header: EdfSignalHeader,

    /// Sample values per data record.
    ///
    /// `records[r].sample` contains `samples_per_record` i16 values for
    /// data record `r`. These are the raw digital values; to convert
    /// to physical units, apply the linear scaling from the header:
    ///
    /// ```text
    /// physical = (digital - digital_min) / (digital_max - digital_min)
    ///            * (physical_max - physical_min) + physical_min
    /// ```
    pub records: Vec<EdfDataRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edf_file_default_construction() {
        let file = EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "test".into(),
                recording_identification: "test".into(),
                start_date: "01.01.00".into(),
                start_time: "00.00.00".into(),
                header_bytes: 256,
                reserved: String::new(),
                data_records_count: 0,
                data_record_duration: 1.0,
                signals_count: 0,
                signal_headers: vec![],
            },
            signals: vec![],
            annotations: vec![],
        };
        assert_eq!(file.signals.len(), 0);
        assert_eq!(file.annotations.len(), 0);
    }

    #[test]
    fn test_edf_signal_with_samples() {
        let signal = EdfSignal {
            header: EdfSignalHeader {
                label: "EEG".into(),
                transducer_type: String::new(),
                physical_dimension: "uV".into(),
                physical_minimum: -500.0,
                physical_maximum: 500.0,
                digital_minimum: -2048,
                digital_maximum: 2047,
                prefiltering: String::new(),
                samples_per_record: 3,
                reserved: String::new(),
            },
            records: vec![
                EdfDataRecord { sample: vec![100, -200, 300] },
                EdfDataRecord { sample: vec![400, -500, 600] },
            ],
        };
        assert_eq!(signal.records.len(), 2);
        assert_eq!(signal.records[0].sample.len(), 3);
    }
}
