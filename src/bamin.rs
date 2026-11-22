//! Direct BAM/CRAM input: read the MHC window, the MHC ALT and HLA-* contigs and all unplaced
//! unmapped reads through the index, drop secondary/supplementary records, pair mates by name
//! and hand complete pairs (sequencing orientation) to stage 0. Same read set as
//! `samtools view -M -L regions` + `samtools view '*'` | `view -F 0x900` | collate | fastq.
use crate::util::Fq;
use ahash::AHashMap;
use noodles::sam::alignment::record::Flags;
use std::sync::mpsc::SyncSender;

const MHC_ALTS: [&str; 16] = [
    "chr6_GL000250v2_alt", "chr6_GL000251v2_alt", "chr6_GL000252v2_alt", "chr6_GL000253v2_alt",
    "chr6_GL000254v2_alt", "chr6_GL000255v2_alt", "chr6_GL000256v2_alt", "chr6_GL383533v1_alt",
    "chr6_KB021644v2_alt", "chr6_KI270758v1_alt", "chr6_KI270797v1_alt", "chr6_KI270798v1_alt",
    "chr6_KI270799v1_alt", "chr6_KI270800v1_alt", "chr6_KI270801v1_alt", "chr6_KI270802v1_alt",
];
const GRCH38_CHR6: usize = 170_805_979;
const GRCH37_CHR6: usize = 171_115_067;

/// regions to query, as "name:start-end" strings (1-based, inclusive)
pub fn regions(refs: &[(String, usize)]) -> Vec<String> {
    let mut out = vec![];
    for (name, len) in refs {
        let n = name.as_str();
        if n == "chr6" || n == "6" {
            let (s, e) = match *len {
                GRCH38_CHR6 => (28_510_120, 33_480_577),
                GRCH37_CHR6 => (28_477_797, 33_448_354),
                _ => {
                    eprintln!("warning: unknown chr6 length {len}; using GRCh38 MHC coordinates");
                    (28_510_120, 33_480_577)
                }
            };
            out.push(format!("{n}:{s}-{e}"));
        } else if n.starts_with("HLA-") || MHC_ALTS.contains(&n) || MHC_ALTS.iter().any(|a| a.strip_prefix("chr") == Some(n)) {
            out.push(format!("{n}:1-{len}"));
        }
    }
    out.dedup();
    out
}

fn revcomp_into(s: &[u8]) -> Vec<u8> {
    crate::util::revcomp(s)
}

struct Pairer {
    open: AHashMap<Vec<u8>, [Option<(Vec<u8>, Vec<u8>)>; 2]>,
    b1: Vec<Fq>,
    b2: Vec<Fq>,
    tx: SyncSender<(Vec<Fq>, Vec<Fq>)>,
    pub records: u64,
    pub pairs: u64,
}

impl Pairer {
    fn add(&mut self, name: &[u8], flags: Flags, seq: Vec<u8>, qual: &[u8]) {
        self.records += 1;
        if flags.intersects(Flags::SECONDARY.union(Flags::SUPPLEMENTARY)) || !flags.is_segmented() {
            return;
        }
        let mate = if flags.is_first_segment() { 0 } else if flags.is_last_segment() { 1 } else { return };
        // 0xFF marks a record without qualities
        let mut qual: Vec<u8> = if qual.len() == seq.len() && qual.first() != Some(&0xFF) { qual.to_vec() } else { vec![] };
        let seq = if flags.is_reverse_complemented() {
            qual.reverse();
            revcomp_into(&seq)
        } else {
            seq
        };
        let e = self.open.entry(name.to_vec()).or_insert([None, None]);
        e[mate] = Some((seq, qual));
        if e[0].is_some() && e[1].is_some() {
            let [a, b] = self.open.remove(name).unwrap();
            let nm = String::from_utf8_lossy(name).into_owned();
            let ((s1, q1), (s2, q2)) = (a.unwrap(), b.unwrap());
            self.b1.push(Fq { name: nm.clone(), seq: s1, qual: q1 });
            self.b2.push(Fq { name: nm, seq: s2, qual: q2 });
            self.pairs += 1;
            if self.b1.len() >= 50_000 {
                self.flush();
            }
        }
    }
    fn flush(&mut self) {
        if !self.b1.is_empty() {
            let b1 = std::mem::take(&mut self.b1);
            let b2 = std::mem::take(&mut self.b2);
            self.tx.send((b1, b2)).unwrap();
        }
    }
}

