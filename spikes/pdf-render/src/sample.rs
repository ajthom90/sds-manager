//! Sample SDS data and asset loading shared by the `render-sample` binary and the tests.
use std::collections::HashMap;
use std::path::Path;

pub struct Assets {
    pub template: String,
    pub files: HashMap<String, Vec<u8>>,
    pub fonts: Vec<Vec<u8>>,
}

/// Load `assets/sds.typ`, `assets/*.svg` and `assets/fonts/*.ttf` from the crate directory.
pub fn load_assets(crate_dir: &Path) -> Assets {
    let assets = crate_dir.join("assets");
    let template = std::fs::read_to_string(assets.join("sds.typ")).expect("assets/sds.typ");
    let mut files = HashMap::new();
    for entry in std::fs::read_dir(&assets).expect("assets dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "svg") {
            let name = path.file_name().unwrap().to_str().unwrap();
            files.insert(format!("assets/{name}"), std::fs::read(&path).unwrap());
        }
    }
    let mut font_paths: Vec<_> = std::fs::read_dir(assets.join("fonts"))
        .expect("assets/fonts missing: run ./fetch-fonts.sh")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "ttf"))
        .collect();
    font_paths.sort();
    let fonts = font_paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    Assets { template, files, fonts }
}

/// A Greek-language SDS excerpt with a Bulgarian heading and a 120-row composition table (multi-page).
pub fn sample_data(draft: bool, user_paragraph: &str) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (0..120)
        .map(|i| serde_json::json!([format!("Ингредиент {i}"), "64-17-5", "1–5 %", "Flam. Liq. 2, H225"]))
        .collect();
    serde_json::json!({
        "product_name": "Καθαριστικό X", "supplier": "Acme", "lang": "el",
        "sds_number": "PX-1-EU", "version": "3", "revision_date": "2026-09-25", "draft": draft,
        "labels": {"sds_number": "Αρ. ΔΔΑ", "version": "Έκδοση", "revision_date": "Αναθεώρηση", "page": "Σελίδα", "of": "από"},
        "end_marker": "Τέλος δελτίου δεδομένων ασφαλείας",
        "sections": [
            {"number": "2", "title": "Προσδιορισμός επικινδυνότητας",
             "paragraphs": ["H225 Υγρό και ατμοί πολύ εύφλεκτα.", user_paragraph],
             "pictograms": ["ghs02", "ghs02"]},
            {"number": "3", "title": "Състав/информация за съставките", "paragraphs": [],
             "table": {"header": ["Name", "CAS", "%", "Classification"], "rows": rows}}
        ]
    })
}
