//! `hla-wgs mcp`: Model Context Protocol server over stdio (JSON-RPC 2.0, one message per line), so
//! that Claude and other agents can run hla-wgs and read what it did. It shows the same data as the
//! result files and the run log, never other data, and works only on this computer: it starts the
//! program itself as a child process and reads files, nothing goes over the network.
//! Tools: run_start, run_status, run_cancel (stops a run: the only changing action), result_read,
//! log_read, explain, version. Resources: the manual pages and the files of the runs it started.

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};

const PROTOCOL: &str = "2025-06-18";

/// manual pages, compiled in
pub const PAGES: &[(&str, &str)] = &[
    ("mcp", include_str!("../docs/manual/mcp.md")),
    ("log", include_str!("../docs/manual/log.md")),
    ("errors", include_str!("../docs/manual/errors.md")),
    ("output", include_str!("../docs/output.md")),
    ("index", include_str!("../docs/manual/README.md")),
    ("typing", include_str!("../docs/manual/typing.md")),
    ("long", include_str!("../docs/manual/long.md")),
    ("build-db", include_str!("../docs/manual/build-db.md")),
    ("build-mask", include_str!("../docs/manual/build-mask.md")),
    ("html", include_str!("../docs/manual/html.md")),
    ("report", include_str!("../docs/manual/report.md")),
    ("diag", include_str!("../docs/manual/diag.md")),
    ("unzip", include_str!("../docs/manual/unzip.md")),
    ("database", include_str!("../docs/manual/database.md")),
    ("switches", include_str!("../docs/manual/switches.md")),
    ("start-file", include_str!("../docs/manual/start-file.md")),
    ("procedures", include_str!("../docs/manual/procedures.md")),
];

struct Run {
    prefix: String,
    child: Child,
    cancelled: bool,
}

struct Server {
    runs: BTreeMap<String, Run>,
    next: u64,
}

