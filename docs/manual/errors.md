# Error codes

Every way a run of `hla-wgs` can end with an error has a stable code. The program prints it on stderr as
`error HW-Ennn: message` followed by what to do, writes it as an `error` event to the run log
(`PREFIX.log.jsonl`, see [run log](log.md)) and ends with exit code 1. A code never changes its meaning;
new situations get new numbers. The table in the source is `src/errors.rs`; a test checks that every code
there has a section here.

| Code | What |
|---|---|
| [HW-E001](#hw-e001) | wrong command line |
| [HW-E002](#hw-e002) | database not found |
| [HW-E003](#hw-e003) | database unreadable |
| [HW-E010](#hw-e010) | input file not found |
| [HW-E011](#hw-e011) | input file empty |
| [HW-E012](#hw-e012) | input is a folder |
| [HW-E013](#hw-e013) | index missing |
| [HW-E014](#hw-e014) | input damaged |
| [HW-E015](#hw-e015) | input not readable |
| [HW-E016](#hw-e016) | FASTQ format error |
| [HW-E017](#hw-e017) | reference missing for CRAM |
| [HW-E020](#hw-e020) | results cannot be written |
| [HW-E021](#hw-e021) | result file unreadable |
| [HW-E030](#hw-e030) | not an IPD-IMGT/HLA hla.dat |
| [HW-E031](#hw-e031) | database cannot be written |
| [HW-E040](#hw-e040) | ZIP archive damaged |
| [HW-E041](#hw-e041) | unsafe path in ZIP archive |
| [HW-E042](#hw-e042) | ZIP compression not supported |
| [HW-E050](#hw-e050) | repeat mask cannot be built |
| [HW-E099](#hw-e099) | internal error |

## HW-E001

**What:** wrong command line.

**Usual cause:** too few arguments or an unknown option.

**What to do:** run hla-wgs without arguments to see the usage lines, or use the start file.

## HW-E002

**What:** database not found.

**Usual cause:** the database folder or file named first does not exist.

**What to do:** build it with: hla-wgs build-db hla.dat freq.json DBDIR, or let the start file download it.

## HW-E003

**What:** database unreadable.

**Usual cause:** hladb.json.gz or panel.fa is damaged, empty or from an incompatible version.

**What to do:** delete the database folder and build it again with hla-wgs build-db.

## HW-E010

**What:** input file not found.

**Usual cause:** a FASTQ, BAM, CRAM or reference path does not exist.

**What to do:** check the path; paths with spaces must be quoted.

## HW-E011

**What:** input file empty.

**Usual cause:** a file of size 0, often an interrupted download or copy.

**What to do:** copy or download the file again.

## HW-E012

**What:** input is a folder.

**Usual cause:** a folder was given where a file is expected.

**What to do:** give the file inside the folder.

## HW-E013

**What:** index missing.

**Usual cause:** a BAM or CRAM file needs its index (.bai, .csi or .crai) next to it.

**What to do:** create it with: samtools index FILE.

## HW-E014

**What:** input damaged.

**Usual cause:** the BAM or CRAM file is not valid or ends early (truncated download, failing disk).

**What to do:** check the file with samtools quickcheck and copy it again.

## HW-E015

**What:** input not readable.

**Usual cause:** reading failed again after all retries (disconnected drive, network share gone).

**What to do:** reconnect the drive or copy the input to a local disk; HLA_IO_RETRIES sets the number of retries.

## HW-E016

**What:** FASTQ format error.

**Usual cause:** a record is malformed, or the two FASTQ files do not hold the same pairs in the same order.

**What to do:** give the R1 and R2 files of the same run; check them with a FASTQ validator.

## HW-E017

**What:** reference missing for CRAM.

**Usual cause:** the CRAM file needs the reference FASTA it was compressed against, with its .fai index.

**What to do:** give the reference FASTA as last argument and create its index with samtools faidx.

## HW-E020

**What:** results cannot be written.

**Usual cause:** the output folder does not exist or is read-only, or the disk is full.

**What to do:** choose a writable folder with free space as OUTPREFIX.

## HW-E021

**What:** result file unreadable.

**Usual cause:** the JSON given to hla-wgs html or diag is missing or not a hla-wgs result.

**What to do:** give the PREFIX.json written by a typing run.

## HW-E030

**What:** not an IPD-IMGT/HLA hla.dat.

**Usual cause:** build-db needs the unpacked hla.dat of an IPD-IMGT/HLA release.

**What to do:** unpack hla.dat.zip (hla-wgs unzip hla.dat.zip DIR) and give the hla.dat inside.

## HW-E031

**What:** database cannot be written.

**Usual cause:** the database folder is read-only or the disk is full.

**What to do:** choose a writable folder with about 1 GB free.

## HW-E040

**What:** ZIP archive damaged.

**Usual cause:** not a ZIP file, truncated, or a checksum does not match.

**What to do:** download the archive again.

## HW-E041

**What:** unsafe path in ZIP archive.

**Usual cause:** an entry would be written outside the target folder.

**What to do:** do not use this archive; download it from the original source.

## HW-E042

**What:** ZIP compression not supported.

**Usual cause:** an entry uses a method other than stored or deflate.

**What to do:** unpack it with another program (unzip, 7-Zip).

## HW-E050

**What:** repeat mask cannot be built.

**Usual cause:** the genome FASTA, the BED file or the older database folder for build-mask is missing or empty.

**What to do:** check the paths given to hla-wgs build-mask.

## HW-E099

**What:** internal error.

**Usual cause:** a state the program does not expect (a bug).

**What to do:** send the diagnostic package (hla-wgs diag PREFIX) with a description to the issue tracker.
