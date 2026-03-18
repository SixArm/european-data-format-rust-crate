# Edge cases

The European Data Format (EDF and EDF+) is a widely used, non-proprietary
standard for time-series data, particularly in medical fields like EEG and
polysomnography. While robust, EDF/EDF+ has specific edge cases, limitations,
and formatting nuances that practitioners must handle.

## Technical and Signal Edge Cases

Contiguous Data Limitations (EDF): Original EDF files must represent contiguous,
uninterrupted recording sessions. Discontinuities cannot be natively handled
without breaking the format, which prompted the creation of EDF+.

Discontinuous Data (EDF+): EDF+ handles interruptions using "Annotation TALs"
(Time-stamped Annotation Lists). While this allows for breaks, it requires
viewers to properly interpret these annotations to represent the time gap
correctly.

Sample Rate Restrictions: The samples of a signal must have equal sample
intervals within a single data record. The interval between the last sample of
one record and the first sample of the next may be different, requiring robust
parsing.

Signal Recording Limits: EDF+ has a maximum data record size of 61,440 bytes.
Storing a high number of signals (e.g., 124+ channels) with high sampling
frequencies (e.g., >1000 Hz) in this space can be difficult.

Data Type Handling: Original EDF files only support 16-bit signed integer data
(little-endian). EDF+ supports floating-point data, but older software might not
handle this, causing interpretation issues.

## File Structure and Header Edge Cases

ASCII Header Constraints: All header information (patient ID, signal labels,
calibration) must be in ASCII. Non-ASCII characters (like accents) can cause
parsing issues.

Length-Restricted Fields: Headers are 256 bytes plus 256 bytes per signal.
Signal labels are strictly limited to 16 characters. If labels exceed this, they
are truncated, requiring padding or truncation management.

Calibration Discrepancies: If the digital minimum/maximum and physical
minimum/maximum fields are not set correctly relative to the physical values,
data visualization will be improperly scaled.

Reserved Field Discrepancies: The "Reserved" field of 44 characters is utilized
differently in EDF+ compared to original EDF. Using old EDF software to open
EDF+ files with specialized annotations in this field can cause issues.

## Annotation and Timing Edge Cases

Negative-Up Convention: In medical EEG, the convention is that "negative-up." If
an EDF file is generated with "positive-up" and not noted, it will be
misinterpreted.

Annotation Duration: EDF+ allows annotations (events, stimuli) that can have
durations. A common mistake is assigning a duration to an annotation that only
represents a point in time (like an onset marker). Date Formatting: While EDF+
fixed Y2K issues present in older formats, ensuring the date and time format in
the header follows the strict dd.mm.yy standard is crucial for chronological
sorting.

## Environmental and Compatibility Edge Cases

Manually Created Files: When EDF files are created via text editors to meet
strict 1020 system coordinates, they might fail due to strict whitespace or
newline requirements in the header, making automatic tools more reliable.

Channel-Oriented vs. Sample-Oriented Data: EDF stores data as contiguous samples
(sample-oriented), which makes accessing a single channel's data slow.
Converting this to channel-oriented data (e.g., via NeuroPigPen) requires
handling potential data loss during conversion.
