use super::{ClientViewRenderer, ShippedScript};
use std::process::Command;

struct Equivalence;

impl Equivalence {
    const CHECK: &'static str = "const read = (name) => require('fs').readFileSync(name, 'utf8');
const transpiler = new Bun.Transpiler({ loader: 'js', minifyWhitespace: true });
const [authored, shipped] = process.argv.slice(-2).map((name) => transpiler.transformSync(read(name)));
console.log(authored === shipped ? 'same' : 'different');";

    fn bun_available() -> bool {
        Command::new("bun")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn same_program(authored: &str, shipped: &str) -> String {
        let root = tempfile::tempdir().expect("folder");
        let authored_path = root.path().join("authored.js");
        let shipped_path = root.path().join("shipped.js");
        std::fs::write(&authored_path, authored).expect("authored");
        std::fs::write(&shipped_path, shipped).expect("shipped");
        let output = Command::new("bun")
            .arg("-e")
            .arg(Self::CHECK)
            .arg(&authored_path)
            .arg(&shipped_path)
            .output()
            .expect("bun");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}

#[test]
fn indentation_blank_lines_and_comment_lines_are_removed() {
    let source = "class A {\n  /** @type {string} */\n  static name = 'a'\n\n  /**\n   * @param {number} x\n   */\n  run(x) {\n    // twice\n    return x * 2  \n  }\n}\n";

    assert_eq!(
        ShippedScript::compact(source),
        "class A {\nstatic name = 'a'\nrun(x) {\nreturn x * 2\n}\n}\n"
    );
}

#[test]
fn comment_marks_inside_a_line_of_code_stay() {
    let source = "  const a = 'http://x /* y */' // note\n  const b = /a*/g\n";

    assert_eq!(
        ShippedScript::compact(source),
        "const a = 'http://x /* y */' // note\nconst b = /a*/g\n"
    );
}

#[test]
fn code_after_a_comment_on_the_same_line_stays() {
    let source = "/* a */ run()\n  /** b\n   */ next()\n";

    assert_eq!(ShippedScript::compact(source), "run()\nnext()\n");
}

#[test]
fn the_shipped_runtime_is_smaller_and_compacting_again_changes_nothing() {
    let shipped = ClientViewRenderer::shipped_runtime();

    assert!(shipped.len() < ClientViewRenderer::RUNTIME.len());
    assert!(!shipped.contains("\n/**"));
    assert!(!shipped.contains("\n "));
    assert_eq!(ShippedScript::compact(shipped), shipped);
}

#[test]
fn the_shipped_runtime_is_the_same_program() {
    if !Equivalence::bun_available() {
        eprintln!("skipping: bun is unavailable");
        return;
    }
    let result = Equivalence::same_program(
        ClientViewRenderer::RUNTIME,
        ClientViewRenderer::shipped_runtime(),
    );

    assert_eq!(result, "same");
}

#[test]
fn the_shipped_behavior_runtime_is_the_same_program() {
    if !Equivalence::bun_available() {
        eprintln!("skipping: bun is unavailable");
        return;
    }
    let authored = include_str!("../behavior/runtime.js");
    let result = Equivalence::same_program(authored, &ShippedScript::compact(authored));

    assert_eq!(result, "same");
}
