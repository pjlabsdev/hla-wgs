//! Run log, schema genome.log.v1: one event stream per run, written line by line at once as
//! PREFIX.log.jsonl (for programs) and PREFIX.log.txt (for people), so that an interrupted run stays
//! readable. Header with version, commit, platform, all parameters and the inputs with size and
//! checksum; every step with start, end, time, memory and counts; every automatic decision with its
//! reason and the measured value; progress with an estimate of the time left; errors with their code.
//! The log never holds read sequences or base qualities.

use serde_json::{Value, json};
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

pub const SCHEMA: &str = "genome.log.v1";

struct Sink {
    json: std::fs::File,
    txt: std::fs::File,
    seq: u64,
}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);
static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
/// genes followed in detail (HLA_TRACE=A,DRB1 or --trace A)
static TRACE: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

pub fn secs() -> f64 {
    T0.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// open PREFIX.log.jsonl and PREFIX.log.txt (replacing older ones)
pub fn start(prefix: &str) -> std::io::Result<()> {
    secs();
    let json = std::fs::File::create(format!("{prefix}.log.jsonl"))?;
    let txt = std::fs::File::create(format!("{prefix}.log.txt"))?;
    *SINK.lock().unwrap() = Some(Sink { json, txt, seq: 0 });
    Ok(())
}

pub fn active() -> bool {
    SINK.lock().map(|s| s.is_some()).unwrap_or(false)
}

pub fn set_trace(genes: Vec<String>) {
    let _ = TRACE.set(genes);
}

pub fn traced(g: &str) -> bool {
    TRACE.get().map_or(false, |v| v.iter().any(|x| x == g))
}

/// one event: `kind` plus the fields of `v` (an object) in the JSON line, `text` in the text line
pub fn emit(kind: &str, v: Value, text: &str) {
    let Ok(mut guard) = SINK.lock() else { return };
    let Some(s) = guard.as_mut() else { return };
    s.seq += 1;
    let t = secs();
    let mut o = json!({"schema": SCHEMA, "seq": s.seq, "t": (t * 1000.0).round() / 1000.0, "kind": kind});
    if let (Some(m), Value::Object(add)) = (o.as_object_mut(), v) {
        for (k, x) in add {
            m.insert(k, x);
        }
    }
    let _ = s.json.write_all(format!("{o}\n").as_bytes());
    let _ = s.txt.write_all(format!("[{t:8.1}s] {kind:<8} {text}\n").as_bytes());
}

/// free text of a step (what the program also prints on stderr)
pub fn note(step: &str, text: &str) {
    emit("note", json!({"step": step, "text": text}), &format!("{step}: {text}"));
}

pub fn warn(step: &str, text: &str) {
    emit("warn", json!({"step": step, "text": text}), &format!("{step}: {text}"));
}

pub fn error(code: &str, msg: &str) {
    let c = crate::errors::find(code);
    emit(
        "error",
        json!({"code": code, "message": msg, "what": c.map(|c| c.what), "cause": c.map(|c| c.cause), "fix": c.map(|c| c.fix)}),
        &format!("{code} {msg}{}", c.map(|c| format!(" | what to do: {}", c.fix)).unwrap_or_default()),
    );
}

/// a decision the program took on its own: what, the reason and the measured values
pub fn decision(step: &str, subject: &str, what: &str, reason: &str, values: Value) {
    emit("decision", json!({"step": step, "subject": subject, "decision": what, "reason": reason, "values": values}), &format!("{step} {subject}: {what} ({reason})"));
}

/// detail for a traced gene only
pub fn trace(gene: &str, what: &str, values: Value) {
    if traced(gene) {
        emit("trace", json!({"gene": gene, "what": what, "values": values}), &format!("{gene} {what} {values}"));
    }
}

/// `since`: start of the step in seconds of the run (secs()), for the estimate of the time left
pub fn progress(step: &str, done: usize, total: usize, since: f64) {
    let t = secs() - since;
    let left = if done > 0 && done < total { Some(((t / done as f64) * (total - done) as f64 * 10.0).round() / 10.0) } else { None };
    emit(
        "progress",
        json!({"step": step, "done": done, "total": total, "eta_s": left}),
        &format!("{step} {done}/{total}{}", left.map(|l| format!(", about {l:.0}s left")).unwrap_or_default()),
    );
}

/// peak resident memory in MB where the system tells it without extra libraries (Linux)
pub fn peak_mb() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let l = s.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kb: f64 = l.split_whitespace().nth(1)?.parse().ok()?;
    Some((kb / 1024.0).round())
}

