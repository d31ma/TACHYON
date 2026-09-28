use super::CompressedFiles;
use crate::compiler::tests::{build_default, write_styled_home_and_about};
use crate::{BuildOptions, BuildProfile, WebCompiler};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

struct Sample;

impl Sample {
    const RUNTIME: &'static str = ".tachyon/tac-client.js";
    const RANDOM_SEED: u32 = 0x9E37_79B9;
    const RANDOM_MULTIPLIER: u32 = 1_664_525;
    const RANDOM_INCREMENT: u32 = 1_013_904_223;

    fn text(size: usize) -> Vec<u8> {
        "const greeting = 'hello';\n"
            .bytes()
            .cycle()
            .take(size)
            .collect()
    }

    fn random(size: usize) -> Vec<u8> {
        let mut state = Self::RANDOM_SEED;
        (0..size)
            .map(|_| {
                state = state
                    .wrapping_mul(Self::RANDOM_MULTIPLIER)
                    .wrapping_add(Self::RANDOM_INCREMENT);
                state.to_be_bytes()[0]
            })
            .collect()
    }

    fn brotli_decoded(path: &Path) -> Vec<u8> {
        let encoded = fs::read(path).expect("brotli file");
        let mut decoded = Vec::new();
        brotli::Decompressor::new(encoded.as_slice(), 4096)
            .read_to_end(&mut decoded)
            .expect("brotli decode");
        decoded
    }

    fn gzip_decoded(path: &Path) -> Vec<u8> {
        let encoded = fs::read(path).expect("gzip file");
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(encoded.as_slice())
            .read_to_end(&mut decoded)
            .expect("gzip decode");
        decoded
    }

    fn written(files: &[(PathBuf, Vec<u8>)], previous: &Path) -> tempfile::TempDir {
        let stage = tempfile::tempdir().expect("stage");
        let compressed = CompressedFiles::create(files, previous, true).expect("compress");
        compressed.write(stage.path()).expect("write variants");
        stage
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .expect("folder")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    fn has_variant(folder: &Path) -> bool {
        fs::read_dir(folder).expect("folder").any(|entry| {
            let path = entry.expect("entry").path();
            let variant = path
                .extension()
                .is_some_and(|extension| extension == "br" || extension == "gz");
            variant || (path.is_dir() && Self::has_variant(&path))
        })
    }
}

#[test]
fn release_builds_write_brotli_and_gzip_files_that_decode_to_the_same_bytes() {
    let root = tempfile::tempdir().expect("project");
    write_styled_home_and_about(root.path());
    let built = build_default(root.path()).expect("build");
    let runtime = built.output_directory().join(Sample::RUNTIME);
    let original = fs::read(&runtime).expect("runtime");
    let brotli = runtime.with_extension("js.br");
    let gzip = runtime.with_extension("js.gz");
    assert_eq!(Sample::brotli_decoded(&brotli), original);
    assert_eq!(Sample::gzip_decoded(&gzip), original);
    assert!(
        fs::metadata(&brotli).expect("brotli").len() < fs::metadata(&gzip).expect("gzip").len()
    );
    let totals = built.compression();
    assert!(totals.files() >= 1);
    assert!(totals.sent_bytes() < totals.raw_bytes());
    assert!(totals.line().expect("line").starts_with("Compressed "));
}

#[test]
fn development_builds_write_no_compressed_files() {
    let root = tempfile::tempdir().expect("project");
    write_styled_home_and_about(root.path());
    let options = BuildOptions {
        profile: BuildProfile::Development,
        ..BuildOptions::default()
    };
    let built = WebCompiler::build(root.path(), &options).expect("build");
    assert!(!Sample::has_variant(built.output_directory()));
    assert_eq!(built.compression().line(), None);
}

#[test]
fn small_files_and_other_formats_are_not_compressed() {
    let files = vec![
        (PathBuf::from("app.js"), Sample::text(4096)),
        (PathBuf::from("tiny.js"), Sample::text(100)),
        (PathBuf::from("photo.png"), Sample::text(4096)),
        (PathBuf::from("noise.wasm"), Sample::random(4096)),
    ];
    let stage = Sample::written(&files, Path::new("missing-previous-output"));
    assert_eq!(Sample::names(stage.path()), ["app.js.br", "app.js.gz"]);
}

#[test]
fn compression_is_deterministic() {
    let files = vec![(PathBuf::from("data.json"), Sample::text(8192))];
    let first = Sample::written(&files, Path::new("missing-previous-output"));
    let second = Sample::written(&files, Path::new("missing-previous-output"));
    for name in ["data.json.br", "data.json.gz"] {
        assert_eq!(
            fs::read(first.path().join(name)).expect("first"),
            fs::read(second.path().join(name)).expect("second")
        );
    }
}

#[test]
fn unchanged_files_reuse_the_previous_variants_and_changed_files_are_encoded_again() {
    let previous = tempfile::tempdir().expect("previous output");
    let source = Sample::text(4096);
    fs::write(previous.path().join("app.js"), &source).expect("previous source");
    fs::write(previous.path().join("app.js.br"), b"reused").expect("previous variant");
    let unchanged = Sample::written(
        &[(PathBuf::from("app.js"), source.clone())],
        previous.path(),
    );
    assert_eq!(
        fs::read(unchanged.path().join("app.js.br")).expect("reused"),
        b"reused"
    );
    let mut changed_source = source;
    changed_source.extend_from_slice(b"// changed\n");
    let changed = Sample::written(
        &[(PathBuf::from("app.js"), changed_source.clone())],
        previous.path(),
    );
    assert_eq!(
        Sample::brotli_decoded(&changed.path().join("app.js.br")),
        changed_source
    );
}

#[test]
fn sent_bytes_are_the_brotli_size_and_raw_files_have_none() {
    let files = vec![
        (PathBuf::from("pages/app.js"), Sample::text(4096)),
        (PathBuf::from("pages/tiny.js"), Sample::text(10)),
    ];
    let compressed = CompressedFiles::create(&files, Path::new("missing-previous-output"), true)
        .expect("compress");
    let stage = tempfile::tempdir().expect("stage");
    fs::create_dir_all(stage.path().join("pages")).expect("pages");
    compressed.write(stage.path()).expect("write");
    let brotli = fs::metadata(stage.path().join("pages/app.js.br"))
        .expect("brotli")
        .len();
    assert_eq!(compressed.sent_bytes("pages/app.js"), Some(brotli));
    assert_eq!(compressed.sent_bytes("pages/tiny.js"), None);
    let disabled =
        CompressedFiles::create(&files, Path::new("missing-previous-output"), false).expect("off");
    assert_eq!(disabled.sent_bytes("pages/app.js"), None);
}
