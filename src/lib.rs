//! European Data Format (EDF/EDF+) library for Rust.
//!
//! This crate provides reading, writing, and conversion of EDF and EDF+
//! files (European Data Format), a standard format for medical time-series
//! data such as EEG, EMG, ECG, and polysomnography (PSG) recordings.
//!
//! # Supported Formats
//!
//! - **EDF binary** (`.edf`) — the native binary format
//! - **JSON** (`.json`) — via `serde_json`
//! - **XML** (`.xml`) — via `quick-xml` with serde support
//!
//! # Quick Start
//!
//! ```no_run
//! use european_data_format::{io_edf, io_json, io_xml, EdfFile};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! // Read an EDF file
//! let file = File::open("recording.edf").unwrap();
//! let edf = io_edf::read_edf(&mut BufReader::new(file)).unwrap();
//!
//! // Convert to JSON
//! let json = io_json::to_json(&edf).unwrap();
//! println!("{json}");
//!
//! // Convert to XML
//! let xml = io_xml::to_xml(&edf).unwrap();
//! println!("{xml}");
//! ```
//!
//! # EDF+ Features
//!
//! EDF+ extends EDF with:
//! - **Annotations**: time-stamped text annotations stored in special
//!   "EDF Annotations" signals using TAL (Time-stamped Annotation List) encoding.
//! - **Discontinuous recordings**: data records that are not contiguous in time,
//!   indicated by `"EDF+D"` in the header's reserved field.
//! - **Standardized patient/recording identification**: structured subfields
//!   in the header for patient code, sex, birthdate, name, etc.
//!
//! # Architecture
//!
//! - [`EdfFile`] — top-level structure containing header, signals, and annotations
//! - [`EdfHeader`] / [`EdfSignalHeader`] — file and per-signal header metadata
//! - [`EdfSignal`] — ordinary signal data (samples per data record)
//! - [`EdfAnnotation`] — parsed EDF+ annotation with onset, duration, and text
//! - [`EdfError`] — unified error type for all operations

pub mod annotation;
pub mod edf_file;
pub mod error;
pub mod header;
pub mod io_edf;
pub mod io_json;
pub mod io_xml;

pub use annotation::EdfAnnotation;
pub use edf_file::{EdfDataRecord, EdfFile, EdfSignal};
pub use error::EdfError;
pub use header::{EdfHeader, EdfSignalHeader};
