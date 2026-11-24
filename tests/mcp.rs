//! Starts `hla-wgs mcp`, calls every tool and reads the resources.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct Srv {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
    id: u64,
}

impl Srv {
    fn new() -> Srv {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hla-wgs")).arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Srv { child, out, id: 0 }
    }
    fn send(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        let m = json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params});
        writeln!(self.child.stdin.as_mut().unwrap(), "{m}").unwrap();
        let mut l = String::new();
        self.out.read_line(&mut l).unwrap();
        let v: Value = serde_json::from_str(&l).unwrap();
        assert_eq!(v["id"], self.id);
        v
    }
    fn notify(&mut self, method: &str) {
        writeln!(self.child.stdin.as_mut().unwrap(), "{}", json!({"jsonrpc": "2.0", "method": method})).unwrap();
    }
    fn tool(&mut self, name: &str, args: Value) -> Value {
        let r = self.send("tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["result"]["isError"], false, "{name}: {r}");
        r["result"]["structuredContent"].clone()
    }
}

impl Drop for Srv {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn mcp_server_tools_and_resources() {
    let d = std::env::temp_dir().join(format!("hlawgs_mcp_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("db")).unwrap();
    // a database folder whose files exist: the run gets past the database check and fails on the input
    std::fs::write(d.join("db/hladb.json.gz"), b"x").unwrap();
    std::fs::write(d.join("db/panel.fa"), b">x\nACGT\n").unwrap();
    let db = d.join("db").to_string_lossy().into_owned();
    let pre = d.join("s1").to_string_lossy().into_owned();

    let mut s = Srv::new();
    let r = s.send("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}));
    assert_eq!(r["result"]["serverInfo"]["name"], "hla-wgs");
    s.notify("notifications/initialized");
    let r = s.send("tools/list", json!({}));
    let names: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["run_start", "run_status", "run_cancel", "result_read", "log_read", "explain", "version"]);

    let v = s.tool("version", json!({"db": db}));
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["log_schema"], "genome.log.v1");
    assert_eq!(v["database"]["exists"], true);

    // a run that fails at once: the input does not exist
    let st = s.tool("run_start", json!({"db": db, "inputs": [d.join("no.R1.fq.gz"), d.join("no.R2.fq.gz")], "prefix": pre, "threads": 1}));
    let id = st["run_id"].as_str().unwrap().to_string();
    let mut status = json!({});
    for _ in 0..100 {
        status = s.tool("run_status", json!({"run_id": id}));
        if status["state"] != "running" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_eq!(status["state"], "failed", "{status}");
    assert_eq!(status["error"]["code"], "HW-E010");
    let by_prefix = s.tool("run_status", json!({"prefix": pre}));
    assert_eq!(by_prefix["state"], "failed");

    let l = s.tool("log_read", json!({"prefix": pre, "kind": "error"}));
    assert_eq!(l["matched"], 1);
    assert_eq!(l["events"][0]["code"], "HW-E010");

    // result and decisions as a finished run writes them
    std::fs::write(format!("{pre}.json"), r#"{"schema":"genome.hla.v1","genes":[{"gene":"A","alleles":[{"allele":"A*02:01:01:01","resolution":4}]},{"gene":"B","alleles":[]}]}"#).unwrap();
    let mut f = std::fs::OpenOptions::new().append(true).open(format!("{pre}.log.jsonl")).unwrap();
    writeln!(f, "{}", json!({"schema": "genome.log.v1", "seq": 99, "t": 1.0, "kind": "decision", "step": "certainty", "subject": "A slot 1", "decision": "A*02:01:01:01 (4 fields)", "reason": "four fields: all leads above their thresholds", "values": {"lead": {"4": 30.0}}})).unwrap();
    writeln!(f, "{}", json!({"schema": "genome.log.v1", "seq": 100, "t": 1.0, "kind": "end", "step": "gene A", "counts": {"pairs": 500}})).unwrap();
    drop(f);
    let res = s.tool("result_read", json!({"prefix": pre, "gene": "A"}));
    assert_eq!(res["genes"].as_array().unwrap().len(), 1);
    let ex = s.tool("explain", json!({"prefix": pre, "gene": "A"}));
    assert_eq!(ex["summary"][0], "A slot 1: A*02:01:01:01 (4 fields) because four fields: all leads above their thresholds");
    assert_eq!(ex["counts"]["pairs"], 500);
    let l = s.tool("log_read", json!({"prefix": pre, "after_seq": 98, "step": "gene"}));
    assert_eq!(l["matched"], 1);

    let c = s.tool("run_cancel", json!({"run_id": id}));
    assert_eq!(c["stopped"], false);
    // a run that waits for input that never comes, then is stopped
    #[cfg(unix)]
    {
        let fifo = d.join("wait.R1.fq");
        assert!(Command::new("mkfifo").arg(&fifo).status().unwrap().success());
        std::fs::write(d.join("x.R2.fq"), b"@r\nACGT\n+\nIIII\n").unwrap();
        let st = s.tool("run_start", json!({"db": db, "inputs": [fifo, d.join("x.R2.fq")], "prefix": d.join("s2"), "threads": 1}));
        let id2 = st["run_id"].as_str().unwrap().to_string();
        let c = s.tool("run_cancel", json!({"run_id": id2}));
        assert_eq!(c["stopped"], true);
        assert_eq!(s.tool("run_status", json!({"run_id": id2}))["state"], "cancelled");
    }

    let r = s.send("resources/list", json!({}));
    let uris: Vec<String> = r["result"]["resources"].as_array().unwrap().iter().map(|x| x["uri"].as_str().unwrap().to_string()).collect();
    assert!(uris.contains(&"hla-wgs://manual/mcp".to_string()));
    assert!(uris.iter().any(|u| u.ends_with("s1.log.jsonl")));
    let r = s.send("resources/read", json!({"uri": "hla-wgs://manual/errors"}));
    assert!(r["result"]["contents"][0]["text"].as_str().unwrap().contains("## HW-E010"));
    let u = uris.iter().find(|u| u.ends_with("s1.json")).unwrap();
    let r = s.send("resources/read", json!({"uri": u}));
    assert!(r["result"]["contents"][0]["text"].as_str().unwrap().contains("genome.hla.v1"));
    let r = s.send("resources/read", json!({"uri": format!("file://{}", d.join("db/panel.fa").display())}));
    assert!(r["error"].is_object(), "files of other runs or folders must not be readable");
    let r = s.send("no/such", json!({}));
    assert_eq!(r["error"]["code"], -32601);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}
