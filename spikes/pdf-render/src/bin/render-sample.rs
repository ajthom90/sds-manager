use spike_pdf::{render_pdf, sample, RenderInput};
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = sample::load_assets(dir);
    let data = sample::sample_data(false, "Δοκιμή: #raw(\"x\") $math$ <b>not markup</b> ]");
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "PX-1-EU-v3" };
    match render_pdf(&input) {
        Ok(pdf) => {
            std::fs::create_dir_all(dir.join("out")).unwrap();
            let out = dir.join("out/sample-el.pdf");
            std::fs::write(&out, pdf).unwrap();
            println!("{}", out.display());
        }
        Err(msgs) => {
            eprintln!("render failed: {msgs:?}");
            std::process::exit(1);
        }
    }
}