/// a step: `begin` writes its start, `end` its duration, peak memory and counts
pub struct Step {
    name: String,
    t: f64,
}

pub fn begin(name: &str) -> Step {
    emit("begin", json!({"step": name}), name);
    Step { name: name.to_string(), t: secs() }
}

impl Step {
    pub fn end(self, counts: Value) {
        let d = secs() - self.t;
        let mem = peak_mb();
        let txt = match &counts {
            Value::Object(m) if !m.is_empty() => format!("{} {:.1}s {}", self.name, d, m.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" ")),
            _ => format!("{} {:.1}s", self.name, d),
        };
        emit("end", json!({"step": self.name, "seconds": (d * 1000.0).round() / 1000.0, "peak_mb": mem, "counts": counts}), &txt);
    }
}

/// CRC-32 of a file; above 2 GiB only of the first and last 64 MiB (named in `scope`), so that a
/// whole-genome BAM is not read twice
pub fn checksum(p: &str) -> Option<(u64, String, &'static str)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).ok()?;
    let size = f.metadata().ok()?.len();
    let mut c = flate2::Crc::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut feed = |f: &mut std::fs::File, mut n: u64| -> Option<()> {
        while n > 0 {
            let k = f.read(&mut buf[..(n.min(1 << 20)) as usize]).ok()?;
            if k == 0 {
                break;
            }
            c.update(&buf[..k]);
            n -= k as u64;
        }
        Some(())
    };
    let part: u64 = 64 << 20;
    let scope = if size > (2u64 << 30) {
        feed(&mut f, part)?;
        f.seek(SeekFrom::Start(size - part)).ok()?;
        feed(&mut f, part)?;
        "first and last 64 MiB"
    } else {
        feed(&mut f, size)?;
        "whole file"
    };
    Some((size, format!("{:08x}", c.sum()), scope))
}

/// description of a file for the header: name without folders, size, CRC-32
pub fn file_info(p: &str) -> Value {
    let name = std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match checksum(p) {
        Some((size, crc, scope)) => json!({"name": name, "bytes": size, "crc32": crc, "crc32_scope": scope}),
        None => json!({"name": name, "bytes": Value::Null}),
    }
}

