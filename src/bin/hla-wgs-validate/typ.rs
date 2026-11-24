//! Typst in the program: compiles the report source to PDF with the IBM Plex Mono fonts of
//! docs/paper/fonts compiled in, so the PDF does not depend on installed fonts or on a browser.
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};

const FONTS: [&[u8]; 4] = [
    include_bytes!("../../../docs/paper/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../../../docs/paper/fonts/IBMPlexMono-Medium.ttf"),
    include_bytes!("../../../docs/paper/fonts/IBMPlexMono-Italic.ttf"),
    include_bytes!("../../../docs/paper/fonts/IBMPlexMono-MediumItalic.ttf"),
];

struct Doc {
    lib: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    files: HashMap<FileId, Bytes>,
    today: (i32, u8, u8),
}

fn fid(p: &str) -> FileId {
    RootedPath::new(VirtualRoot::Project, VirtualPath::new(p).expect("path")).intern()
}

impl World for Doc {
    fn library(&self) -> &LazyHash<Library> {
        &self.lib
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        let b = self.file(id)?;
        Ok(Source::new(id, String::from_utf8(b.to_vec()).map_err(|_| FileError::InvalidUtf8)?))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| FileError::NotFound(format!("{id:?}").into()))
    }
    fn font(&self, i: usize) -> Option<Font> {
        self.fonts.get(i).cloned()
    }
    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(self.today.0, self.today.1, self.today.2)
    }
}

/// main.typ plus further files (figures) by name -> PDF bytes; errors name the source line
pub fn pdf(main: &str, files: &[(String, Vec<u8>)], today: (i32, u8, u8)) -> Vec<u8> {
    let fonts: Vec<Font> = FONTS.iter().flat_map(|f| Font::iter(Bytes::new(f.to_vec()))).collect();
    let book = FontBook::from_fonts(&fonts);
    let id = fid("main.typ");
    let mut m = HashMap::new();
    m.insert(id, Bytes::new(main.as_bytes().to_vec()));
    for (n, b) in files {
        m.insert(fid(n), Bytes::new(b.clone()));
    }
    let w = Doc { lib: LazyHash::new(Library::builder().build()), book: LazyHash::new(book), fonts, main: id, files: m, today };
    let res = typst::compile::<typst_layout::PagedDocument>(&w);
    let doc = res.output.unwrap_or_else(|errs| {
        for e in errs.iter() {
            let line = w.range(e.span).map(|r| main[..r.start.min(main.len())].lines().count());
            eprintln!("typst error at line {line:?}: {}", e.message);
        }
        std::process::exit(1)
    });
    let opts = typst_pdf::PdfOptions { creator: typst::foundations::Smart::Custom(Some("hla-wgs-validate paper".into())), ..Default::default() };
    typst_pdf::pdf(&doc, &opts).unwrap_or_else(|e| panic!("pdf: {e:?}"))
}
