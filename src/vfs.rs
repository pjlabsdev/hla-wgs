//! File access, clock and progress messages of the program. Native builds use the file system,
//! `std::time::Instant` and stderr. The WebAssembly build for the browser (web/) serves files that
//! the page registered under the same paths (in-memory data or the user's own local files, read
//! in pieces), keeps written files in memory for download, and hands progress lines to the page.
//! The typing code is the same in both builds, so the browser gives byte-identical results.
use std::io;

pub trait ReadSeek: io::Read + io::Seek {}
impl<T: io::Read + io::Seek> ReadSeek for T {}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::ReadSeek;
    use std::io;

    pub type Instant = std::time::Instant;

    pub fn open(p: &str) -> io::Result<Box<dyn ReadSeek + Send>> {
        Ok(Box::new(std::fs::File::open(p)?))
    }
    pub fn read_to_string(p: &str) -> io::Result<String> {
        std::fs::read_to_string(p)
    }
    pub fn create(p: &str) -> io::Result<Box<dyn io::Write>> {
        Ok(Box::new(std::fs::File::create(p)?))
    }
    pub fn write(p: &str, data: &[u8]) -> io::Result<()> {
        std::fs::write(p, data)
    }
    pub fn create_dir_all(p: &str) -> io::Result<()> {
        std::fs::create_dir_all(p)
    }
    pub fn is_dir(p: &str) -> bool {
        std::path::Path::new(p).is_dir()
    }
    pub fn is_file(p: &str) -> bool {
        std::path::Path::new(p).is_file()
    }
    pub fn exists(p: &str) -> bool {
        std::path::Path::new(p).exists()
    }
    /// size of a file; Err if missing, Ok(None) for a folder
    pub fn file_len(p: &str) -> io::Result<Option<u64>> {
        let m = std::fs::metadata(p)?;
        Ok(if m.is_dir() { None } else { Some(m.len()) })
    }
    pub fn log(s: &str) {
        eprintln!("{s}");
    }
    /// seconds since 1970 (time of the analysis in the result)
    pub fn unix_time() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::ReadSeek;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io::{self, Read, Seek, SeekFrom};
    use std::rc::Rc;

    /// milliseconds since the page started, from the browser's monotonic clock
    fn now_ms() -> f64 {
        js_sys::Reflect::get(&js_sys::global(), &"performance".into())
            .ok()
            .and_then(|p| js_sys::Reflect::get(&p, &"now".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).and_then(|f| f.call0(&p).ok()))
            .and_then(|v| v.as_f64())
            .unwrap_or_else(js_sys::Date::now)
    }
    use wasm_bindgen::JsCast;

    #[derive(Clone, Copy)]
    pub struct Instant(f64);
    impl Instant {
        pub fn now() -> Instant {
            Instant(now_ms())
        }
        pub fn elapsed(&self) -> std::time::Duration {
            Instant::now() - *self
        }
    }
    impl std::ops::Sub for Instant {
        type Output = std::time::Duration;
        fn sub(self, o: Instant) -> std::time::Duration {
            std::time::Duration::from_secs_f64(((self.0 - o.0) / 1000.0).max(0.0))
        }
    }

    pub enum Src {
        Bytes(Rc<Vec<u8>>),
        Blob(web_sys::Blob),
    }

    thread_local! {
        static FILES: RefCell<BTreeMap<String, Src>> = RefCell::new(BTreeMap::new());
        static LOG: RefCell<Option<js_sys::Function>> = RefCell::new(None);
    }

    pub fn put_bytes(p: &str, data: Vec<u8>) {
        FILES.with(|f| f.borrow_mut().insert(p.to_string(), Src::Bytes(Rc::new(data))));
    }
    pub fn put_blob(p: &str, b: web_sys::Blob) {
        FILES.with(|f| f.borrow_mut().insert(p.to_string(), Src::Blob(b)));
    }
    pub fn take_bytes(p: &str) -> Option<Vec<u8>> {
        FILES.with(|f| match f.borrow_mut().remove(p) {
            Some(Src::Bytes(b)) => Some(Rc::try_unwrap(b).unwrap_or_else(|b| (*b).clone())),
            Some(x) => {
                f.borrow_mut().insert(p.to_string(), x);
                None
            }
            None => None,
        })
    }
    /// all files below a folder (path prefix "dir/")
    pub fn list(dir: &str) -> Vec<String> {
        let pre = format!("{}/", dir.trim_end_matches('/'));
        FILES.with(|f| f.borrow().keys().filter(|k| k.starts_with(&pre)).cloned().collect())
    }
    pub fn unix_time() -> u64 {
        (js_sys::Date::now() / 1000.0) as u64
    }
    pub fn set_log(f: Option<js_sys::Function>) {
        LOG.with(|l| *l.borrow_mut() = f);
    }

    fn norm(p: &str) -> String {
        // "a/./b" and "a//b" as the native file system would read them
        let mut out: Vec<&str> = vec![];
        for c in p.split('/') {
            match c {
                "" | "." => {}
                ".." => {
                    out.pop();
                }
                x => out.push(x),
            }
        }
        format!("/{}", out.join("/"))
    }
    fn not_found(p: &str) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, format!("No such file or directory: {p}"))
    }

    /// a local file of the user, read in pieces through FileReaderSync (Web Worker only)
    struct BlobReader {
        blob: web_sys::Blob,
        rd: web_sys::FileReaderSync,
        size: u64,
        pos: u64,
        buf: Vec<u8>,
        buf_at: u64,
    }
    const CHUNK: u64 = 8 << 20;
    impl Read for BlobReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.size || out.is_empty() {
                return Ok(0);
            }
            let in_buf = self.pos >= self.buf_at && self.pos < self.buf_at + self.buf.len() as u64;
            if !in_buf {
                let end = (self.pos + CHUNK).min(self.size);
                let part = self.blob.slice_with_f64_and_f64(self.pos as f64, end as f64).map_err(|e| io::Error::other(format!("{e:?}")))?;
                let ab = self.rd.read_as_array_buffer(&part).map_err(|e| io::Error::other(format!("cannot read the file: {e:?}")))?;
                self.buf = js_sys::Uint8Array::new(&ab).to_vec();
                self.buf_at = self.pos;
                if self.buf.is_empty() {
                    return Ok(0);
                }
            }
            let off = (self.pos - self.buf_at) as usize;
            let n = out.len().min(self.buf.len() - off);
            out[..n].copy_from_slice(&self.buf[off..off + n]);
            self.pos += n as u64;
            Ok(n)
        }
    }
    impl Seek for BlobReader {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            let p = match to {
                SeekFrom::Start(x) => x as i128,
                SeekFrom::End(x) => self.size as i128 + x as i128,
                SeekFrom::Current(x) => self.pos as i128 + x as i128,
            };
            if p < 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before the start of the file"));
            }
            self.pos = p as u64;
            Ok(self.pos)
        }
    }

    struct BytesReader {
        data: Rc<Vec<u8>>,
        pos: u64,
    }
    impl Read for BytesReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let d = &self.data[..];
            let at = (self.pos as usize).min(d.len());
            let n = out.len().min(d.len() - at);
            out[..n].copy_from_slice(&d[at..at + n]);
            self.pos += n as u64;
            Ok(n)
        }
    }
    impl Seek for BytesReader {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            let len = self.data.len() as i128;
            let p = match to {
                SeekFrom::Start(x) => x as i128,
                SeekFrom::End(x) => len + x as i128,
                SeekFrom::Current(x) => self.pos as i128 + x as i128,
            };
            if p < 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before the start of the file"));
            }
            self.pos = p as u64;
            Ok(self.pos)
        }
    }

    /// The browser build runs on one thread; noodles' reference repository asks for Send.
    pub struct OneThread<T>(pub T);
    unsafe impl<T> Send for OneThread<T> {}
    unsafe impl<T> Sync for OneThread<T> {}
    impl<T: io::BufRead> io::BufRead for OneThread<T> {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            self.0.fill_buf()
        }
        fn consume(&mut self, n: usize) {
            self.0.consume(n)
        }
    }
    impl<T: Read> Read for OneThread<T> {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.0.read(b)
        }
    }
    impl<T: Seek> Seek for OneThread<T> {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.0.seek(to)
        }
    }

    pub fn open(p: &str) -> io::Result<Box<dyn ReadSeek + Send>> {
        let k = norm(p);
        FILES.with(|f| match f.borrow().get(&k) {
            Some(Src::Bytes(b)) => Ok(Box::new(OneThread(BytesReader { data: b.clone(), pos: 0 })) as Box<dyn ReadSeek + Send>),
            Some(Src::Blob(b)) => {
                let rd = web_sys::FileReaderSync::new().map_err(|e| io::Error::other(format!("FileReaderSync: {e:?}")))?;
                Ok(Box::new(OneThread(BlobReader { size: b.size() as u64, blob: b.clone(), rd, pos: 0, buf: vec![], buf_at: 0 })) as Box<dyn ReadSeek + Send>)
            }
            None => Err(not_found(p)),
        })
    }
    pub fn read_to_string(p: &str) -> io::Result<String> {
        let mut s = String::new();
        open(p)?.read_to_string(&mut s)?;
        Ok(s)
    }

    struct MemWriter {
        path: String,
        buf: Vec<u8>,
    }
    impl io::Write for MemWriter {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.buf.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            put_bytes(&self.path, self.buf.clone());
            Ok(())
        }
    }
    impl Drop for MemWriter {
        fn drop(&mut self) {
            put_bytes(&self.path, std::mem::take(&mut self.buf));
        }
    }
    pub fn create(p: &str) -> io::Result<Box<dyn io::Write>> {
        let k = norm(p);
        put_bytes(&k, vec![]);
        Ok(Box::new(MemWriter { path: k, buf: vec![] }))
    }
    pub fn write(p: &str, data: &[u8]) -> io::Result<()> {
        put_bytes(&norm(p), data.to_vec());
        Ok(())
    }
    pub fn create_dir_all(_p: &str) -> io::Result<()> {
        Ok(())
    }
    pub fn is_dir(p: &str) -> bool {
        let pre = format!("{}/", norm(p).trim_end_matches('/'));
        FILES.with(|f| f.borrow().range(pre.clone()..).next().map_or(false, |(k, _)| k.starts_with(&pre)))
    }
    pub fn is_file(p: &str) -> bool {
        FILES.with(|f| f.borrow().contains_key(&norm(p)))
    }
    pub fn exists(p: &str) -> bool {
        is_file(p) || is_dir(p)
    }
    pub fn file_len(p: &str) -> io::Result<Option<u64>> {
        let k = norm(p);
        let n = FILES.with(|f| match f.borrow().get(&k) {
            Some(Src::Bytes(b)) => Some(b.len() as u64),
            Some(Src::Blob(b)) => Some(b.size() as u64),
            None => None,
        });
        match n {
            Some(n) => Ok(Some(n)),
            None if is_dir(p) => Ok(None),
            None => Err(not_found(p)),
        }
    }
    pub fn log(s: &str) {
        LOG.with(|l| {
            if let Some(f) = l.borrow().as_ref() {
                let _ = f.call1(&wasm_bindgen::JsValue::NULL, &s.into());
            }
        });
    }
}

pub use imp::*;

/// a progress line: stderr natively, the page's progress display in the browser
#[macro_export]
macro_rules! elog {
    ($($t:tt)*) => { $crate::vfs::log(&format!($($t)*)) };
}
