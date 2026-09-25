use spike_pdf::{render_pdf, sample, RenderInput};
use std::path::Path;
use std::process::Command;

fn render(draft: bool, paragraph: &str) -> Vec<u8> {
    let a = sample::load_assets(Path::new(env!("CARGO_MANIFEST_DIR")));
    let data = sample::sample_data(draft, paragraph);
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "PX-1-EU-v3" };
    render_pdf(&input).expect("render")
}

/// Text of one page (1-based) via poppler's pdftotext.
fn page_text(pdf: &[u8], page: u32) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.pdf");
    std::fs::write(&path, pdf).unwrap();
    let p = page.to_string();
    let out = Command::new("pdftotext").args(["-f", &p, "-l", &p, path.to_str().unwrap(), "-"]).output().expect("pdftotext (brew install poppler)");
    String::from_utf8(out.stdout).unwrap()
}

fn page_count(pdf: &[u8]) -> u32 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.pdf");
    std::fs::write(&path, pdf).unwrap();
    let out = Command::new("pdfinfo").arg(&path).output().expect("pdfinfo (brew install poppler)");
    let text = String::from_utf8(out.stdout).unwrap();
    let line = text.lines().find(|l| l.starts_with("Pages:")).expect("Pages line");
    line.split_whitespace().last().unwrap().parse().unwrap()
}

#[test]
fn multi_page_with_page_x_of_y_footer() {
    let pdf = render(false, "x");
    let n = page_count(&pdf);
    assert!(n >= 3, "120-row table should span several pages, got {n}");
    assert!(page_text(&pdf, 1).contains(&format!("Σελίδα 1 από {n}")));
    assert!(page_text(&pdf, n).contains(&format!("Σελίδα {n} από {n}")));
    assert!(page_text(&pdf, n).contains("Τέλος δελτίου δεδομένων ασφαλείας"));
}

#[test]
fn greek_and_cyrillic_text_extracts_correctly() {
    let t = page_text(&render(false, "x"), 1);
    assert!(t.contains("Προσδιορισμός επικινδυνότητας"));
    assert!(t.contains("Състав/информация за съставките"));
}

#[test]
fn table_header_repeats_on_following_pages() {
    let t = page_text(&render(false, "x"), 2);
    assert!(t.contains("Classification"), "header row missing on page 2");
    assert!(t.contains("Ингредиент"));
}

#[test]
fn user_text_is_printed_literally_not_interpreted() {
    let evil = "#raw(\"x\") $math$ <b>not markup</b> ] #set text(size: 40pt)";
    let t = page_text(&render(false, evil), 1);
    assert!(t.contains("#raw(\"x\") $math$ <b>not markup</b> ] #set text(size: 40pt)"), "got: {t}");
}

#[test]
fn identical_input_gives_identical_bytes() {
    assert_eq!(render(true, "x"), render(true, "x"));
}

fn upper_ascii_counts(s: &str) -> [usize; 26] {
    let mut c = [0; 26];
    for b in s.bytes().filter(u8::is_ascii_uppercase) {
        c[(b - b'A') as usize] += 1;
    }
    c
}

#[test]
fn draft_watermark_only_on_drafts() {
    // The rotated watermark is extracted as scattered letters, so compare letter counts:
    // a draft page has exactly the letters D, R, A, F, T more than the final page.
    let (draft, fin) = (page_text(&render(true, "x"), 1), page_text(&render(false, "x"), 1));
    let (d, f, w) = (upper_ascii_counts(&draft), upper_ascii_counts(&fin), upper_ascii_counts("DRAFT"));
    for i in 0..26 {
        assert_eq!(d[i], f[i] + w[i], "letter {}", (b'A' + i as u8) as char);
    }
}

#[test]
fn bad_data_returns_error_not_panic() {
    let a = sample::load_assets(Path::new(env!("CARGO_MANIFEST_DIR")));
    let data = serde_json::json!({"product_name": "only this"});
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "x" };
    let err = render_pdf(&input).expect_err("missing fields must fail");
    assert!(!err.is_empty());
}
