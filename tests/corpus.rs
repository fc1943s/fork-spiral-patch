use biturbo_spiral::highlight::highlight;
use std::path::{Path, PathBuf};

fn spiral_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            spiral_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "spi" || extension == "spir") {
            found.push(path);
        }
    }
}

#[test]
fn lib_spiral_spans_are_sorted_disjoint_and_in_bounds() {
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("deps/spiral/lib/spiral");
    let mut sources = Vec::new();
    spiral_sources(&lib, &mut sources);
    assert!(sources.len() > 10, "lib/spiral sources not found under {} (run scripts/init.ps1)", lib.display());
    for source in sources {
        let text = std::fs::read_to_string(&source).unwrap_or_default();
        let length = text.encode_utf16().count() as u32;
        let spans = highlight(&text, &[(0, length)]);
        assert!(!spans.is_empty() || text.trim().is_empty(), "no spans in {}", source.display());
        let mut previous_end = 0;
        for span in &spans {
            assert!(previous_end <= span.start && span.start < span.end && span.end <= length, "{}", source.display());
            previous_end = span.end;
        }
    }
}
