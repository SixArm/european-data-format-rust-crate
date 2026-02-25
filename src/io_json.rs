//! JSON serialization and deserialization for EDF/EDF+ files.
//!
//! Converts an [`EdfFile`] to/from JSON using `serde_json`.
//! The JSON representation preserves all header fields, signal data,
//! and annotations.
//!
//! # Examples
//!
//! ```
//! use european_data_format::{EdfFile, EdfHeader, io_json};
//!
//! let edf = EdfFile {
//!     header: EdfHeader {
//!         version: "0".into(),
//!         patient_identification: "X X X X".into(),
//!         recording_identification: "Startdate X X X X".into(),
//!         start_date: "01.01.00".into(),
//!         start_time: "00.00.00".into(),
//!         header_bytes: 256,
//!         reserved: String::new(),
//!         data_records_count: 0,
//!         data_record_duration: 1.0,
//!         signals_count: 0,
//!         signal_headers: vec![],
//!     },
//!     signals: vec![],
//!     annotations: vec![],
//! };
//!
//! let json = io_json::to_json(&edf).unwrap();
//! let parsed: EdfFile = io_json::from_json(&json).unwrap();
//! assert_eq!(parsed.header.version, "0");
//! ```

use crate::edf_file::EdfFile;
use crate::error::EdfError;

/// Serialize an [`EdfFile`] to a pretty-printed JSON string.
///
/// # Errors
///
/// Returns [`EdfError::Json`] if serialization fails.
pub fn to_json(edf: &EdfFile) -> Result<String, EdfError> {
    serde_json::to_string_pretty(edf).map_err(EdfError::Json)
}

/// Deserialize an [`EdfFile`] from a JSON string.
///
/// # Errors
///
/// Returns [`EdfError::Json`] if the JSON is malformed or does not
/// match the expected structure.
pub fn from_json(json: &str) -> Result<EdfFile, EdfError> {
    serde_json::from_str(json).map_err(EdfError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::EdfAnnotation;
    use crate::edf_file::{EdfDataRecord, EdfSignal};
    use crate::header::{EdfHeader, EdfSignalHeader};

    fn sample_edf() -> EdfFile {
        EdfFile {
            header: EdfHeader {
                version: "0".into(),
                patient_identification: "MCH-0234567 F 02-MAY-1951 Haagse_Harry".into(),
                recording_identification: "Startdate 02-MAR-2002 EMG561 BK/JOP Sony.".into(),
                start_date: "17.04.01".into(),
                start_time: "11.25.00".into(),
                header_bytes: 768,
                reserved: "EDF+D".into(),
                data_records_count: 2,
                data_record_duration: 0.05,
                signals_count: 2,
                signal_headers: vec![
                    EdfSignalHeader {
                        label: "R APB".into(),
                        transducer_type: "AgAgCl electrodes".into(),
                        physical_dimension: "mV".into(),
                        physical_minimum: -100.0,
                        physical_maximum: 100.0,
                        digital_minimum: -2048,
                        digital_maximum: 2047,
                        prefiltering: "HP:3Hz LP:20kHz".into(),
                        samples_per_record: 1000,
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
                        samples_per_record: 60,
                        reserved: String::new(),
                    },
                ],
            },
            signals: vec![EdfSignal {
                header: EdfSignalHeader {
                    label: "R APB".into(),
                    transducer_type: "AgAgCl electrodes".into(),
                    physical_dimension: "mV".into(),
                    physical_minimum: -100.0,
                    physical_maximum: 100.0,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefiltering: "HP:3Hz LP:20kHz".into(),
                    samples_per_record: 1000,
                    reserved: String::new(),
                },
                records: vec![
                    EdfDataRecord { sample: vec![0; 1000] },
                    EdfDataRecord { sample: vec![0; 1000] },
                ],
            }],
            annotations: vec![
                EdfAnnotation {
                    onset: 0.0,
                    duration: None,
                    texts: vec![],
                },
                EdfAnnotation {
                    onset: 0.0,
                    duration: None,
                    texts: vec!["Stimulus right wrist".into()],
                },
                EdfAnnotation {
                    onset: 10.0,
                    duration: None,
                    texts: vec![],
                },
                EdfAnnotation {
                    onset: 10.0,
                    duration: None,
                    texts: vec!["Stimulus right elbow".into()],
                },
            ],
        }
    }

    #[test]
    fn test_json_round_trip() {
        let edf = sample_edf();
        let json = to_json(&edf).unwrap();
        let parsed = from_json(&json).unwrap();
        assert_eq!(edf, parsed);
    }

    #[test]
    fn test_json_contains_expected_fields() {
        let edf = sample_edf();
        let json = to_json(&edf).unwrap();
        assert!(json.contains("MCH-0234567"));
        assert!(json.contains("R APB"));
        assert!(json.contains("EDF Annotations"));
        assert!(json.contains("Stimulus right wrist"));
    }

    #[test]
    fn test_json_invalid_input() {
        let result = from_json("not valid json");
        assert!(result.is_err());
    }
}
