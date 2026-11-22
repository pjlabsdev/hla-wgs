//! Entry points of the browser build (web/): the page registers the database and the input files
//! under paths, calls the program with the same arguments as on the command line and takes the
//! written result files back. Runs inside a Web Worker (local files are read with FileReaderSync).
use wasm_bindgen::prelude::*;

/// once per worker: a panic (including util::die) sends its message as a progress line
/// "error: ..." before the program stops (the instance cannot be called again after that)
#[wasm_bindgen]
pub fn init() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown error".to_string()
        };
        let msg = msg.strip_prefix("error: ").map(str::to_string).unwrap_or(msg);
        let at = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        crate::vfs::log(&format!("error: {msg}{at}"));
    }));
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// database format this program needs (as `hla-wgs --db-format`)
#[wasm_bindgen]
pub fn db_format() -> u32 {
    crate::builddb::DB_FORMAT
}

/// a file held in memory (database files, small inputs)
#[wasm_bindgen]
pub fn put_file(path: &str, data: Vec<u8>) {
    crate::vfs::put_bytes(path, data);
}

/// a local file of the user (File or Blob); read in pieces when the program needs it
#[wasm_bindgen]
pub fn put_blob(path: &str, blob: web_sys::Blob) {
    crate::vfs::put_blob(path, blob);
}

/// a written file, removed from memory; undefined if there is none
#[wasm_bindgen]
pub fn take_file(path: &str) -> Option<Vec<u8>> {
    crate::vfs::take_bytes(path)
}

/// paths of all files below a folder
#[wasm_bindgen]
pub fn list_files(dir: &str) -> Vec<String> {
    crate::vfs::list(dir)
}

/// progress lines of the program go to this function (one string per call)
#[wasm_bindgen]
pub fn set_log(f: Option<js_sys::Function>) {
    crate::vfs::set_log(f);
}

/// the program with a command line, e.g. ["hla-wgs", "/db", "/in/S.bam", "/out/S"];
/// on an error this throws after the line "error: ..." went to the progress function
#[wasm_bindgen]
pub fn run(args: Vec<String>) {
    crate::cli(args);
}
