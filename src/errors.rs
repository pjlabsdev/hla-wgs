//! Stable error codes. Every way a run can end with an error has a code here, with what happened,
//! the usual cause and what to do. The codes never change their meaning; new ones get new numbers.
//! docs/manual/errors.md documents each code (a test checks that every code is there).

pub struct Code {
    pub code: &'static str,
    pub what: &'static str,
    pub cause: &'static str,
    pub fix: &'static str,
}

pub const CODES: &[Code] = &[
    Code { code: "HW-E001", what: "wrong command line", cause: "too few arguments or an unknown option", fix: "run hla-wgs without arguments to see the usage lines, or use the start file" },
    Code { code: "HW-E002", what: "database not found", cause: "the database folder or file named first does not exist", fix: "build it with: hla-wgs build-db hla.dat freq.json DBDIR, or let the start file download it" },
    Code { code: "HW-E003", what: "database unreadable", cause: "hladb.json.gz or panel.fa is damaged, empty or from an incompatible version", fix: "delete the database folder and build it again with hla-wgs build-db" },
    Code { code: "HW-E010", what: "input file not found", cause: "a FASTQ, BAM, CRAM or reference path does not exist", fix: "check the path; paths with spaces must be quoted" },
    Code { code: "HW-E011", what: "input file empty", cause: "a file of size 0, often an interrupted download or copy", fix: "copy or download the file again" },
    Code { code: "HW-E012", what: "input is a folder", cause: "a folder was given where a file is expected", fix: "give the file inside the folder" },
    Code { code: "HW-E013", what: "index missing", cause: "a BAM or CRAM file needs its index (.bai, .csi or .crai) next to it", fix: "create it with: samtools index FILE" },
    Code { code: "HW-E014", what: "input damaged", cause: "the BAM or CRAM file is not valid or ends early (truncated download, failing disk)", fix: "check the file with samtools quickcheck and copy it again" },
    Code { code: "HW-E015", what: "input not readable", cause: "reading failed again after all retries (disconnected drive, network share gone)", fix: "reconnect the drive or copy the input to a local disk; HLA_IO_RETRIES sets the number of retries" },
    Code { code: "HW-E016", what: "FASTQ format error", cause: "a record is malformed, or the two FASTQ files do not hold the same pairs in the same order", fix: "give the R1 and R2 files of the same run; check them with a FASTQ validator" },
    Code { code: "HW-E017", what: "reference missing for CRAM", cause: "the CRAM file needs the reference FASTA it was compressed against, with its .fai index", fix: "give the reference FASTA as last argument and create its index with samtools faidx" },
    Code { code: "HW-E020", what: "results cannot be written", cause: "the output folder does not exist or is read-only, or the disk is full", fix: "choose a writable folder with free space as OUTPREFIX" },
    Code { code: "HW-E021", what: "result file unreadable", cause: "the JSON given to hla-wgs html or diag is missing or not a hla-wgs result", fix: "give the PREFIX.json written by a typing run" },
    Code { code: "HW-E030", what: "not an IPD-IMGT/HLA hla.dat", cause: "build-db needs the unpacked hla.dat of an IPD-IMGT/HLA release", fix: "unpack hla.dat.zip (hla-wgs unzip hla.dat.zip DIR) and give the hla.dat inside" },
    Code { code: "HW-E031", what: "database cannot be written", cause: "the database folder is read-only or the disk is full", fix: "choose a writable folder with about 1 GB free" },
    Code { code: "HW-E040", what: "ZIP archive damaged", cause: "not a ZIP file, truncated, or a checksum does not match", fix: "download the archive again" },
    Code { code: "HW-E041", what: "unsafe path in ZIP archive", cause: "an entry would be written outside the target folder", fix: "do not use this archive; download it from the original source" },
    Code { code: "HW-E042", what: "ZIP compression not supported", cause: "an entry uses a method other than stored or deflate", fix: "unpack it with another program (unzip, 7-Zip)" },
    Code { code: "HW-E050", what: "repeat mask cannot be built", cause: "the genome FASTA, the BED file or the older database folder for build-mask is missing or empty", fix: "check the paths given to hla-wgs build-mask" },
    Code { code: "HW-E099", what: "internal error", cause: "a state the program does not expect (a bug)", fix: "send the diagnostic package (hla-wgs diag PREFIX) with a description to the issue tracker" },
];

pub fn find(code: &str) -> Option<&'static Code> {
    CODES.iter().find(|c| c.code == code)
}

/// end the run: message on stderr with the code and what to do, event in the run log, exit code 1
pub fn fail(code: &str, msg: &str) -> ! {
    let c = find(code);
    eprintln!("error {code}: {msg}");
    if let Some(c) = c {
        eprintln!("  what to do: {}", c.fix);
    }
    crate::runlog::error(code, msg);
    // the browser build has no process to end: the page shows the message of the panic
    #[cfg(target_arch = "wasm32")]
    panic!("error {code}: {msg}");
    #[cfg(not(target_arch = "wasm32"))]
    std::process::exit(1)
}

/// panics become HW-E099 with their place in the code, in the log and on stderr
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        crate::runlog::error("HW-E099", &format!("{msg} at {at}"));
        prev(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_unique_and_documented() {
        let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/manual/errors.md")).unwrap();
        let mut seen = std::collections::HashSet::new();
        for c in CODES {
            assert!(seen.insert(c.code), "{} twice", c.code);
            assert!(doc.contains(&format!("## {}", c.code)), "{} missing in docs/manual/errors.md", c.code);
        }
    }

    /// every code the program can raise is in the table, and nothing ends a run without a code
    #[test]
    fn codes_in_source_known() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().map_or(true, |x| x != "rs") {
                continue;
            }
            let s = std::fs::read_to_string(&p).unwrap();
            for (i, _) in s.match_indices("\"HW-E") {
                let code = &s[i + 1..(i + 8).min(s.len())];
                if !code[4..].bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
                assert!(find(code).is_some(), "{} in {} not in CODES", code, p.display());
            }
            if !p.ends_with("errors.rs") {
                assert!(!s.contains(concat!("util::", "die(")), "{} still ends a run without a code", p.display());
            }
        }
    }
}