/// stream all pairs of an indexed BAM or CRAM into `tx`; returns (records seen, pairs, singletons).
/// An I/O error (USB disk dropping out, volume briefly unmounted) does not abort the run: the
/// file is reopened after a pause and reading resumes after the last record already consumed.
/// Records arrive in the same order as in an uninterrupted pass, so results are unchanged.
pub fn stream(path: &str, reference: Option<&str>, threads: usize, tx: SyncSender<(Vec<Fq>, Vec<Fq>)>) -> (u64, u64, usize) {
    let mut p = Pairer { open: AHashMap::new(), b1: vec![], b2: vec![], tx, records: 0, pairs: 0 };
    let tries = crate::util::io_retries();
    let mut attempt = 0;
    let mut last_eof: Option<u64> = None;
    loop {
        let skip = p.records;
        match pass(path, reference, threads, skip, &mut p) {
            Ok(()) => break,
            Err(e) => {
                // nothing read yet and the content is at fault (no index, not a BAM, cut short):
                // a pause does not help
                let eof = e.kind() == std::io::ErrorKind::UnexpectedEof;
                if crate::util::permanent(&e) || (eof && p.records == 0) {
                    crate::util::die(&format!("{path}: not a valid or complete BAM/CRAM with index ({e})"));
                }
                // the same early end at the same record after a reopen: the file itself is cut short
                if eof && last_eof == Some(p.records) {
                    crate::util::die(&format!("{path}: file ends early after {} records ({e}); it is truncated or damaged", p.records));
                }
                last_eof = if eof { Some(p.records) } else { None };
                attempt += 1;
                if attempt > tries {
                    crate::util::die(&format!("{path}: {e} (gave up after {tries} retries)"));
                }
                let wait = crate::util::backoff(attempt);
                eprintln!("[input] {path}: I/O error after {} records: {e}; retry {attempt}/{tries} in {wait}s", p.records);
                std::thread::sleep(std::time::Duration::from_secs(wait));
            }
        }
    }
    p.flush();
    (p.records, p.pairs, p.open.len())
}

fn err<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

/// one pass over regions and unmapped tail; records 1..=skip are read but not handed on
fn pass(path: &str, reference: Option<&str>, threads: usize, skip: u64, p: &mut Pairer) -> std::io::Result<()> {
    let mut seen: u64 = 0;
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".cram") {
        use noodles::{cram, fasta};
        let repo = reference
            .map(|src| fasta::io::indexed_reader::Builder::default().build_from_path(src).expect("reference fasta (.fai needed)"))
            .map(fasta::repository::adapters::IndexedReader::new)
            .map(fasta::Repository::new)
            .unwrap_or_default();
        let mut rd = cram::io::reader::Builder::default().set_reference_sequence_repository(repo).build_from_path(path)?;
        let header = rd.read_header()?;
        let index = cram::fs::read_associated_index(path)?;
        let refs: Vec<(String, usize)> = header.reference_sequences().iter().map(|(n, m)| (n.to_string(), m.length().get())).collect();
        for reg in regions(&refs) {
            let region = reg.parse().unwrap();
            for r in rd.query(&header, &index, &region)?.records() {
                let r = r?;
                seen += 1;
                if seen <= skip {
                    continue;
                }
                let nm = r.name().map(|n| n.to_vec()).unwrap_or_default();
                p.add(&nm, r.flags(), r.sequence().as_ref().to_vec(), r.quality_scores().as_ref());
            }
        }
        for r in rd.query_unmapped(&header, &index)? {
            let r = r?;
            seen += 1;
            if seen <= skip {
                continue;
            }
            let nm = r.name().map(|n| n.to_vec()).unwrap_or_default();
            p.add(&nm, r.flags(), r.sequence().as_ref().to_vec(), r.quality_scores().as_ref());
        }
    } else {
        use noodles::{bam, bgzf};
        // multithreaded BGZF decompression over a large read buffer: the unmapped tail of a WGS
        // BAM is several GB, and USB disks like large sequential reads
        let workers = std::num::NonZero::new(threads.clamp(1, 8)).unwrap();
        let f = std::fs::File::open(path)?;
        let mut rd = bam::io::Reader::from(bgzf::io::MultithreadedReader::with_worker_count(
            workers,
            std::io::BufReader::with_capacity(8 << 20, f),
        ));
        let header = rd.read_header()?;
        let index = bam::fs::read_associated_index(path)
            .or_else(|_| {
                let alt = path.strip_suffix(".bam").map(|b| format!("{b}.bai")).unwrap_or_default();
                noodles::bam::bai::fs::read(&alt).map(noodles::bam::Index::Bai)
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("index .bam.bai/.bai/.csi needed: {e}")))?;
        let refs: Vec<(String, usize)> = header.reference_sequences().iter().map(|(n, m)| (n.to_string(), m.length().get())).collect();
        for reg in regions(&refs) {
            let region = reg.parse().unwrap();
            for r in rd.query(&header, &index, &region)?.records() {
                let r = r?;
                seen += 1;
                if seen <= skip {
                    continue;
                }
                let nm = r.name().map(|n| n.to_vec()).unwrap_or_default();
                let seq: Vec<u8> = r.sequence().iter().collect();
                p.add(&nm, r.flags(), seq, r.quality_scores().as_ref());
            }
        }
        for r in rd.query_unmapped(&index)? {
            let r = r?;
            seen += 1;
            if seen <= skip {
                continue;
            }
            if crate::util::test_fault(seen) {
                return Err(err("injected test fault (HLA_TEST_IO_FAULT)"));
            }
            let nm = r.name().map(|n| n.to_vec()).unwrap_or_default();
            let seq: Vec<u8> = r.sequence().iter().collect();
            p.add(&nm, r.flags(), seq, r.quality_scores().as_ref());
        }
    }
    Ok(())
}
