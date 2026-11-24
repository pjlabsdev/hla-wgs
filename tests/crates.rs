//! The crates in crates/ are an exact copy of Genome 4 (crates/README.md): every file has the
//! checksum listed there.
use std::path::Path;

fn crc32(b: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in b {
        c ^= x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

#[test]
fn copies_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates");
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    let mut n = 0;
    for line in readme.lines().filter(|l| l.starts_with("| gc-")) {
        let f: Vec<&str> = line.split('|').map(str::trim).collect();
        // git may check out text files with CRLF on Windows
        let text = std::fs::read(root.join(f[1])).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
        let text: Vec<u8> = text.iter().copied().filter(|&b| b != b'\r').collect();
        assert_eq!(format!("{:08x}", crc32(&text)), f[2], "crates/{} differs from the copy of Genome 4; change it there and copy it over (crates/README.md)", f[1]);
        n += 1;
    }
    assert_eq!(n, 3);
}
