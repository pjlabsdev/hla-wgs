//! `hla-wgs unzip ZIP DIR`: unpacks a ZIP archive (stored or deflated entries), so that the start
//! file needs no unzip program on a freshly installed Linux system.
use flate2::read::DeflateDecoder;
use std::io::Read;

fn u16le(b: &[u8], i: usize) -> usize {
    u16::from_le_bytes([b[i], b[i + 1]]) as usize
}
fn u32le(b: &[u8], i: usize) -> usize {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize
}

pub fn run(a: &[String]) {
    if a.len() != 2 {
        crate::errors::fail("HW-E001", "usage: hla-wgs unzip ZIP DIR");
    }
    let (zip, dir) = (&a[0], std::path::Path::new(&a[1]));
    let b = std::fs::read(zip).unwrap_or_else(|e| crate::errors::fail("HW-E010", &format!("{zip}: {e}")));
    // end of central directory: the last signature PK\5\6 within the final 64 kB
    let lo = b.len().saturating_sub(65557);
    let eocd = (lo..b.len().saturating_sub(21)).rev().find(|&i| b[i..i + 4] == [0x50, 0x4b, 0x05, 0x06]).unwrap_or_else(|| crate::errors::fail("HW-E040", &format!("{zip}: not a ZIP file")));
    let n = u16le(&b, eocd + 10);
    let mut p = u32le(&b, eocd + 16);
    for _ in 0..n {
        if p + 46 > b.len() || b[p..p + 4] != [0x50, 0x4b, 0x01, 0x02] {
            crate::errors::fail("HW-E040", &format!("{zip}: damaged central directory"));
        }
        let method = u16le(&b, p + 10);
        let crc = u32le(&b, p + 16) as u32;
        let csize = u32le(&b, p + 20);
        let usize_ = u32le(&b, p + 24);
        let (nl, xl, cl) = (u16le(&b, p + 28), u16le(&b, p + 30), u16le(&b, p + 32));
        let off = u32le(&b, p + 42);
        let name = String::from_utf8_lossy(&b[p + 46..p + 46 + nl]).to_string();
        p += 46 + nl + xl + cl;
        // only plain relative names: no root, drive letter or parent step on any system
        let plain = !name.is_empty()
            && !name.starts_with(['/', '\\'])
            && !name.contains(':')
            && name.split(['/', '\\']).all(|c| c != ".." && c != ".")
            && std::path::Path::new(&name).components().all(|c| matches!(c, std::path::Component::Normal(_)));
        if !plain {
            crate::errors::fail("HW-E041", &format!("{zip}: unsafe path {name}"));
        }
        let out = dir.join(&name);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out).unwrap_or_else(|e| crate::errors::fail("HW-E020", &format!("{}: {e}", out.display())));
            continue;
        }
        if off + 30 > b.len() || b[off..off + 4] != [0x50, 0x4b, 0x03, 0x04] {
            crate::errors::fail("HW-E040", &format!("{zip}: damaged entry {name}"));
        }
        let data = off + 30 + u16le(&b, off + 26) + u16le(&b, off + 28);
        let raw = b.get(data..data + csize).unwrap_or_else(|| crate::errors::fail("HW-E040", &format!("{zip}: truncated entry {name}")));
        let bytes = match method {
            0 => raw.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(usize_);
                DeflateDecoder::new(raw).read_to_end(&mut v).unwrap_or_else(|e| crate::errors::fail("HW-E040", &format!("{zip}: {name}: {e}")));
                v
            }
            m => crate::errors::fail("HW-E042", &format!("{zip}: {name}: compression method {m} not supported")),
        };
        // a damaged archive can still inflate: compare length and checksum with the directory
        let mut c = flate2::Crc::new();
        c.update(&bytes);
        if bytes.len() != usize_ || c.sum() != crc {
            crate::errors::fail("HW-E040", &format!("{zip}: {name}: checksum mismatch, the archive is damaged"));
        }
        if let Some(d) = out.parent() {
            std::fs::create_dir_all(d).unwrap_or_else(|e| crate::errors::fail("HW-E020", &format!("{}: {e}", d.display())));
        }
        std::fs::write(&out, bytes).unwrap_or_else(|e| crate::errors::fail("HW-E020", &format!("{}: {e}", out.display())));
    }
}