/// every switch the program reads, with its default; the header lists them all and which are set
pub const KNOBS: &[(&str, &str)] = &[
    ("GENES", "all 26 genes"),
    ("HLA_TGRP", "4 (lead for certain two and three fields)"),
    ("HLA_F401", "on (fourth field 01 with a lead of 1)"),
    ("HLA_FIELD1", "on (group withheld on near ties)"),
    ("HLA_DRBREL", "on (DRB1 exon refinement sees DRB3/4/5 reads)"),
    ("HLA_REP4", "on (a fourth field resting on a repeat length needs spanning reads)"),
    ("HLA_NOFREQ", "off (population frequency prior used)"),
    ("HLA_PADF1", "on (flanks from the first field for alleles stored without their ends)"),
    ("HLA_PAD_MIN", "8"),
    ("HLA_PAD_CACHE", "on"),
    ("HLA_IO_RETRIES", "6"),
    ("HLA_NONOVEL", "off"),
    ("HLA_NOKNOWN", "off"),
    ("HLA_TRACE", "none"),
    ("HLA_PANEL_N", "100"),
    ("HLA_PANEL_G", "25"),
    ("HLA_PANEL_C", "25"),
    ("HLA_PANEL_SE", "60"),
    ("HLA_PANEL_MW", "11"),
    ("HLA_PANEL_FULL", "80"),
    ("HLA_PANEL_W", "0"),
    ("HLA_PANEL_CLIP", "0"),
    ("HLA_PANEL_S", "70"),
    ("HLA_PANEL_B", "33"),
    ("HLA_PANEL_Q", "0"),
    ("HLA_PANEL_RESCUE", "on"),
    ("HLA_PANEL_RW", "40"),
    ("HLA_LPLUR", "on (long reads: plurality consensus)"),
    ("HLA_LSPAN", "on (long reads: rank by spanning reads)"),
    ("HLA_LEXON", "on (long reads: nearest allele by exons)"),
    ("HLA_LMERGE", "on (long reads: joint consensus of groups)"),
    ("HLA_LMU", "2"),
    ("HLA_PAD1", "off (flanks borrowed from padded alleles too)"),
    ("HLA_NOREP", "off"),
    ("HLA_NOBIG", "off"),
    ("HLA_NOEXT", "off"),
    ("HLA_NOBLIND", "off"),
    ("HLA_NOFIRM", "off"),
    ("HLA_NOPARA", "off"),
    ("HLA_NOPHASE", "off"),
    ("HLA_NOV20", "off"),
    ("HLA_NMIN", "5 (4 below 18x, 3 below 12x)"),
    ("HLA_NOV_EDGE", "15"),
    ("HLA_UNPH", "off"),
    ("HLA_UNSTHIN", "on"),
    ("HLA_PHASE", "off"),
    ("HLA_BREAK", "off"),
    ("HLA_REPV", "off"),
    ("HLA_FULLKMERS", "off"),
    ("HLA_MASK", "DBDIR/panel.fa.mask.bed"),
    ("HLA_DECOYS", "DBDIR/repeat_decoys.fa"),
    ("HLA_PANEL_RS", "as HLA_PANEL_S"),
    ("HLA_PAIR", "off"),
    ("HLA_RESCUE_V", "3"),
    ("HLA_RESCUE_BAND", "24"),
];

/// header of a run: tool, version, commit, platform, command, parameters, inputs, reference
pub fn header(mode: &str, args: &[String], params: Value, inputs: Vec<Value>, reference: Value) {
    let set: serde_json::Map<String, Value> = std::env::vars().filter(|(k, _)| k.starts_with("HLA_") || k == "GENES").map(|(k, v)| (k, json!(v))).collect();
    let defaults: serde_json::Map<String, Value> = KNOBS.iter().map(|(k, d)| (k.to_string(), json!(d))).collect();
    let v = json!({
        "tool": "hla-wgs", "version": env!("CARGO_PKG_VERSION"), "commit": option_env!("HLA_WGS_COMMIT").unwrap_or("unknown"),
        "platform": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH, "cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0)},
        "mode": mode,
        // file names only, no folders
        "command": args.iter().enumerate().map(|(i, a)| if i == 0 { "hla-wgs".to_string() } else { a.rsplit(['/', '\\']).next().unwrap_or(a).to_string() }).collect::<Vec<_>>(),
        "parameters": params, "environment_set": set, "defaults": defaults,
        "inputs": inputs, "reference": reference,
    });
    let text = format!(
        "hla-wgs {} ({}) {} {}/{}, inputs: {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("HLA_WGS_COMMIT").unwrap_or("unknown"),
        mode,
        std::env::consts::OS,
        std::env::consts::ARCH,
        v["inputs"].as_array().map(|a| a.iter().map(|x| format!("{} {} B crc32 {}", x["name"].as_str().unwrap_or("?"), x["bytes"], x["crc32"].as_str().unwrap_or("-"))).collect::<Vec<_>>().join(", ")).unwrap_or_default()
    );
    emit("header", v, &text);
}

pub fn finish(ok: bool, outputs: Vec<String>) {
    emit("end_run", json!({"ok": ok, "seconds": (secs() * 1000.0).round() / 1000.0, "peak_mb": peak_mb(), "outputs": outputs}), &format!("run {} after {:.1}s", if ok { "finished" } else { "failed" }, secs()));
}

