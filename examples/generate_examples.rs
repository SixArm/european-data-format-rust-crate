//! Generates example EDF, JSON, and XML files based on the Motor Nerve
//! Conduction example from the EDF+ specification (section 3.7).
//!
//! Run with: `cargo run --example generate_examples`

use european_data_format::annotation::EdfAnnotation;
use european_data_format::edf_file::{EdfDataRecord, EdfFile, EdfSignal};
use european_data_format::header::{EdfHeader, EdfSignalHeader};
use european_data_format::{io_edf, io_json, io_xml};
use std::fs::File;
use std::io::BufWriter;

fn main() {
    let edf = build_motor_nerve_conduction_example();

    // Write EDF binary
    let edf_path = "examples/example.edf";
    let file = File::create(edf_path).expect("create example.edf");
    let mut writer = BufWriter::new(file);
    io_edf::write_edf(&edf, &mut writer).expect("write EDF");
    drop(writer);
    eprintln!("Wrote {edf_path}");

    // Write JSON
    let json_path = "examples/example.json";
    let json = io_json::to_json(&edf).expect("serialize JSON");
    std::fs::write(json_path, format!("{json}\n")).expect("write JSON");
    eprintln!("Wrote {json_path}");

    // Write XML
    let xml_path = "examples/example.xml";
    let xml = io_xml::to_xml(&edf).expect("serialize XML");
    std::fs::write(xml_path, format!("{xml}\n")).expect("write XML");
    eprintln!("Wrote {xml_path}");
}

/// Build the Motor Nerve Conduction example from the EDF+ spec section 3.7.
///
/// Patient: MCH-0234567 F 02-MAY-1951 Haagse_Harry
/// Recording: Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve.
/// 2 signals: "R APB" (1000 samples/record) + "EDF Annotations" (60 samples/record)
/// 2 data records (wrist + elbow stimulation)
/// Duration: 0.050s per record, EDF+D (discontinuous)
fn build_motor_nerve_conduction_example() -> EdfFile {
    let signal_header = EdfSignalHeader {
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
    };

    let annotation_header = EdfSignalHeader {
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
    };

    let header = EdfHeader {
        version: "0".into(),
        patient_identification: "MCH-0234567 F 02-MAY-1951 Haagse_Harry".into(),
        recording_identification:
            "Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve.".into(),
        start_date: "17.04.01".into(),
        start_time: "11.25.00".into(),
        header_bytes: 768, // 256 + 2*256
        reserved: "EDF+D".into(),
        data_records_count: 2,
        data_record_duration: 0.05,
        signals_count: 2,
        signal_headers: vec![signal_header.clone(), annotation_header],
    };

    // Generate simulated EMG waveform data for the R APB signal.
    // Record 1: wrist stimulation response — simulated compound muscle action potential (CMAP)
    // Record 2: elbow stimulation response — similar CMAP with longer latency
    let wrist_samples = generate_cmap_waveform(1000, 76, 500); // peak at ~3.8ms (sample 76 at 20kHz)
    let elbow_samples = generate_cmap_waveform(1000, 156, 500); // peak at ~7.8ms (sample 156)

    let signals = vec![EdfSignal {
        header: signal_header,
        records: vec![
            EdfDataRecord {
                sample: wrist_samples,
            },
            EdfDataRecord {
                sample: elbow_samples,
            },
        ],
    }];

    // Annotations from the spec
    let annotations = vec![
        // Record 1: time-keeping + wrist stimulation
        EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![],
        },
        EdfAnnotation {
            onset: 0.0,
            duration: None,
            texts: vec![
                "Stimulus right wrist 0.2ms x 8.2mA at 6.5cm from recording site".into(),
                "Response 7.2mV at 3.8ms".into(),
            ],
        },
        // Record 2: time-keeping + elbow stimulation
        EdfAnnotation {
            onset: 10.0,
            duration: None,
            texts: vec![],
        },
        EdfAnnotation {
            onset: 10.0,
            duration: None,
            texts: vec![
                "Stimulus right elbow 0.2ms x 15.3mA at 28.5cm from recording site".into(),
                "Response 7.2mV at 7.8ms (55.0m/s)".into(),
            ],
        },
    ];

    EdfFile {
        header,
        signals,
        annotations,
    }
}

/// Generate a simulated Compound Muscle Action Potential (CMAP) waveform.
///
/// Creates a biphasic waveform with a negative peak at `peak_sample`
/// and amplitude scaled to `amplitude` digital units.
fn generate_cmap_waveform(num_samples: usize, peak_sample: usize, amplitude: i16) -> Vec<i16> {
    let mut samples = vec![0i16; num_samples];
    let width = 40.0_f64; // width of the response in samples

    for i in 0..num_samples {
        let x = (i as f64 - peak_sample as f64) / width;
        // Biphasic waveform: negative peak followed by positive overshoot
        let value = -amplitude as f64 * x * (-x * x).exp() * 2.0;
        samples[i] = value.round().clamp(-2048.0, 2047.0) as i16;
    }

    samples
}