fn tools() -> Value {
    let s = |props: Value, req: &[&str]| json!({"type": "object", "properties": props, "required": req});
    json!([
        {"name": "run_start", "title": "Start a typing run",
         "description": "Start hla-wgs on this computer. Short reads: inputs = [R1.fq.gz, R2.fq.gz] or [file.bam|file.cram] (CRAM: add the reference FASTA as reference). Long reads (PacBio HiFi): mode = long, inputs = [reads.fastq.gz|reads.bam]. Results and the run log are written to PREFIX.*. Returns a run_id.",
         "inputSchema": s(json!({"db": {"type": "string", "description": "database folder (built by hla-wgs build-db)"}, "inputs": {"type": "array", "items": {"type": "string"}}, "prefix": {"type": "string", "description": "output prefix, e.g. /data/out/SAMPLE"}, "threads": {"type": "integer"}, "mode": {"type": "string", "enum": ["short", "long"]}, "reference": {"type": "string"}, "trace": {"type": "string", "description": "genes to follow in detail, e.g. A,DRB1"}}), &["db", "inputs", "prefix"]),
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "run_status", "title": "Status and progress of a run",
         "description": "State (running, finished, failed, interrupted, cancelled), last step, progress with estimated time left, and the error with its code if the run failed. Give run_id, or prefix for any run with a log.",
         "inputSchema": s(json!({"run_id": {"type": "string"}, "prefix": {"type": "string"}}), &[]),
         "annotations": {"readOnlyHint": true, "openWorldHint": false}},
        {"name": "run_cancel", "title": "Stop a run",
         "description": "Stops a run that this server started. Its partial files stay; the log ends without end_run.",
         "inputSchema": s(json!({"run_id": {"type": "string"}}), &["run_id"]),
         "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}},
        {"name": "result_read", "title": "Read the result",
         "description": "The result of a finished run: PREFIX.json (schema genome.hla.v1) for short reads, the rows of PREFIX.long.tsv for long reads. Optional gene filter.",
         "inputSchema": s(json!({"prefix": {"type": "string"}, "gene": {"type": "string"}}), &["prefix"]),
         "annotations": {"readOnlyHint": true, "openWorldHint": false}},
        {"name": "log_read", "title": "Read and filter the run log",
         "description": "Events of PREFIX.log.jsonl (schema genome.log.v1), filtered by kind (header, begin, end, progress, decision, trace, note, warn, error, end_run), step, text contained, and events after a sequence number. At most limit events (default 200), the last ones when more match.",
         "inputSchema": s(json!({"prefix": {"type": "string"}, "kind": {"type": "string"}, "step": {"type": "string"}, "contains": {"type": "string"}, "after_seq": {"type": "integer"}, "limit": {"type": "integer"}}), &["prefix"]),
         "annotations": {"readOnlyHint": true, "openWorldHint": false}},
        {"name": "explain", "title": "Explain the result of a gene",
         "description": "Everything the run recorded about one gene: its result, why each allele is certain to the fields it has (leads, thresholds, competitors), why it was not typed, its counts, trace events and errors.",
         "inputSchema": s(json!({"prefix": {"type": "string"}, "gene": {"type": "string"}}), &["prefix", "gene"]),
         "annotations": {"readOnlyHint": true, "openWorldHint": false}},
        {"name": "version", "title": "Program and database version",
         "description": "Version and commit of hla-wgs, database format it needs, and for a database folder its IPD-IMGT/HLA release and format.",
         "inputSchema": s(json!({"db": {"type": "string"}}), &[]),
         "annotations": {"readOnlyHint": true, "openWorldHint": false}}
    ])
}

fn read_log(prefix: &str) -> Result<Vec<Value>, String> {
    let p = format!("{prefix}.log.jsonl");
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?;
    Ok(s.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
}

fn str_arg<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

fn db_info(d: &str) -> Value {
    let d = d.trim_end_matches('/');
    let rd = |f: &str| std::fs::read_to_string(format!("{d}/{f}")).ok().map(|x| x.trim().to_string());
    json!({"folder": d, "exists": std::path::Path::new(&format!("{d}/hladb.json.gz")).is_file(), "release": rd("release"), "format": rd("format").and_then(|f| f.parse::<u32>().ok()).unwrap_or(1)})
}

impl Server {
    fn status_of(&mut self, run_id: Option<&str>, prefix: Option<&str>) -> Result<Value, String> {
        let (prefix, mut state) = match run_id {
            Some(id) => {
                let r = self.runs.get_mut(id).ok_or(format!("unknown run_id {id}"))?;
                let st = match r.child.try_wait() {
                    Ok(None) => "running",
                    Ok(Some(_)) if r.cancelled => "cancelled",
                    Ok(Some(e)) if e.success() => "finished",
                    Ok(Some(_)) => "failed",
                    Err(_) => "unknown",
                };
                (r.prefix.clone(), Some(st))
            }
            None => (prefix.ok_or("give run_id or prefix")?.to_string(), None),
        };
        let ev = read_log(&prefix).unwrap_or_default();
        let last = |k: &str| ev.iter().rev().find(|e| e["kind"] == k).cloned();
        let err = last("error");
        let end = last("end_run");
        if state.is_none() {
            state = Some(if err.is_some() { "failed" } else if end.is_some() { "finished" } else if ev.is_empty() { "no log" } else { "running or interrupted" });
        }
        let step = ev.iter().rev().find(|e| e["kind"] == "begin" || e["kind"] == "end").map(|e| json!({"step": e["step"], "kind": e["kind"], "t": e["t"]}));
        Ok(json!({"prefix": prefix, "state": state, "progress": last("progress"), "last_step": step, "error": err, "end": end, "events": ev.len()}))
    }

    fn call(&mut self, name: &str, a: &Value) -> Result<Value, String> {
        match name {
            "run_start" => {
                let db = str_arg(a, "db").ok_or("db missing")?;
                let prefix = str_arg(a, "prefix").ok_or("prefix missing")?;
                let inputs: Vec<String> = a.get("inputs").and_then(|v| v.as_array()).ok_or("inputs missing")?.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                let threads = a.get("threads").and_then(|v| v.as_u64()).unwrap_or(4).to_string();
                let long = str_arg(a, "mode") == Some("long");
                let mut args: Vec<String> = vec![];
                if long {
                    if inputs.len() != 1 {
                        return Err("long reads: give one input".into());
                    }
                    args.extend(["long".into(), db.into(), inputs[0].clone(), prefix.into(), threads]);
                } else {
                    match inputs.len() {
                        1 => {
                            args.extend([db.into(), inputs[0].clone(), prefix.into(), threads]);
                            if let Some(r) = str_arg(a, "reference") {
                                args.push(r.into());
                            }
                        }
                        2 => args.extend([db.into(), inputs[0].clone(), inputs[1].clone(), prefix.into(), threads]),
                        _ => return Err("short reads: give two FASTQ files or one BAM/CRAM".into()),
                    }
                }
                if let Some(t) = str_arg(a, "trace") {
                    args.extend(["--trace".into(), t.into()]);
                }
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                let child = Command::new(exe).args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
                self.next += 1;
                let id = format!("run{}", self.next);
                self.runs.insert(id.clone(), Run { prefix: prefix.into(), child, cancelled: false });
                Ok(json!({"run_id": id, "prefix": prefix, "log": format!("{prefix}.log.jsonl")}))
            }
            "run_status" => self.status_of(str_arg(a, "run_id"), str_arg(a, "prefix")),
            "run_cancel" => {
                let id = str_arg(a, "run_id").ok_or("run_id missing")?;
                let r = self.runs.get_mut(id).ok_or(format!("unknown run_id {id}"))?;
                let was = matches!(r.child.try_wait(), Ok(None));
                if was {
                    r.cancelled = true;
                    let _ = r.child.kill();
                    let _ = r.child.wait();
                }
                Ok(json!({"run_id": id, "stopped": was, "note": if was { "run stopped; partial files stay" } else { "the run had already ended" }}))
            }
            "result_read" => {
                let prefix = str_arg(a, "prefix").ok_or("prefix missing")?;
                let gene = str_arg(a, "gene");
                if let Ok(s) = std::fs::read_to_string(format!("{prefix}.json")) {
                    let mut j: Value = serde_json::from_str(&s).map_err(|e| format!("{prefix}.json: {e}"))?;
                    if let (Some(g), Some(arr)) = (gene, j.get_mut("genes").and_then(|v| v.as_array_mut())) {
                        arr.retain(|x| x["gene"] == g);
                    }
                    return Ok(j);
                }
                let s = std::fs::read_to_string(format!("{prefix}.long.tsv")).map_err(|_| format!("no result for {prefix} ({prefix}.json or {prefix}.long.tsv)"))?;
                let mut it = s.lines();
                let head: Vec<&str> = it.next().unwrap_or("").split('\t').collect();
                let rows: Vec<Value> = it
                    .map(|l| Value::Object(head.iter().zip(l.split('\t')).map(|(k, v)| (k.to_string(), json!(v))).collect()))
                    .filter(|r| gene.map_or(true, |g| r["gene"] == g))
                    .collect();
                Ok(json!({"schema": "hla-wgs long.tsv", "rows": rows}))
            }
            "log_read" => {
                let prefix = str_arg(a, "prefix").ok_or("prefix missing")?;
                let ev = read_log(prefix)?;
                let limit = a.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
                let after = a.get("after_seq").and_then(|v| v.as_u64()).unwrap_or(0);
                let (kind, step, contains) = (str_arg(a, "kind"), str_arg(a, "step"), str_arg(a, "contains"));
                let m: Vec<&Value> = ev
                    .iter()
                    .filter(|e| e["seq"].as_u64().unwrap_or(0) > after)
                    .filter(|e| kind.map_or(true, |k| e["kind"] == k))
                    .filter(|e| step.map_or(true, |s| e["step"].as_str().map_or(false, |x| x == s || x.starts_with(&format!("{s} ")))))
                    .filter(|e| contains.map_or(true, |c| e.to_string().contains(c)))
                    .collect();
                let n = m.len();
                Ok(json!({"matched": n, "returned": n.min(limit), "events": m[n.saturating_sub(limit)..]}))
            }
            "explain" => {
                let prefix = str_arg(a, "prefix").ok_or("prefix missing")?;
                let g = str_arg(a, "gene").ok_or("gene missing")?;
                let ev = read_log(prefix)?;
                let mine = |e: &&Value| {
                    let sub = e["subject"].as_str().unwrap_or("");
                    sub == g || sub.starts_with(&format!("{g} ")) || e["gene"] == g || e["step"] == format!("gene {g}")
                };
                let decisions: Vec<&Value> = ev.iter().filter(|e| e["kind"] == "decision").filter(mine).collect();
                let traces: Vec<&Value> = ev.iter().filter(|e| e["kind"] == "trace").filter(mine).collect();
                let counts = ev.iter().find(|e| e["kind"] == "end" && e["step"] == format!("gene {g}")).map(|e| e["counts"].clone());
                let errors: Vec<&Value> = ev.iter().filter(|e| e["kind"] == "error").collect();
                let result = self.call("result_read", &json!({"prefix": prefix, "gene": g})).ok();
                let summary: Vec<String> = decisions.iter().map(|d| format!("{}: {} because {}", d["subject"].as_str().unwrap_or(""), d["decision"].as_str().unwrap_or(""), d["reason"].as_str().unwrap_or(""))).collect();
                Ok(json!({"gene": g, "summary": summary, "decisions": decisions, "counts": counts, "trace": traces, "errors": errors, "result": result}))
            }
            "version" => Ok(json!({
                "tool": "hla-wgs", "version": env!("CARGO_PKG_VERSION"), "commit": option_env!("HLA_WGS_COMMIT").unwrap_or("unknown"),
                "database_format_needed": crate::builddb::DB_FORMAT, "log_schema": crate::runlog::SCHEMA, "result_schema": "genome.hla.v1",
                "database": str_arg(a, "db").map(db_info),
            })),
            _ => Err(format!("unknown tool {name}")),
        }
    }

    fn resources(&self) -> Value {
        let mut v: Vec<Value> = PAGES.iter().map(|(n, _)| json!({"uri": format!("hla-wgs://manual/{n}"), "name": format!("manual: {n}"), "mimeType": "text/markdown"})).collect();
        for (id, r) in &self.runs {
            for (ext, mime) in [("json", "application/json"), ("report.json", "application/json"), ("tsv", "text/tab-separated-values"), ("conf.tsv", "text/tab-separated-values"), ("long.tsv", "text/tab-separated-values"), ("log.jsonl", "application/jsonl"), ("log.txt", "text/plain")] {
                let p = format!("{}.{ext}", r.prefix);
                if std::path::Path::new(&p).is_file() {
                    v.push(json!({"uri": format!("file://{}", abs(&p)), "name": format!("{id}: {ext}"), "mimeType": mime}));
                }
            }
        }
        json!(v)
    }

    fn read_resource(&self, uri: &str) -> Result<Value, String> {
        if let Some(n) = uri.strip_prefix("hla-wgs://manual/") {
            let (_, t) = PAGES.iter().find(|(k, _)| *k == n).ok_or(format!("no manual page {n}"))?;
            return Ok(json!({"contents": [{"uri": uri, "mimeType": "text/markdown", "text": t}]}));
        }
        let p = uri.strip_prefix("file://").ok_or("unknown resource")?;
        // only the result and log files of runs started here
        let ok = self.runs.values().any(|r| {
            let rp = abs(&r.prefix);
            ["json", "report.json", "tsv", "conf.tsv", "long.tsv", "log.jsonl", "log.txt"].iter().any(|e| p == format!("{rp}.{e}"))
        });
        if !ok {
            return Err("only the result and log files of runs started by this server can be read".into());
        }
        let t = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        Ok(json!({"contents": [{"uri": uri, "text": t}]}))
    }

    fn handle(&mut self, m: &Value) -> Option<Value> {
        let id = m.get("id").cloned();
        let method = m["method"].as_str().unwrap_or("");
        let p = &m["params"];
        let res: Result<Value, (i64, String)> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": p["protocolVersion"].as_str().unwrap_or(PROTOCOL),
                "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false}},
                "serverInfo": {"name": "hla-wgs", "title": "hla-wgs HLA typing", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "HLA typing from whole-genome sequencing reads on this computer. Start with run_start, follow with run_status, then read result_read and explain. Every allele carries the fields the reads support (resolution); explain says why. Research use only."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = p["name"].as_str().unwrap_or("");
                let args = p.get("arguments").cloned().unwrap_or(json!({}));
                Ok(match self.call(name, &args) {
                    Ok(v) => json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default()}], "structuredContent": v, "isError": false}),
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                })
            }
            "resources/list" => Ok(json!({"resources": self.resources()})),
            "resources/read" => self.read_resource(p["uri"].as_str().unwrap_or("")).map_err(|e| (-32002, e)),
            _ if id.is_none() => return None,
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        let id = id?;
        Some(match res {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((c, e)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": c, "message": e}}),
        })
    }
}

fn abs(p: &str) -> String {
    let pb = std::path::Path::new(p);
    let full = if pb.is_absolute() { pb.to_path_buf() } else { std::env::current_dir().map(|d| d.join(pb)).unwrap_or(pb.to_path_buf()) };
    full.to_string_lossy().into_owned()
}

pub fn run() {
    let mut s = Server { runs: BTreeMap::new(), next: 0 };
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let r: Vec<Value> = batch.iter().filter_map(|m| s.handle(m)).collect();
                if r.is_empty() { None } else { Some(json!(r)) }
            }
            Ok(m) => s.handle(&m),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
    // runs started here end with the server
    for r in s.runs.values_mut() {
        if matches!(r.child.try_wait(), Ok(None)) {
            let _ = r.child.kill();
        }
    }
}

#[cfg(test)]
pub fn tool_names() -> Vec<String> {
    tools().as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(|s| s.to_string())).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_tool_has_a_manual_section() {
        let doc = include_str!("../docs/manual/mcp.md");
        for t in super::tool_names() {
            assert!(doc.contains(&format!("## {t}")), "tool {t} missing in docs/manual/mcp.md");
        }
    }
}