/// hla-wgs diag PREFIX: PREFIX.diag.tar with the run log and the program version, no results and
/// no reads, for a bug report
pub fn diag(args: &[String]) {
    let Some(prefix) = args.first() else { crate::errors::fail("HW-E001", "usage: hla-wgs diag PREFIX") };
    let prefix = prefix.trim_end_matches(".json").trim_end_matches(".log.jsonl");
    let mut files: Vec<(String, Vec<u8>)> = vec![];
    for ext in ["log.jsonl", "log.txt"] {
        let p = format!("{prefix}.{ext}");
        match std::fs::read(&p) {
            Ok(b) => files.push((format!("diag.{ext}"), b)),
            Err(e) => crate::errors::fail("HW-E021", &format!("{p}: {e}")),
        }
    }
    let ver = format!("hla-wgs {}\ncommit {}\nos {} {}\n", env!("CARGO_PKG_VERSION"), option_env!("HLA_WGS_COMMIT").unwrap_or("unknown"), std::env::consts::OS, std::env::consts::ARCH);
    files.push(("version.txt".into(), ver.into_bytes()));
    let out = format!("{prefix}.diag.tar");
    std::fs::write(&out, tar(&files)).unwrap_or_else(|e| crate::errors::fail("HW-E020", &format!("{out}: {e}")));
    println!("{out}");
}

/// a plain ustar archive (readable by tar, 7-Zip, Windows Explorer)
pub fn tar(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut o = vec![];
    for (name, data) in files {
        let mut h = [0u8; 512];
        h[..name.len().min(100)].copy_from_slice(&name.as_bytes()[..name.len().min(100)]);
        let put = |h: &mut [u8; 512], at: usize, len: usize, s: &str| h[at..at + s.len().min(len)].copy_from_slice(&s.as_bytes()[..s.len().min(len)]);
        put(&mut h, 100, 8, "0000644\0");
        put(&mut h, 108, 8, "0000000\0");
        put(&mut h, 116, 8, "0000000\0");
        put(&mut h, 124, 12, &format!("{:011o}\0", data.len()));
        put(&mut h, 136, 12, &format!("{:011o}\0", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)));
        h[148..156].copy_from_slice(b"        ");
        h[156] = b'0';
        put(&mut h, 257, 6, "ustar\0");
        put(&mut h, 263, 2, "00");
        let sum: u32 = h.iter().map(|&b| b as u32).sum();
        put(&mut h, 148, 8, &format!("{sum:06o}\0 "));
        o.extend_from_slice(&h);
        o.extend_from_slice(data);
        o.resize(o.len().div_ceil(512) * 512, 0);
    }
    o.resize(o.len() + 1024, 0);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_lines_and_tar() {
        let d = std::env::temp_dir().join(format!("hlawgs_log_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let pre = d.join("s").to_string_lossy().into_owned();
        start(&pre).unwrap();
        header("test", &["x".into()], json!({"threads": 1}), vec![file_info(&format!("{pre}.log.txt"))], json!({}));
        let s = begin("panel");
        s.end(json!({"pairs_in": 3, "kept": 2, "dropped_no_kmer": 1}));
        decision("output", "A slot 1", "three fields", "lead at field 4 2.0 < 4", json!({"margin4": 2.0}));
        error("HW-E099", "test");
        finish(true, vec![]);
        let j = std::fs::read_to_string(format!("{pre}.log.jsonl")).unwrap();
        let lines: Vec<Value> = j.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines.len(), 6);
        assert!(lines.iter().all(|l| l["schema"] == SCHEMA));
        assert_eq!(lines[0]["kind"], "header");
        assert_eq!(lines[2]["counts"]["kept"], 2);
        assert_eq!(lines[4]["fix"], crate::errors::find("HW-E099").unwrap().fix);
        let t = std::fs::read_to_string(format!("{pre}.log.txt")).unwrap();
        assert_eq!(t.lines().count(), 6);
        let a = tar(&[("a.txt".into(), b"hello".to_vec())]);
        assert_eq!(a.len() % 512, 0);
        assert_eq!(&a[257..262], b"ustar");
        let _ = std::fs::remove_dir_all(&d);
    }
}
