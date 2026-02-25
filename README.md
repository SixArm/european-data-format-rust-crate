# European Data Format (EDF/EDF+) Rust crate

[https://www.edfplus.info/specs/edfplus.html](https://www.edfplus.info/specs/edfplus.html)

Related works:

- [EDFlib](https://gitlab.com/Teuniz/EDFlib): C/C++ library to read/write EDF+ and BDF+ files.
- [EDFlib-Java](https://gitlab.com/Teuniz/EDFlib-Java): Java library to read/write EDF+ and BDF+ files.
- [PyEDFlib](https://pyedflib.readthedocs.io/en/latest/) [code](https://github.com/holgern/pyedflib): Python wavelet toolbox for reading / writing EDF/EDF+/BDF files.
- [edfplus](https://docs.rs/edfplus/latest/edfplus/) [code](https://github.com/2986002971/edfplus): Rust library for reading and writing EDF+ (European Data Format Plus) files.

## 1. Introduction

After its introduction in 1992, the European Data Format (EDF) became the
standard for EEG and PSG (Sleep) recordings. During that time, several users
pointed us at its limitations also for application in other fields such as
myography, evoked potentials and cardiology. A major limitation was that EDF can
only handle uninterrupted recordings. So we simply skipped that limitation but
kept all other specifications of EDF intact. While maintaining EDF
compatibility, we also standardized most of the labels and added a possibility
to save annotations and analysis results. The result is EDF+ and can save most
EEG, PSG, ECG, EMG, and Evoked Potential data that can not be saved into common
hospital information systems.

Using EDF+, all signals, annotations and events that are recorded in one session
using one recording system can be kept safely together in one file. EDF+ can
also store events and annotations only, without any signals. This flexibility
allows the user to choose an optimal mix. For instance, our sleep centre stores
all on-line recorded data (signals, annotations) in one file, its hypnogram and
apnea detections in another EDF+ file, the same sleep scorings but made by
another technician in a third file. In Neurophysiology, the on-line obtained raw
EEG traces with stimulus events from an EP investigation might be stored in one
file and the averaged curves with detected latencies in a second file. In
Cardiology, the raw ECG with annotations about the patients exercises can be in
one file, the detected QRS parameters in another file.

EDF+ allows storage of several NON-CONTIGUOUS recordings into one file. This is
the only incompatibility with EDF. All other features are EDF compatible. In
fact, old EDF viewers still work and display EDF+ recordings as if they were
continuous. Therefore, we recommend EDF+ files of EEG or PSG studies to be
continuous if there are no good reasons for the opposite.

Because EDF+ is very close to EDF, and equally simple, EDF+ software can
relatively easily be developed based on available EDF software.

## 2. The EDF+ protocol

Because EDF+ is based on EDF, you should first read the EDF specs. Section 2.1
below describes how EDF+ differs from EDF. Section 2.2 describes how one of the
EDF+ signals can be specially coded to store text annotations, time, events and
stimuli.

EDF+ prescribes the following filename convention. Signals, recorded using the
same technique and constant amplifier settings, can be stored in one file.
Different techniques, or identical techniques but with different amplifier
settings, must be stored in separate files. All EDF+ files must have .edf or
.EDF as filename extension. See also section 2.3.

### 2.1. EDF+ compared to EDF

A standard EDF file consists of a header record followed by data records. The
header record identifies the patient and specifies the technical characteristics
of the recorded signals. The data records contain consecutive fixed-duration
epochs of the recorded signals. A standard EDF+ file also consists of a header
record followed by data records. The structure of these records is compatible to
EDF but contains additional specifications. Note that for your EDF+ software to
also be EDF compatible, it should support but not rely on these additional
specifications.

### 2.1.1. The EDF+ header

The EDF+ header record identifies the patient and specifies the technical
characteristics of the recorded signals according to the EDF specs, except for
the first 'reserved' field (44 characters) which must start with 'EDF+C' if the
recording is uninterrupted, thus having contiguous data records, i.e. the
starttime of each data record coincides with the end (starttime + duration) of
the preceding one. In this case, the file is EDF compatible and the recording
ends (number x duration) seconds after its startdate/time. The 'reserved' field
must start with 'EDF+D' if the recording is interrupted, so not all data records
are contiguous. In both cases, the time must be kept in each data record as
specified in section 2.2.4.

The only incompatibility with EDF is, that signals may be recorded
discontinuously. Therefore, we have decided that the EDF+ 'version' field must
still read '0 ' like in EDF. In this way, old EDF viewers will still work and
display EDF+ files (be they continuous or discontinuous) as continuous EDF
files. EDF+ software will know the difference between continuous and
discontinuous files from the mentioned 'reserved' field.

#### 2.1.2. The EDF+ data records

A signal in an EDF data record is a series of 2-byte samples, the subsequent
samples representing subsequent integer values of that signal, sampled with
equal time intervals. We will refer to this kind of signal as an 'ordinary
signal' from now on. EDF+ data records can (and usually do) also contain
ordinary signals. The EDF+ data records contain the ordinary signals according
to the EDF specs (including the size limit of 61440), except for the fact that
the data records may unconditionally be shorter than 1s, and subsequent data
records need not form a continuous recording. However, as in EDF, data records
that follow up in time must also follow up in the file. The samples of an
ordinary signal must have equal sample intervals inside each data record, but
the interval to the first sample of the next data record may be different.

For instance, in a motor nerve conduction study with a number of stimuli, each
data record would hold the ordinary signals corresponding to one stimulus. In
this case, the duration of a data record corresponds to the "window size" in an
ENMGEP study.

Specifying a duration makes no sense if the EDF+ file does not contain any
ordinary signals, for instance in a file that only contains manual sleep scores
(like in both demo scoring files that come with Polyman, and the example in
3.3). This is also true in the extreme case in which each ordinary signal only
occupies one sample in each data record, while the file is discontinuous
(EDF+D). In those two cases, specify the 'duration of a data record' to be 0.

#### 2.1.3. Additional specifications in EDF+

1. In the header, use only printable US-ASCII characters with byte values
   32..126.

2. The 'startdate' and 'starttime' fields in the header should contain only
   characters 0-9, and the period (.) as a separator, for example "02.08.51". In
   the 'startdate', use 1985 as a clipping date in order to avoid the Y2K problem.
   So, the years 1985-1999 must be represented by yy=85-99 and the years 2000-2084
   by yy=00-84. After 2084, yy must be 'yy' and only item 4 of this paragraph
   defines the date.

3. The 'local patient identification' field must start with the subfields
   (subfields do not contain, but are separated by, spaces):
   - the code by which the patient is known in the hospital administration.
   - sex (English, so F or M).
   - birthdate in dd-MMM-yyyy format using the English 3-character abbreviations
     of the month in capitals. 02-AUG-1951 is OK, while 2-AUG-1951 is not.
   - the patients name.
   - Any space inside the hospital code or the name of the patient must be
     replaced by a different character, for instance an underscore. For instance,
     the 'local patient identification' field could start with: MCH-0234567 F
     02-MAY-1951 Haagse_Harry. Subfields whose contents are unknown, not
     applicable or must be made anonymous are replaced by a single character 'X'.
     So, if everything is unknown then the 'local patient identification' field
     would start with: 'X X X X'. Additional subfields may follow the ones
     described here.

4. The 'local recording identification' field must start with the subfields
   (subfields do not contain, but are separated by, spaces):
   - The text 'Startdate'.
   - The startdate itself in dd-MMM-yyyy format using the English 3-character abbreviations of the month in capitals.
   - The hospital administration code of the investigation, i.e. EEG number or PSG number.
   - A code specifying the responsible investigator or technician.
   - A code specifying the used equipment.
   - Any space inside any of these codes must be replaced by a different character,
     for instance an underscore. The 'local recording identification' field could
     contain: Startdate 02-MAR-2002 PSG-1234/2002 NN Telemetry03. Subfields whose
     contents are unknown, not applicable or must be made anonymous are replaced by
     a single character 'X'. So, if everything is unknown then the 'local recording
     identification' field would start with: 'Startdate X X X X'. Additional
     subfields may follow the ones described here.

5. 'Digital maximum' must be larger than 'Digital minimum'. In case of a
   negative amplifier gain the corresponding 'Physical maximum' is smaller than the
   'Physical minimum'. Check item 9 on how to apply the 'negativity upward' rule in
   Clinical Neurophysiology to the physical ordinary signal. 'Physical maximum'
   must differ from 'Physical minimum'. In case of uncalibrated signals, physical
   dimension is left empty (that is 8 spaces), while 'Physical maximum' and
   'Physical minimum' must still contain different values (this is to avoid
   'division by 0' errors by some viewers).

6. Never use any digit grouping symbol in numbers. Never use a comma "," for a
   for a decimal separator. When a decimal separator is required, use a dot (".").

7. The ordinary signal samples (2-byte two's complement integers) must be stored
   in 'little-endian' format, that is the least significant byte first. This is the
   default format in PC applications.

8. The 'starttime' should be local time at the patients location when the
   recording was started.

9. Use the standard texts and polarity rules at
   http://www.edfplus.info/specs/edftexts.html. These standard texts may in the
   future be extended with further texts, a.o. for Sleep scorings, ENG and various
   evoked potentials.

10. The 'number of data records' can only be -1 during recording. As soon as the
    file is closed, the correct number is known and must be entered.

11. If filters (such as HighPass, LowPass or Notch) were applied to the ordinary
    signals then, preferably automatically, specify them like "HP:0.1Hz LP:75Hz
    N:50Hz" in the "prefiltering" field of the header. If the file contains an
    analysis result, the prefiltering field should mention the relevant analysis
    parameters.

12. The "transducertype" field should specify the applied sensor, such as
    "AgAgCl electrode" or "thermistor".

### 2.2. Annotations for text, time-keeping, events and stimuli

This section describes how one of the EDF+ signals can be specially coded to
store text annotations, time, events and stimuli. In this way, annotations and
events are kept in the same file as the signals that they refer to. The coding
is EDF compatible in the sense that old EDF software would simply treat this
'EDF Annotations' signal as if it were a (strange-looking) ordinary signal.

#### 2.2.1. The 'EDF Annotations' signal

EDF+ data records can (and often do) contain ordinary signals. EDF+ introduces
one other kind of signal, in which the values are annotations that can occur at
any arbitrary point of time. This signal is identified by giving it (in the EDF+
header) the label 'EDF Annotations'. As in EDF, the 'nr of samples in each data
record' field in the header specifies how many 2-byte integers this 'EDF
Annotations' signal occupies in each datarecord. But instead of storing
'ordinary signal' samples, those 2-byte integers are filled with characters.

The character-bytes are stored byte-by-byte without changing their order. For
instance, the text 'abc' is represented by successive byte values 97, 98 and 99
in the 'EDF Annotations' signal. Even if no annotations are to be kept, an EDF+
file must contain at least one 'EDF Annotations' signal in order to specify the
starttime of each datarecord (see section 2.2.4). Of course, the label 'EDF
Annotations' is not allowed for ordinary signals.

The 'EDF Annotations' signal only has meaningful header fields 'label' and 'nr
of samples in each data record'. For the sake of EDF compatibility, the fields
'digital minimum' and 'digital maximum' must be filled with -32768 and 32767,
respectively. The 'Physical maximum' and 'Physical minimum' fields must contain
values that differ from each other. The other fields of this signal are filled
with spaces.

#### 2.2.2. Time-stamped Annotations Lists (TALs) in an 'EDF Annotations' signal

Text, time-keeping, events and stimuli are coded as text annotations in this
'EDF Annotations' signal. The annotations are listed in Time-stamped Annotations
Lists (TALs) as follows.

Each TAL starts with a time stamp Onset21Duration20 in which 21 and 20 are
single bytes with value 21 and 20, respectively (unprintable ASCII characters)
and Onset as well as Duration are coded using US-ASCII characters with byte
values 43, 45, 46 and 48-57 (the '+', '-', '.' and '0'-'9' characters,
respectively). Onset must start with a '+' or a '-' character and specifies the
amount of seconds by which the onset of the annotated event follows ('+') or
precedes ('-') the startdate/time of the file, that is specified in the header.

Duration must not contain any '+' or '-' and specifies the duration of the
annotated event in seconds. If such a specification is not relevant, Duration
can be skipped in which case its preceding 21 must also be skipped. Both Onset
and Duration can contain a dot ('.') but only if the fraction of a second is
specified (up to arbitrary accuracy).

After the time stamp, a list of annotations all sharing the same Onset and
Duration may follow. Each annotation is followed by a single 20 and may not
contain any 20. A 0-byte (the unprintable ASCII character with byte value 0)
follows after the last 20 of this TAL. So the TAL ends with a 20 followed by a 0.

In each data record, the first TAL must start at the first byte of the 'EDF
Annotations signal'. Subsequent TALs in the same data record must follow
immediately after the trailing 0 of the preceding TAL. A TAL, including its
trailing 0, may not overflow into another data record. Each event is annotated
only once, even if its duration makes it extend into the time period of other
data records. Unused bytes of the 'EDF Annotations' signal in the remainder of
the data record are also filled with 0-bytes. Additional 'EDF Annotations'
signals may be defined according to the same specification.

For example, if the technician switches off the lights and closes the door 3
minutes after startdate/time, this can be stored as the 28-bytes TAL
'+18020Lights off20Close door200' without the quotes. Alternatively, the two
events can be stored as two separate shorter TALs '+18020Lights
off200+18020Close door200', also without the quotes. The TAL
'+1800.22125.520Apnea200' codes a 25.5s apnea that begins 30 minutes and 0.2s
after starttime.

#### 2.2.3. Annotations in a TAL

The part between 20 and the next20 is called one annotation. These annotations
may only contain UCS characters (ISO 10646, the 'Universal Character Set', which
is identical to the Unicode version 3+ character set) encoded by UTF-8. This
encoding is supported by the major operating systems, compilers and
applications.

The first 127 UCS characters are identical to those in US-ASCII and are encoded
in the corresponding single byte values. US-ASCII characters that are
represented by byte values 0-31 are allowed in the annotations only if
explicitly prescribed by this EDF+ protocol.

In order to enable multi-line texts and tables, US-ASCII characters that are
represented by byte values 9 (TAB), 10 (LF) and 13 (CR) are allowed in the
annotations. The first 65534 characters (the Basic Multilingual Plane: BMP) of
the UCS contain virtually all characters used in any language in the world
including Asian languages and UTF-8 encodes these in up to three byte-values.
Remember that this encoding applies to the 'EDF Annotations' signal only: in the
EDF+ header, only US-ASCII characters with byte values 32..126 are allowed.

In order to support automatic averaging and superimposition, identical events or
stimuli that occur several times in one file must be coded each time by the
same, unique annotation. Annotations (the part between 20 and the next 20) of
different events/stimuli (or the same stimulus at a different location) must
differ from this unique annotation.

Annotations, for instance stimuli, that are related to information in only one
particular data record, must be in that same data record. Even annotations
describing events preceding the start of that data record, for instance a
pre-interval stimulus must follow the time-keeping annotation.

#### 2.2.4. Time keeping of data records

Because data records need not be contiguous, the starttime of each data record
must be specified in another way. So, the first annotation of the first 'EDF
Annotations' signal in each data record is empty, but its timestamp specifies
how many seconds after the filestartdate/time that data record starts. So, if
the first TAL in a data record reads '+5672020', then that data record starts
567s after the startdate/time of the file. If the data records contain 'ordinary
signals', then the starttime of each data record must be the starttime of its
signals.

If there are no 'ordinary signals', then a non-empty annotation immediately
following the time-keeping annotation (in the same TAL) must specify what event
defines the starttime of this data record. For example, '+3456.7892020R-wave20
indicates that this data record starts at the occurrence of an R-wave, which is
3456.789s after file start.

The startdate/time of a file is specified in the EDF+ header fields 'startdate
of recording' and 'starttime of recording'. These fields must indicate the
absolute second in which the start of the first data record falls. So, the first
TAL in the first data record always starts with +0.X2020, indicating that the
first data record starts a fraction, X, of a second after the startdate/time
that is specified in the EDF+ header. If X=0, then the .X may be omitted.

### 2.3. Analysis results in EDF+

Ideally, all data (signals, annotations, events) recorded in one session using
one recording system are in one EDF+ file. Data from the same patient but from
other sessions or equipment will usually be kept in separate files. Ideally, all
these files have an identical 'Patient identification' field. In this way,
accurate synchronicity between signals and events is kept within the files and
it is exactly known to what period in which patients life the data apply.

In practice, this will not always be possible. However, it ís easy to maintain
synchronicity and patient identification between a recording and data that are
derived from that recording. Such derived data can be analysis results such as
averages, QRS parameters, peak latencies or sleep stages or simply a subset of
the recording. If such analysis results are stored in EDF+ then this must be
done as follows.

If the original recording is in file R.edf (R can be any string), then the
derived-file name must be RA.edf in which A can be any string. For instance a
PSG would be recorded in file PSG0123_2002.edf and its sleep stage analysis in
PSG0123_2002_hyp.edf. Copy the patient-id line (80 characters) from the recorded
file into the analysis file.

Make sure that startdate, starttime, and number and duration of datarecords, are
correct. So, if the analysis contains a period from 01:05:00 till 01:25:00 of a
24-hour recording that was started on August 2, 1999, 23:00:00hr, then the
analysis file should have startdate 03.08.99 and starttime 01.05.00. In this way
it is clear that both files refer to one and the same time period in the
patient's life. Some viewers (like PolyMan) are then capable of showing the two
(or more) files time-synchronized on one screen. Because the analysis may reduce
or increase the amount of data, the durations of analysis-file data records and
recording-file data records may differ.

Apply suitable scaling factors in such a way that a large part of the available
range of -32768 till 32767 for the values of the analysis results is used. If
necessary, the scaling factor can be adapted to the dynamic range of the
analysis result even after the analysis was done. Put these scaling factors in
the header (digital and physical minimum and maximum) of the analysis file. If
such scaling is really impossible because the useful dynamic range of the
analysis result is too large, but only then, apply the standardized logarithmic
transformation to store floating point values. Be aware that old EDF software is
not aware of this transformation, and will show the analysis results on a
logarithmic scale. So really try scaling first!

If a hypnogram is stored as an ordinary signal, sleep stages W,1,2,3,4,R,M
should be coded in the data records as the integer numbers 0,1,2,3,4,5,6
respectively. Unscored epochs should be coded as the integer number 9. If a
hypnogram is stored as annotations, use the standard texts.

Automatically document the analysis principle and parameters in the Recording-id
and, in case of ordinary signals, also in Label, Transducer type, Physical
dimension and Prefiltering fields in the header of the analysis file.

## 3. Some examples

### 3.1. Auditory EP recording

The following is an example of annotations in the first two data records of an
auditory EP recording. Each data record has two TALs, the first one includes the
(mandatory) time-keeping annotation, the second one specifies a pre-interval
stimulus.

```
+0&#20;&#20;Stimulus click 35dB both ears&#20;Free text&#20;&#0;
-0.065&#20;Pre-stimulus beep 1000Hz&#20;&#0;
+0.3&#20;&#20;Stimulus click 35dB both ears&#20;&#0;
+0.235&#20;Pre-stimulus beep 1000Hz&#20;&#0;
```

In this example, averaging can be triggered by the unique texts "Stimulus click 35dB both ears" and/or "Pre-stimulus beep 1000Hz".

### 3.2. Sleep recording (PSG) with MSLT

A PSG that is followed by an MSLT can be stored in separate files. The PSG file,
including lights-off and final wake-up annotations, is a continuous EDF+ file.
The MSLT is a discontinuous EDF+ file which contains only the 20-minute periods
in bed. Alternatively, the PSG and the MSLT can also be stored together into one
single (discontinuous) file.

### 3.3. Sleep scoring

A 8-24hr sleep recording takes about 30-300MB when stored in EDF+. The recording
can be analyzed manually, resulting in apnea's, leg movements and sleep stages.
These results are kept in a separate EDF+ file (about 10-100kB) which, in this
example, only contains one data record with one 'EDF Annotations' signal and no
'ordinary' signals. The table below shows the first half hour and the last few
minutes in this data record. This patient fell asleep 9 minutes after switching
off the light and had limb movements (Right and/or Left leg) and apneas after
reaching sleep stage 2 and 3, respectively. If another technician also scores
apnea's, leg movements or sleep stages, these scorings can be kept in another
separate EDF+ file.

```
+0&#20;&#20;Recording starts&#20;&#0;
+0&#21;660&#20;Sleep stage W&#20;&#0;
+120&#20;Lights off&#20;&#0;
+660&#21;300&#20;Sleep stage N1&#20;&#0;
+742&#20;Turning from right side on back&#20;&#0;
+960&#21;180&#20;Sleep stage N2&#20;&#0;
+993.2&#21;1.2&#20;Limb movement&#20;R+L leg&#20;&#0;
+1019.4&#21;0.8&#20;Limb movement&#20;R leg&#20;&#0;
+1140&#21;300&#20;Sleep stage N3&#20;&#0;
+1526.8&#21;30.0&#20;Obstructive apnea&#20;&#0;
+1603.2&#21;24.1&#20;Obstructive apnea&#20;&#0;
+1440&#21;210&#20;Sleep stage N2&#20;&#0;
+1650&#21;270&#20;Sleep stage N3&#20;&#0;
+1634&#20;Turning from back on left side&#20;&#0;
+1920&#21;30&#20;Sleep stage N2&#20;&#0;
…
…
+30100&#20;Lights on&#20;&#0;
+30210&#20;Recording ends&#20;&#0;&#0;&#0;&#0;&#0;&#0;&#0;&#0;</td>
```

### 3.4. A large neurophysiological session

The example session includes the following investigations that are stored in
separate EDF+ files. Note that the EDF+ processing software must implement any
'negativity upward' rule after reading the signal from the file. See further
label and polarity rules at http://www.edfplus.info/specs/edftexts.html.

A continuous EMG is stored in a file with two signals: the raw EMG and the
obligatory 'EDF Annotations' signal. This is a 'continuous' EDF+ file, so the
recording might alternatively be stored as an EDF file. In case of a concentric
needle electrode recording, a positivity at the centrally insulated wire
relative to the cannula of the needle is stored as a positive value in the file.

An F response is also stored in a file with the raw EMG and the annotations. The
duration of a data record equals the duration of the investigator's screen, i.e.
of the 'window' (e.g. 50 ms). Each data record contains one single response. The
annotations describe timing and any further characteristics of the stimulus.
They can also describe measured distances and latencies.

A Motor Nerve Conduction Velocity with one EMG channel is also stored in a file
with the raw EMG and the annotations. The duration of a data record equals the
duration of the 'window'. The curves from wrist and elbow stimulation are stored
in the first and the second data record, respectively. The annotations describe
timing and further characteristics of the stimulus. They can also describe
measured distances and latencies. This file is described in full detail below.

A Somatosensory Evoked Potential with four recorded signals is stored in a file
with five signals: the four raw SSEP signals and one 'EDF Annotations' signal.
The duration of a data record equals the duration of the window (e.g. 100 ms).
The annotations describe timing and characteristics of the stimulus. Another
EDF+ file contains the 4-channel averaged responses (averages of odd and even
sweeps are kept in different data records) and the 'EDF Annotations' signal
which stores stimulus characteristics and measured latencies.

A Visual Evoked Potential is investigated by recording two sagittal EEG signals
during checkerboard stimulation of the left and right field, respectively. The
left- and right-stimulated averages are stored in two separate files. Left and
right investigations are repeated once for checking reproducibility, so both
files contain two data records. Each data record lasts 300ms and contains three
signals: two EEG averages and the EDF Annotations. The annotations describe
stimulus characteristics and measured latencies. The signal sampling starts 10ms
before each stimulus, so the first two TALs in the 'left' file is 0.0002020 and
0.01020Stimulus checkerboard left20.

### 3.5. Intra-operative monitoring

Four (left and right) signals with alternating right and left stimulus are
monitored. In this case, left and right stimulation affect correspondingly
lateralized signals. So, the recording can be stored in two ('left' and 'right')
EDF+ files. Each file contains 4 electrophysiological signals and 1 'EDF
Annotations' signal. Alternatively, if sufficient amplifiers are available, the
recording can be stored in one file containing 9 signals (4 'left' and 4 'right'
electrophysiological signals and 1 'EDF Annotations' signal). In both cases, the
response to each stimulus is stored in a separate data record and the 'EDF
Annotations' signal specifies timing and characteristics of the stimulus (a.o.
whether it is left or right).

### 3.6. Routine EEG

The 10/20 system electrodes (for instance F3, C3, T3, Cz and O1 and so on) are
recorded against a common reference and saved as such in the EDF+ file.
Therefore the montages can be made during review, such as F3-C3, T3-C3, C3-Cz
and C3-O1. Because electrode locations are specified using standard texts,
re-montaging (i.e. re-referencing) EEG derivations can be done automatically.
The Annotations signal contains events such as 'Eyes Closed' or
'Hyperventilation'.

### 3.7. The Motor Nerve Conduction file

A right Median Nerve conduction velocity is investigated by recording the right
Abductor Pollicis Brevis while stimuli are given at wrist and elbow. The
averaged signal and the corresponding annotations are stored in two data
records: one for wrist and the other for elbow stimulation.

The header record contains:

| Specification                                    | Value                                                         |
| ------------------------------------------------ | ------------------------------------------------------------- |
| 8 ascii : version of this data format (0)        | 0                                                             |
| 80 ascii : local patient identification          | MCH-0234567 F 02-MAY-1951 Haagse_Harry                        |
| 80 ascii : local recording identification        | Startdate 02-MAR-2002 EMG561 BK/JOP Sony. MNC R Median Nerve. |
| 8 ascii : startdate of recording (dd.mm.yy)      | 17.04.01                                                      |
| 8 ascii : starttime of recording (hh.mm.ss)      | 11.25.00                                                      |
| 8 ascii : number of bytes in header record       | 768                                                           |
| 44 ascii : reserved                              | EDF+D                                                         |
| 8 ascii : number of data records (-1 if unknown) | 2                                                             |
| 8 ascii : duration of a data record, in seconds  | 0.050                                                         |
| 4 ascii : number of signals (ns) in data record  | 2                                                             |

| Specification                                                | 1st signal        | 2nd signal      |
| ------------------------------------------------------------ | ----------------- | --------------- |
| ns _ 16 ascii : ns _ label                                   | R APB             | EDF Annotations |
| ns _ 80 ascii : ns _ transducer type (e.g. AgAgCl electrode) | AgAgCl electrodes | &nbsp;          |
| ns _ 8 ascii : ns _ physical dimension (e.g. uV)             | mV                | &nbsp;          |
| ns _ 8 ascii : ns _ physical minimum (e.g. -500 or 34)       | -100              | -1              |
| ns _ 8 ascii : ns _ physical maximum (e.g. 500 or 40)        | 100               | 1               |
| ns _ 8 ascii : ns _ digital minimum (e.g. -2048)             | -2048             | -32768          |
| ns _ 8 ascii : ns _ digital maximum (e.g. 2047)              | 2047              | 32767           |
| ns _ 80 ascii : ns _ prefiltering (e.g. HP:0.1Hz LP:75Hz)    | HP:3Hz LP:20kHz   | &nbsp;          |
| ns _ 8 ascii : ns _ nr of samples in each data record        | 1000              | 60              |
| ns _ 32 ascii : ns _ reserved                                | &nbsp;            | &nbsp;          |

Each data record contains:

- 1000 \* 2-byte integer : R APB samples

- 60 \* 2-byte integer : EDF Annotations

The EDF Annotations signal in the 1st data record contains one TAL and is then filled out with 0-bytes until the end.

The TAL is:

```
+0&#20;&#20;Stimulus right wrist 0.2ms x 8.2mA at 6.5cm from recording site&#20;Response 7.2mV at 3.8ms&#20;
```

The EDF Annotations signal in the 2nd data record also contains one TAL:

```
+10&#20;&#20;Stimulus right elbow 0.2ms x 15.3mA at 28.5cm from recording site&#20;Response 7.2mV at 7.8ms (55.0m/s)&#20;
```

In this example, the TALs take less than 100 characters per data record, so the
header reserves 120 characters (60 'samples') for the EDF Annotations signal.

If desired, an internal structure of the annotations inside a TAL can be chosen.
This is not obligatory, usually increases size, does not improve exchange to
other systems, but can be useful locally. For example, the last TAL can be coded
as four separate TALs and the annotation inside each TAL can be coded in XML as
follows:

```xml
+10&#20;&#20;&#0;
+10&#20;Stimulus_elbow&#20;&#0;
+10&#20;<EDF_XMLnote>
   <Stimulus_elbow><duration unit="ms" >0.2</duration>
      <intensity mode="current" unit="mA">15.3</intensity>
      <position>right elbow</position>
      <distance mode="stimulus to recording" unit="cm">28.5</distance>
   </Stimulus_elbow>
</EDF_XMLnote>&#20;&#0;
+10&#20;<EDF_XMLnote>
   <measurements>
      <latency unit="ms">7.8</latency>
      <amplitude  mode=" baseline to peak" unit="mV">7.2</amplitude>
      <velocity mode = "segmental" unit = "m/s">55.0</velocity>
   </measurements>
</EDF_XMLnote>&#20;&#0;
```

## Rust crate file overview

- `src/error.rs` - EdfError enum with variants for IO, header, signal, data record, annotation, parse, XML, and JSON errors (via thiserror)
- `src/header.rs` - EdfHeader + EdfSignalHeader structs with binary read/write and serde derives
- `src/annotation.rs` - EdfAnnotation struct + TAL parsing/encoding per the EDF+ spec
- `src/edf_file.rs` - EdfFile, EdfSignal, EdfDataRecord top-level structures
- `src/io_edf.rs` - EDF binary reading (header + data records + annotation extraction) and writing
- `src/io_json.rs` - JSON serialization/deserialization via serde_json
- `src/io_xml.rs` - XML serialization/deserialization via quick-xml
- `src/lib.rs` - Module declarations and public re-exports
- `src/main.rs` - CLI with clap — auto-detects format from .edf/.json/.xml extension
- `examples/generate_examples.rs` - Generates the Motor Nerve Conduction example from EDF+ spec section 3.7
- `examples/example.edf` - EDF file binary (5008 bytes)
- `examples/example.json` - JSON representation
- `examples/example.xml` - XML representation
- `tests/integration_test.rs` - 13 integration tests including byte-perfect round-trips and CLI smoke tests
