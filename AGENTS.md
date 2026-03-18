# European Data Format (EDF/EDF+) Rust crate

Think carefully. Plan. Use tasks.md file.

Research the EDF/EDF+ specification:

- [https://www.edfplus.info/specs/edfplus.html](https://www.edfplus.info/specs/edfplus.html)

Research previous implementions in C/C++ Python Rust:

- [EDFlib](https://gitlab.com/Teuniz/EDFlib): C/C++ library to read/write EDF+ and BDF+ files.
- [EDFlib-Java](https://gitlab.com/Teuniz/EDFlib-Java): Java library to read/write EDF+ and BDF+ files.
- [PyEDFlib](https://pyedflib.readthedocs.io/en/latest/) [code](https://github.com/holgern/pyedflib): Python wavelet toolbox for reading / writing EDF/EDF+/BDF files.
- [edfplus](https://docs.rs/edfplus/latest/edfplus/) [code](https://github.com/2986002971/edfplus): Rust library for reading and writing EDF+ (European Data Format Plus) files.

Serialization/deserialization:

- Use "serde" rust crate.
- Create lib code to serialize/deserialze
- From EDF file or EDF+ file
- Into XML file or JSON file

Command line argument parsing:

- Use "clap" rust crate.
- Create comprehensive command line help and argument options explanations.

Error handling:

- Use "thiserror" rust crate.
- Create custom errors for various kinds of parsing erros.

Documentation:

- Create comprehensive rustdoc comments.
- Show syntax, usage, examples, notes.

Testing:

- Create comprehensive unit tests.
- Create comprehenstive integration tests with example files
- Create example files with the same information: example.edf, example.xml, example.json
- Read EDF+ example.edf then verify XML output equals example.xml
- Read EDF+ example.edf then verify JSON output equals example.json
- Read XML example.xml then verify EDF+ output equals example.edf
- Read JSON example.json then verify EDF+ output equals example.edf

IMPORTANT VERIFY:

- EDF -> JSON -> EDF: byte-perfect round-trip
- EDF -> XML -> EDF: byte-perfect round-trip
- JSON -> XML: structurally identical
- CLI: all 6 conversion directions work (edf <-> json, edf <-> xml, json <-> xml)
- Documentation: cargo doc builds with zero warnings
