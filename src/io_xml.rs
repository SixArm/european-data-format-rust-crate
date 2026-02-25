//! XML serialization and deserialization for EDF/EDF+ files.
//!
//! Converts an [`EdfFile`] to/from XML using `quick-xml` with serde support.
//! The root element is `<EdfFile>`.
//!
//! # Examples
//!
//! ```
//! use european_data_format::{EdfFile, EdfHeader, io_xml};
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
//! let xml = io_xml::to_xml(&edf).unwrap();
//! assert!(xml.contains("<EdfFile>"));
//! ```

use crate::edf_file::EdfFile;
use crate::error::EdfError;

/// Serialize an [`EdfFile`] to an XML string.
///
/// The root element is `<EdfFile>`. The XML is not pretty-printed
/// by default (quick-xml serialization produces compact XML).
///
/// # Errors
///
/// Returns [`EdfError::XmlSe`] if serialization fails.
pub fn to_xml(edf: &EdfFile) -> Result<String, EdfError> {
    let xml = quick_xml::se::to_string(edf).map_err(EdfError::XmlSe)?;
    // Add XML declaration and format nicely
    Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{xml}"))
}

/// Deserialize an [`EdfFile`] from an XML string.
///
/// Expects the root element to be `<EdfFile>`.
///
/// # Errors
///
/// Returns [`EdfError::XmlDe`] if the XML is malformed or does not
/// match the expected structure.
pub fn from_xml(xml: &str) -> Result<EdfFile, EdfError> {
    quick_xml::de::from_str(xml).map_err(EdfError::XmlDe)
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
                patient_identification: "test patient".into(),
                recording_identification: "Startdate X X X X".into(),
                start_date: "01.01.00".into(),
                start_time: "12.00.00".into(),
                header_bytes: 512,
                reserved: "EDF+C".into(),
                data_records_count: 1,
                data_record_duration: 1.0,
                signals_count: 1,
                signal_headers: vec![EdfSignalHeader {
                    label: "EEG".into(),
                    transducer_type: "electrode".into(),
                    physical_dimension: "uV".into(),
                    physical_minimum: -500.0,
                    physical_maximum: 500.0,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefiltering: String::new(),
                    samples_per_record: 3,
                    reserved: String::new(),
                }],
            },
            signals: vec![EdfSignal {
                header: EdfSignalHeader {
                    label: "EEG".into(),
                    transducer_type: "electrode".into(),
                    physical_dimension: "uV".into(),
                    physical_minimum: -500.0,
                    physical_maximum: 500.0,
                    digital_minimum: -2048,
                    digital_maximum: 2047,
                    prefiltering: String::new(),
                    samples_per_record: 3,
                    reserved: String::new(),
                },
                records: vec![EdfDataRecord { sample: vec![100, -200, 300] }],
            }],
            annotations: vec![EdfAnnotation {
                onset: 0.0,
                duration: None,
                texts: vec!["test event".into()],
            }],
        }
    }

    #[test]
    fn test_xml_round_trip() {
        let edf = sample_edf();
        let xml = to_xml(&edf).unwrap();
        let parsed = from_xml(&xml).unwrap();
        assert_eq!(edf, parsed);
    }

    #[test]
    fn test_xml_contains_expected_elements() {
        let edf = sample_edf();
        let xml = to_xml(&edf).unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<EdfFile>"));
        assert!(xml.contains("test patient"));
        assert!(xml.contains("test event"));
    }

    #[test]
    fn test_xml_invalid_input() {
        let result = from_xml("not valid xml");
        assert!(result.is_err());
    }
}
