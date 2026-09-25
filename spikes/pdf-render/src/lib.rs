//! THROWAWAY spike code (Sub-project 0, spike C): render an SDS-like document
//! from JSON data with embedded Typst, output PDF/A-2b.
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

pub struct RenderInput<'a> {
    /// Typst template source. It reads the data with `json(bytes(sys.inputs.data))`.
    pub template: &'a str,
    pub data: &'a serde_json::Value,
    /// Files the template may load, keyed by path without leading slash, e.g. "assets/ghs02.svg".
    pub files: &'a HashMap<String, Vec<u8>>,
    /// Raw TTF/OTF font files.
    pub fonts: &'a [Vec<u8>],
    /// Revision date (year, month, day), used as PDF creation date for reproducible output.
    pub date: (i32, u8, u8),
    /// Stable document identifier, e.g. "{sds number}-v{version}".
    pub ident: &'a str,
}

struct SdsWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: Source,
    files: HashMap<String, Bytes>,
    today: Datetime,
}

impl World for SdsWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main.id()
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { Ok(self.main.clone()) } else { Err(not_found(id)) }
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(id.get().vpath().get_without_slash()).cloned().ok_or_else(|| not_found(id))
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Some(self.today)
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.get().vpath().get_without_slash().into())
}

/// Render to PDF/A-2b bytes, or return the Typst error messages.
pub fn render_pdf(input: &RenderInput) -> Result<Vec<u8>, Vec<String>> {
    let fonts: Vec<Font> = input.fonts.iter().flat_map(|f| Font::iter(Bytes::new(f.clone()))).collect();
    let mut inputs = Dict::new();
    inputs.insert("data".into(), input.data.to_string().into_value());
    let library = Library::builder().with_inputs(inputs).build();
    let main_id = RootedPath::new(VirtualRoot::Project, VirtualPath::new("/main.typ").expect("valid path")).intern();
    let (y, m, d) = input.date;
    let today = Datetime::from_ymd(y, m, d).ok_or_else(|| vec![format!("invalid date {y}-{m}-{d}")])?;
    let world = SdsWorld {
        library: LazyHash::new(library),
        book: LazyHash::new(FontBook::from_fonts(&fonts)),
        fonts,
        main: Source::new(main_id, input.template.to_string()),
        files: input.files.iter().map(|(k, v)| (k.clone(), Bytes::new(v.clone()))).collect(),
        today,
    };
    let messages = |diags: &[typst::diag::SourceDiagnostic]| diags.iter().map(|d| d.message.to_string()).collect::<Vec<_>>();
    let doc: PagedDocument = typst::compile(&world).output.map_err(|e| messages(&e))?;
    let options = typst_pdf::PdfOptions {
        ident: Smart::Custom(input.ident.to_string()),
        standards: typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::A_2b]).map_err(|e| vec![e.message().to_string()])?,
        timestamp: Some(typst_pdf::Timestamp::new_utc(today)),
        ..Default::default()
    };
    typst_pdf::pdf(&doc, &options).map_err(|e| messages(&e))
}

pub mod sample;
