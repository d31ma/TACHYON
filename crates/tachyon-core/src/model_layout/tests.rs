use crate::model_layout::validate;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

fn assert_accepted(paths: &[&str], source: &str) {
    for path in paths {
        validate(Path::new(path), source).unwrap_or_else(|error| panic!("{path}: {error}"));
    }
}

fn refusal(path: &str, source: &str) -> Diagnostic {
    let failure = validate(Path::new(path), source).expect_err(source);
    failure.diagnostics()[0].clone()
}

fn refused_at(diagnostic: &Diagnostic) -> usize {
    usize::try_from(diagnostic.spans[0].start).expect("span start")
}

fn assert_refused(path: &str, message: &str, cases: &[(&str, &str)]) {
    for (source, token) in cases {
        let diagnostic = refusal(path, source);
        assert_eq!(diagnostic.code.number(), 1310, "{source}");
        assert_eq!(diagnostic.message, message, "{source}");
        let at = refused_at(&diagnostic);
        assert!(
            source[at..].starts_with(token),
            "{source}: refused at {at}, expected '{token}'"
        );
    }
}

fn assert_role_layout(path: &str, source: &str) {
    let failure = crate::annotation_layout::validate(Path::new(path), source).expect_err(source);
    let message = failure.diagnostics()[0].message.clone();
    assert!(
        message.ends_with("sits outside its realm's shared role folder"),
        "{path}: {message}"
    );
}

fn assert_language_refused(path: &str, message: &str, help: &str) {
    let diagnostic = refusal(path, "");
    assert_eq!(diagnostic.code.number(), 1310, "{path}");
    assert_eq!(diagnostic.message, message, "{path}");
    assert_eq!(diagnostic.help.as_deref(), Some(help), "{path}");
}

#[test]
fn behavior_help_names_the_realm_folders() {
    let diagnostic = refusal("server/models/Guide.rs", "impl Guide {}\n");
    let help = "Move behavior to a service or role file, for example 'server/services/', and plain code to 'server/shared/'.";
    assert_eq!(diagnostic.help.as_deref(), Some(help));
}

#[test]
fn each_realm_holds_its_own_languages() {
    assert_language_refused(
        "server/models/Guide.swift",
        "'Guide.swift' is a Swift file, and only 'client/models/' holds Swift types",
        "Swift is not a Yon language. Declare this type in a server language in 'server/models/', or move it to 'client/models/'.",
    );
    for (extension, language) in [("groovy", "Groovy"), ("php", "PHP")] {
        assert_language_refused(
            &format!("client/models/Guide.{extension}"),
            &format!(
                "'Guide.{extension}' is a {language} file, and only 'server/models/' holds {language} types"
            ),
            &format!(
                "{language} runs only in the server realm. Declare this type in a client language in 'client/models/', or move it to 'server/models/'."
            ),
        );
    }
    assert_language_refused(
        "client/models/Guide.lua",
        "'Guide.lua' is in a language that a models folder does not hold",
        "'client/models/' holds data types in TypeScript, JavaScript, Rust, C#, Dart, Go, Kotlin, Swift, Java, Python and Ruby. Move code to its role folder, for example 'client/services/'.",
    );
    assert_language_refused(
        "server/models/nested/Guide.lua",
        "'Guide.lua' is in a language that a models folder does not hold",
        "'server/models/' holds data types in TypeScript, JavaScript, Rust, C#, Dart, Go, Kotlin, Java, Groovy, Python, PHP and Ruby. Move code to its role folder, for example 'server/services/'.",
    );
}

#[test]
fn python_models_remain_data_only_in_both_realms() {
    assert_accepted(
        &["client/models/Guide.py", "server/models/Guide.py"],
        "from dataclasses import dataclass\n\n@dataclass\nclass Guide:\n    title: str\n    pages: int\n",
    );
    for path in ["client/models/Guide.py", "server/models/Guide.py"] {
        let diagnostic = refusal(
            path,
            "class Guide:\n    title: str\n\n    def read(self):\n        return self.title\n",
        );
        assert_eq!(diagnostic.code.number(), 1310);
    }
}

#[test]
fn javascript_models_hold_jsdoc_typedefs() {
    assert_accepted(
        &["client/models/Guide.js", "server/models/Guide.js"],
        "/**\n * @typedef {Object} Guide\n * @property {string} title\n */\nexport {}\n",
    );
    assert_accepted(
        &["client/models/Guide.js"],
        "/** @import { Section } from '/models/Section.ts' */\n/**\n * @typedef {Object} Guide\n * @property {Section[]} sections\n */\nexport {};\n",
    );
}

#[test]
fn javascript_models_refuse_code() {
    assert_refused(
        "client/models/Guide.js",
        "A JavaScript models file declares only JSDoc @typedef comments and `export {}`",
        &[
            (
                "/** @typedef {Object} Guide */\nexport class Guide {}\n",
                "export class",
            ),
            (
                "/** @typedef {Object} Guide */\nconst guide = {};\nexport {}\n",
                "const guide",
            ),
            ("export { Guide } from './Other.js'\n", "export {"),
        ],
    );
}

#[test]
fn role_markers_keep_the_role_layout_message() {
    for (path, source) in [
        (
            "server/models/GuideService.rs",
            "#[yon::service]\npub struct GuideService;\n",
        ),
        (
            "client/models/GuideService.cs",
            "[Service]\npublic sealed class GuideService {}\n",
        ),
        (
            "client/models/GuideService.dart",
            "@Service\nclass GuideService {}\n",
        ),
        (
            "client/models/GuideService.swift",
            "@Service\nfinal class GuideService {}\n",
        ),
        (
            "client/models/GuideService.kt",
            "@Service\nclass GuideService\n",
        ),
        (
            "server/models/GuideService.java",
            "@Service\npublic class GuideService {}\n",
        ),
        (
            "server/models/GuideService.py",
            "@service\nclass GuideService:\n    pass\n",
        ),
        (
            "server/models/GuideService.php",
            "<?php\n#[Service]\nfinal class GuideService {}\n",
        ),
    ] {
        assert_role_layout(path, source);
    }
}

#[test]
fn empty_models_files_are_valid() {
    for extension in ["rs", "cs", "dart", "kt", "go"] {
        assert_accepted(
            &[
                &format!("client/models/Empty.{extension}"),
                &format!("server/models/Empty.{extension}"),
            ],
            "",
        );
    }
    assert_accepted(&["client/models/Empty.swift"], "\n");
    assert_accepted(
        &[
            "server/models/Empty.java",
            "server/models/Empty.py",
            "server/models/Empty.php",
        ],
        "\n",
    );
}

#[test]
fn ruby_models_hold_records_and_documented_classes() {
    let record = "# frozen_string_literal: true\n\n# @!attribute [r] title\n#   @return [String]\nGuide = Data.define(\n  :title,\n  :sections\n)\n";
    let class = "class Guide < Base\n  # @return [String]\n  attr_reader :title\n\n  def initialize(title:)\n    @title = title\n  end\nend\n";
    let subclass = "class Point < Data.define(:x, :y)\nend\n";
    for source in [record, class, subclass] {
        assert_accepted(
            &["server/models/Guide.rb", "client/models/Guide.rb"],
            source,
        );
    }
    assert_refused(
        "server/models/Guide.rb",
        "A Ruby models file declares only Data.define and Struct.new records, classes with attr_reader fields and an initialize method, and the require lines they need",
        &[
            ("class Guide\n  def describe\n  end\nend\n", "def describe"),
            ("puts 'loaded'\n", "puts"),
            (
                "Guide = Struct.new(:title) do\n  def x; end\nend\n",
                "Guide =",
            ),
            ("class Guide\n  attr_reader :title\n", ""),
        ],
    );
}

#[test]
fn rust_models_hold_data_types() {
    let source = r#"use serde::{Deserialize, Serialize};
// fn helper() {} impl Guide {}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "impl Guide { fn describe() {} }"]
pub struct Guide {
    pub title: String,
    pub(crate) sections: Vec<Section>,
    #[serde(default)]
    tags: Vec<String>,
}
pub struct Pair(pub u8, pub u8);
pub(in crate::models) struct Unit;
#[repr(u8)]
pub enum Kind {
    Guide = 1,
    Recipe { steps: u32 },
    Note(String),
}
pub type Guides = Vec<Guide>;
"#;
    assert_accepted(
        &["client/models/Guide.rs", "server/models/Guide.rs"],
        source,
    );
}

#[test]
fn rust_models_refuse_behavior() {
    assert_refused(
        "client/models/Guide.rs",
        "A Rust models file declares only structs, enums, type aliases and the use declarations they need",
        &[
            ("struct Guide;\nimpl Guide {}\n", "impl"),
            ("struct Guide;\nfn helper() {}\n", "fn"),
            ("pub const LIMIT: u8 = 3;\n", "const"),
            ("static NAME: &str = \"x\";\n", "static"),
            ("pub trait Named {}\n", "trait"),
            ("mod nested {}\n", "mod"),
            ("macro_rules! guide { () => {} }\n", "macro_rules"),
            ("#![allow(dead_code)]\nstruct Guide;\n", "#![allow"),
            ("pub(crate) union Bits { a: u8 }\n", "union"),
            ("extern crate serde;\n", "extern"),
            ("#[Lifetime]\npub struct Guide;\n", "#[Lifetime]"),
            (
                "pub struct Guide {\n    #[yon::on_start]\n    title: String,\n}\n",
                "#[yon::on_start]",
            ),
        ],
    );
}

#[test]
fn go_models_hold_one_data_type() {
    let sources = [
        "package main\n\nimport (\n\t\"time\"\n\tformat \"fmt\"\n)\n\n// func helper() {} var limit = 3\ntype Guide struct {\n\tTitle   string    `json:\"title\"`\n\tPages   int       `json:\"pages,omitempty\"`\n\tUpdated time.Time `json:\"updated\"`\n\tTags    []string\n\tMeta    struct {\n\t\tOwner string\n\t}\n\tSection\n\t*Shelf\n\tOnChange func(int) error\n\tAny      interface{}\n\tLabel    format.Stringer\n}\n",
        "package main; type Kind string\n",
        "import \"encoding/json\"\n\ntype Page[T any] struct {\n\tItems []T\n\tRaw   json.RawMessage\n}\n",
        "type Guides = []Guide\n",
        "type Lookup map[string]Guide\n",
    ];
    for source in sources {
        assert_accepted(
            &["client/models/Guide.go", "server/models/Guide.go"],
            source,
        );
    }
}

#[test]
fn go_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.go",
        "A Go models file declares only one struct, other defined type or type alias, and the imports it needs",
        &[
            (
                "type Guide struct{}\n\nfunc (guide Guide) Title() string { return \"\" }\n",
                "func (guide",
            ),
            ("func helper() {}\n", "func helper"),
            ("type Guide struct{}\nfunc init() {}\n", "func init"),
            ("const Limit = 3\n", "const"),
            ("var Shelf = []int{}\n", "var"),
            ("type Guide struct{}\ntype Kind string\n", "type Kind"),
            ("type (\n\tGuide struct{}\n)\n", "type ("),
            ("type Named interface {\n\tName() string\n}\n", "interface"),
            ("type Named[T any] interface{ Name() T }\n", "interface"),
            ("import _ \"embed\"\n", "_"),
            ("import (\n\t\"fmt\"\n\t_ \"embed\"\n)\n", "_"),
            ("type Guide struct{}\npackage main\n", "package"),
            ("fmt.Println(1)\n", "fmt"),
        ],
    );
}

#[test]
fn go_models_belong_to_package_main() {
    for source in ["package models\n\ntype Guide struct{}\n", "package\n"] {
        let diagnostic = refusal("client/models/Guide.go", source);
        assert_eq!(diagnostic.code.number(), 1310, "{source}");
        assert_eq!(
            diagnostic.message,
            "A Go models file belongs to package main; found another package clause"
        );
        assert_eq!(
            diagnostic.help.as_deref(),
            Some(
                "Write `package main` as the first line, or leave the package clause out. Go code uses a model by its type name alone, without an import."
            )
        );
        assert_eq!(refused_at(&diagnostic), 0, "{source}");
    }
}

#[test]
fn csharp_models_hold_data_types() {
    let source = r#"using System;
using System.Collections.Generic;
using static System.Math;
using Names = System.Collections.Generic.List<string>;
#nullable enable

// public void Run() { }
[Serializable]
public sealed record Guide(string Title, [property: JsonPropertyName("n")] int Count);
public record struct Point(int X, int Y);
public readonly record struct Size(int Width) { public int Height { get; init; } }
public class Section
{
    #region Größe
    public string Title { get; set; } = "void Run() { }";
    public required List<string> Tags { get; init; }
    public int Count { get; private set; }
    public string Name;
    private readonly List<string> items = new();
    public const int Limit = 3;
    [JsonIgnore] public int Hidden;
    public (int, int) Pair;
    #endregion
}
internal struct Pair { public int Left; public int Right; }
public enum Kind : byte { Guide = 1, Recipe }
"#;
    assert_accepted(
        &["client/models/Guide.cs", "server/models/Guide.cs"],
        source,
    );
}

#[test]
fn csharp_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.cs",
        "A C# models file declares only records, classes and structs with fields and properties, enums and the using directives they need",
        &[
            ("public class Guide { public void Run() {} }", "void Run"),
            ("public class Guide { public Guide() {} }", "Guide()"),
            ("public class Guide { public int Count => 1; }", "int Count"),
            (
                "public class Guide { public int Count { get { return 1; } } }",
                "get {",
            ),
            (
                "public class Guide { public int Count { get => 1; } }",
                "get =>",
            ),
            (
                "public class Guide { public string this[int i] { get; } }",
                "string this",
            ),
            (
                "public class Guide { public event Action Changed; }",
                "event",
            ),
            (
                "public class Guide { public class Inner {} }",
                "class Inner",
            ),
            (
                "public record Guide(string Title) { public string Describe() => Title; }",
                "string Describe",
            ),
            ("namespace Models;", "namespace"),
            ("public interface IGuide {}", "interface"),
            ("public delegate void Handler();", "delegate"),
            ("public static class Guides {}", "static"),
            ("global using System;", "global"),
            ("using (var reader = Open()) {}", "using"),
            ("Console.WriteLine(1);", "Console"),
            ("[OnStart]\npublic class Guide {}", "OnStart"),
            (
                "public class Guide { [LifetimeAttribute] public int Count; }",
                "LifetimeAttribute",
            ),
        ],
    );
}

#[test]
fn dart_models_hold_data_types() {
    let source = r"import 'package:meta/meta.dart';
import 'section.dart' as section show Section;

// void main() {}
typedef Guides = List<Guide>;

class Guide {
  final String title;
  int count = 0;
  late String note;
  static const limit = 3;
  final void Function(int) onChange;
  final Map<String, int> totals = {};
  String label = 'get set { }';

  Guide(this.title, this.onChange);
  Guide.named(this.title, {required this.onChange}) : count = 1;
  Guide.copy(Guide other) : this(other.title, other.onChange);
}

sealed class Shape {}

final class Circle extends Shape {
  final double radius;
  const Circle(this.radius);
}

enum Kind { guide, recipe }

enum Level {
  low(1),
  high(2);

  final int value;
  const Level(this.value);
}
";
    assert_accepted(
        &["client/models/guide.dart", "server/models/guide.dart"],
        source,
    );
}

#[test]
fn dart_models_refuse_behavior() {
    assert_refused(
        "client/models/guide.dart",
        "A Dart models file declares only classes with fields and constructors, enums, typedefs and the imports they need",
        &[
            ("class Guide { void run() {} }", "void run"),
            ("class Guide { int get count => 1; }", "int get"),
            ("class Guide { set count(int value) {} }", "set"),
            ("class Guide { factory Guide() => Guide._(); }", "factory"),
            ("class Guide { Guide() { print(1); } }", "Guide()"),
            (
                "class Guide { bool operator ==(Object other) => true; }",
                "bool",
            ),
            (
                "class Guide { String describe() => ''; }",
                "String describe",
            ),
            ("class Guide { void buildFunction(); }", "void"),
            ("void main() {}", "void"),
            ("final guides = <Guide>[];", "final"),
            ("mixin Named {}", "mixin"),
            ("extension GuideText on Guide {}", "extension"),
            ("export 'guide.dart';", "export"),
            ("part 'guide.g.dart';", "part"),
            ("@immutable\nclass Guide {}", "@immutable"),
            (
                "class Guide { @override final String title = ''; }",
                "@override",
            ),
            ("enum Kind { a; void run() {} }", "void run"),
        ],
    );
}

#[test]
fn swift_models_hold_data_types() {
    let source = r#"import Foundation

// func run() {}
public struct Guide: Codable, Hashable {
    public let title: String
    var count: Int = 0
    private(set) var note: String = "func { }"
    static let limit = 3
    let tags: [String]
    let totals: [String: Int]
}

struct Box<
    Value: Codable
>: Codable where Value: Hashable {
    let value: Value
}

struct Pair:
    Equatable & Sendable
{
    let left: Int; let right: Int
}

enum Kind: String, CaseIterable, Codable {
    case guide = "g"
    case recipe, note
}

indirect enum Tree: Equatable {
    case leaf(Int)
    indirect case node(Tree, Tree)
}

typealias Guides = [Guide]
"#;
    assert_accepted(&["client/models/Guide.swift"], source);
}

#[test]
fn swift_models_refuse_behavior() {
    assert_refused(
        "client/models/Guide.swift",
        "A Swift models file declares only structs with stored properties, enums with cases, typealiases and the imports they need",
        &[
            (
                "struct Guide {\n    func describe() -> String { \"\" }\n}\n",
                "func",
            ),
            ("struct Guide {\n    var total: Int { 1 }\n}\n", "var total"),
            (
                "struct Guide {\n    var title = \"\" {\n        didSet {}\n    }\n}\n",
                "var title",
            ),
            ("struct Guide {\n    init() {}\n}\n", "init"),
            ("struct Guide {\n    lazy var count = 1\n}\n", "lazy"),
            ("struct Guide {\n    struct Inner {}\n}\n", "struct Inner"),
            (
                "struct Guide {\n    @Published var count = 1\n}\n",
                "@Published",
            ),
            ("struct Guide: View {}\n", "View"),
            ("enum Kind: Error {\n    case a\n}\n", "Error"),
            ("enum Kind {\n    case a\n    func run() {}\n}\n", "func"),
            ("func run() {}\n", "func"),
            ("let limit = 3\n", "let"),
            ("final class Guide {}\n", "final"),
            ("extension Guide {}\n", "extension"),
            ("protocol Named {}\n", "protocol"),
            ("@main\nstruct App {}\n", "@main"),
        ],
    );
}

#[test]
fn kotlin_models_hold_data_types() {
    let source = r#"import kotlinx.serialization.Serializable
import kotlinx.serialization.SerialName

// fun run() {}
@Serializable
data class Guide(
    val title: String,
    @SerialName("n") val count: Int = 0,
    var tags: List<String> = emptyList(),
) {
    val note: String = "fun { }"
    var total = 0
}

class Pair<Left, Right>(val left: Left, val right: Right) : java.io.Serializable

sealed class Shape
open class Circle(val radius: Double)
    : Shape()

enum class Kind { GUIDE, RECIPE }

enum class Level(val value: Int) {
    LOW(1),
    HIGH(2);
}

typealias Guides = List<Guide>
"#;
    assert_accepted(
        &["client/models/Guide.kt", "server/models/Guide.kt"],
        source,
    );
}

#[test]
fn kotlin_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.kt",
        "A Kotlin models file declares only classes with properties, enum classes with entries, typealiases and the imports they need",
        &[
            (
                "class Guide(val title: String) {\n    fun describe() = title\n}\n",
                "fun",
            ),
            (
                "data class Guide(val title: String) {\n    init { require(title.isNotEmpty()) }\n}\n",
                "init",
            ),
            (
                "class Guide {\n    constructor(title: String)\n}\n",
                "constructor",
            ),
            (
                "class Guide {\n    companion object { const val LIMIT = 3 }\n}\n",
                "companion",
            ),
            (
                "class Guide {\n    val size: Int get() = 1\n}\n",
                "val size",
            ),
            (
                "class Guide {\n    val size: Int\n        get() = 1\n}\n",
                "get",
            ),
            (
                "class Guide {\n    val text by lazy { \"\" }\n}\n",
                "val text",
            ),
            ("class Guide {\n    class Inner\n}\n", "class Inner"),
            ("class Guide(title: String)\n", "title"),
            ("fun helper() = 1\n", "fun"),
            ("val limit = 3\n", "val"),
            ("object Registry\n", "object"),
            ("interface Named\n", "interface"),
            ("package models\n", "package"),
            ("value class Id(val raw: String)\n", "value"),
            (
                "enum class Kind { A { override fun toString() = \"a\" } }\n",
                "A {",
            ),
            ("enum class Kind { A; fun describe() = 1 }\n", "fun"),
            (
                "class Guide @Needs constructor(val title: String)\n",
                "@Needs",
            ),
            (
                "@Lifetime\ndata class Guide(val title: String)\n",
                "@Lifetime",
            ),
        ],
    );
}

#[test]
fn java_models_hold_data_types() {
    let source = r#"import java.util.List;
import java.util.ArrayList;
import static java.util.Objects.requireNonNull;

// public void run() {}
public record Guide(String title, @JsonProperty("n") int count) {}

public record Pair(int left, int right) implements java.io.Serializable {
}

public enum Kind { GUIDE, RECIPE, }

public final class Section {
    public String title = "void run() { }";
    private int count;
    @JsonIgnore
    List<String> tags = new ArrayList<>();
    public static final int LIMIT = 3;
    int[] totals = {1, 2};
}
"#;
    assert_accepted(&["server/models/Guide.java"], source);
}

#[test]
fn java_checked_models_hold_only_constrained_data() {
    let source = "@Checked\nclass OrderInput {\n    @NotEmpty\n    @Length({\"min\":1,\"max\":80})\n    String name;\n    @Required\n    @Min(1)\n    @Max(9)\n    Object level;\n}\n";
    assert_accepted(
        &[
            "client/models/OrderInput.java",
            "server/models/OrderInput.java",
        ],
        source,
    );
}

#[test]
fn java_field_only_settings_are_data_but_unchecked_constraints_are_refused() {
    let source = include_str!(
        "../../tests/fixtures/behavior/authored/java/server/helpers/ApplicationValues.java"
    );
    assert_accepted(&["server/models/ApplicationValues.java"], source);
    let plain =
        super::JavaModelSchema::blanked(Path::new("server/models/ApplicationValues.java"), source)
            .expect("field settings stripped");
    assert!(!plain.contains("@Setting"));
    assert!(plain.contains("class ApplicationValues"));
    for marker in ["Required", "Min(1)", "Max(9)", "NotEmpty", "Length(3)"] {
        let source = format!("class Input {{ @{marker} String name; }}");
        assert_eq!(
            refusal("server/models/Input.java", &source).code.number(),
            1310
        );
    }
}

#[test]
fn java_model_schema_removal_preserves_literals_native_annotations_and_offsets() {
    let source = "@Checked\nclass OrderInput {\n    @Deprecated\n    @Length({\"min\":1,\"max\":80})\n    String name = \"@Required @Min(1)\";\n}\n";
    let compiled =
        super::JavaModelSchema::blanked(Path::new("client/models/OrderInput.java"), source)
            .expect("schema stripped");
    assert!(!compiled.contains("@Checked") && !compiled.contains("@Length"));
    assert!(compiled.contains("@Deprecated") && compiled.contains("\"@Required @Min(1)\""));
    assert_eq!(compiled.len(), source.len());
    assert_eq!(compiled.lines().count(), source.lines().count());
}

#[test]
fn java_checked_models_refuse_roles_behavior_and_misplaced_schema() {
    for source in [
        "@Service\n@Checked\nclass OrderInput { String name; }",
        "@Checked\nclass OrderInput { @Cached String name; }",
        "@Checked\nclass OrderInput { String read() { return name; } }",
        "@Checked\nclass OrderInput { OrderInput() {} }",
        "@Required\nclass OrderInput { String name; }",
        "@Checked\nclass OrderInput { @Checked String name; }",
        "@Checked\nrecord OrderInput(String name) {}",
    ] {
        assert_eq!(
            refusal("client/models/OrderInput.java", source)
                .code
                .number(),
            1310,
            "{source}"
        );
    }
}

#[test]
fn groovy_models_hold_data_types() {
    let source = r#"import java.time.LocalDate

record Guide(String title, int count) {}

enum Kind { GUIDE, RECIPE }

class Section {
    String title = "void run() { }"
    private int count
    List<String> tags = []
    LocalDate published
}
"#;
    assert_accepted(&["server/models/Guide.groovy"], source);
}

#[test]
fn groovy_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.groovy",
        "A Groovy models file declares only records without a body, enums with constants, classes with typed fields and the imports they need",
        &[
            (
                "record Guide(String title) { String describe() { title } }",
                "String describe",
            ),
            (
                "class Guide {\n    String describe() { '' }\n}",
                "String describe",
            ),
            ("class Guide {\n    class Inner {}\n}", "class Inner"),
            ("String helper() { 'x' }", "String helper"),
        ],
    );
}

#[test]
fn java_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.java",
        "A Java models file declares only records without a body, enums with constants, classes with fields and the imports they need",
        &[
            (
                "public record Guide(String title) { public String describe() { return title; } }",
                "public String",
            ),
            (
                "public class Guide { public String describe() { return \"\"; } }",
                "String describe",
            ),
            ("public class Guide { public Guide() {} }", "Guide()"),
            ("public class Guide { static { } }", "{ }"),
            ("public class Guide { { } }", "{ }"),
            ("public class Guide { class Inner {} }", "class Inner"),
            ("public enum Kind { A(1); }", "A(1)"),
            ("public enum Kind { A { }; }", "A {"),
            (
                "public enum Kind { A; public void run() {} }",
                "public void",
            ),
            ("package models;", "package"),
            ("public interface Named {}", "interface"),
            ("public @interface Marker {}", "@interface"),
            ("@OnStart\npublic class Guide {}", "@OnStart"),
            (
                "public class Guide { @Required String title; }",
                "@Required",
            ),
        ],
    );
}

#[test]
fn python_models_hold_data_types() {
    let source = r#"from __future__ import annotations

import dataclasses
from dataclasses import dataclass, field
from enum import Enum, unique
from typing import (
    NamedTuple,
    TypedDict,
)

# def helper(): pass


@dataclass(frozen=True)
class Guide:
    """A guide. def run(self): pass"""

    title: str
    count: int = 0
    tags: list[str] = field(
        default_factory=list,
    )


@dataclasses.dataclass
class Section:
    name: str


class Totals(TypedDict, total=False):
    guides: int


class Point(NamedTuple):
    x: int
    y: int


@unique
class Kind(str, Enum):
    GUIDE = "guide"
    RECIPE = "recipe"


class Empty(TypedDict): pass
"#;
    assert_accepted(&["server/models/guide.py"], source);
}

#[test]
fn python_models_refuse_behavior() {
    assert_refused(
        "server/models/guide.py",
        "A Python models file declares only dataclass, TypedDict, NamedTuple and Enum classes with fields, and the imports they need",
        &[
            (
                "@dataclass\nclass Guide:\n    title: str\n\n    def describe(self) -> str:\n        return self.title\n",
                "def",
            ),
            (
                "@dataclass\nclass Guide:\n    async def load(self): ...\n",
                "async",
            ),
            (
                "@dataclass\nclass Guide:\n    @property\n    def size(self): ...\n",
                "@property",
            ),
            (
                "@dataclass\nclass Guide:\n    class Meta:\n        pass\n",
                "class Meta",
            ),
            ("@dataclass\nclass Guide:\n    count = 0\n", "count"),
            ("@dataclass\nclass Guide:\n    try:\n        pass\n", "try"),
            ("class Guide:\n    title: str\n", "class"),
            ("class Guide(Base):\n    title: str\n", "class"),
            (
                "@total_ordering\n@dataclass\nclass Guide:\n    title: str\n",
                "@total_ordering",
            ),
            (
                "@unique\n@dataclass\nclass Guide:\n    title: str\n",
                "class",
            ),
            ("def helper():\n    pass\n", "def"),
            ("LIMIT = 3\n", "LIMIT"),
            ("type Guides = list[str]\n", "type"),
            ("if __name__ == '__main__':\n    pass\n", "if"),
            (
                "@dataclass\nclass Guide:\n    title: str\nprint(1)\n",
                "print",
            ),
            (
                "@dataclass\nclass Guide:\n    title: str; count: int\n",
                "; count",
            ),
            (
                "@OnStart\nclass Guide(TypedDict):\n    title: str\n",
                "@OnStart",
            ),
        ],
    );
}

#[test]
fn php_models_hold_data_types() {
    let source = r"<?php

use App\Models\Section;
use App\Models\{Kind, Level};

// function run() {}
# public function helper() {}
final class Guide
{
    public string $title = 'function run() { }';
    public ?int $count = null;
    public readonly array $tags;
    protected static int $limit = 0;
    private Section|null $section = null;

    public function __construct(
        public string $name,
        private readonly int $total = 0,
    ) {}
}

readonly class Point
{
    public function __construct(public int $x, public int $y) {}
}

enum Kind: string
{
    case Guide = 'guide';
    case Recipe = 'recipe';
}

enum Level { case Low; case High; }
";
    assert_accepted(&["server/models/Guide.php"], source);
}

#[test]
fn php_models_refuse_behavior() {
    assert_refused(
        "server/models/Guide.php",
        "A PHP models file declares only classes with typed properties, enums with cases and the use statements they need",
        &[
            (
                "<?php\nclass Guide { public function describe(): string { return ''; } }",
                "public function",
            ),
            (
                "<?php\nclass Guide { public function __construct(string $title) {} }",
                "string $title",
            ),
            (
                "<?php\nclass Guide { public function __construct(public string $title) { $this->x = 1; } }",
                "{ $this",
            ),
            (
                "<?php\nclass Guide { public string $title { get => ''; } }",
                "public string",
            ),
            ("<?php\nclass Guide { use Named; }", "use"),
            ("<?php\nclass Guide { const LIMIT = 3; }", "const"),
            ("<?php\nclass Guide { public $title; }", "public $title"),
            ("<?php\nnamespace App\\Models;", "namespace"),
            ("<?php\ndeclare(strict_types=1);", "declare"),
            ("<?php\n#[Immutable]\nclass Guide {}", "#[Immutable]"),
            ("<?php\nfunction helper() {}", "function"),
            ("<?php\ninterface Named {}", "interface"),
            ("<?php\ntrait Named {}", "trait"),
            ("<?php\nuse function App\\helper;", "use"),
            (
                "<?php\nenum Kind: string implements Labeled { case A = 'a'; }",
                "implements",
            ),
            (
                "<?php\nenum Kind { case A; public function label() {} }",
                "public function",
            ),
            ("<?php\nclass Guide {}\n?>\n", "?>"),
            ("<?php\necho 'x';", "echo"),
        ],
    );
}

#[test]
fn malformed_models_files_fail_without_panics() {
    let sources = [
        "struct Grüße { ä: u8 }\n}}}",
        "class A(\n",
        "class A { val x: Int get",
        "@\n#[\n<\n$\n\\\n",
        "enum E { é(1), }\nstruct S<T: Codable\n",
        "<?php\nclass A { public function __construct(public int $x {\n",
        "class A:\n  x: int\n    y: int\n\tz\n",
        "record R(int X",
    ];
    for extension in ["rs", "cs", "dart", "kt", "swift", "java", "py", "php", "go"] {
        for realm in ["client", "server"] {
            let path = format!("{realm}/models/Guide.{extension}");
            for source in sources {
                let _ = validate(Path::new(&path), source);
            }
        }
    }
}

#[test]
fn a_byte_order_mark_does_not_indent_the_first_line() {
    let source =
        "\u{feff}from dataclasses import dataclass\n\n\n@dataclass\nclass Guide:\n    title: str\n";
    assert_accepted(&["server/models/Guide.py"], source);
    let refused = "\u{feff}def helper():\n    pass\n";
    let diagnostic = refusal("server/models/Guide.py", refused);
    assert!(refused[refused_at(&diagnostic)..].starts_with("def"));
}

#[test]
fn declarative_model_metadata_keeps_configuration_and_validation_data() {
    let cases = [
        (
            "rs",
            "ProductInput",
            include_str!(
                "../../tests/fixtures/behavior/authored/rs/server/helpers/ProductInput.rs"
            ),
        ),
        (
            "cs",
            "AnnotatedInput",
            include_str!(
                "../../tests/fixtures/behavior/authored/cs/server/helpers/AnnotatedInput.cs"
            ),
        ),
        (
            "java",
            "Settings",
            include_str!(
                "../../tests/fixtures/behavior/authored/java/server/helpers/Settings.java"
            ),
        ),
        (
            "groovy",
            "Input",
            include_str!(
                "../../tests/fixtures/behavior/authored/groovy/server/helpers/Input.groovy"
            ),
        ),
        (
            "kt",
            "Settings",
            include_str!("../../tests/fixtures/behavior/authored/kt/server/helpers/Settings.kt"),
        ),
        (
            "go",
            "ProductInput",
            include_str!(
                "../../tests/fixtures/behavior/authored/go/server/helpers/ProductInput.go"
            ),
        ),
        (
            "py",
            "ApplicationValues",
            include_str!(
                "../../tests/fixtures/behavior/authored/py/server/helpers/ApplicationValues.py"
            ),
        ),
        (
            "php",
            "ProductInput",
            include_str!(
                "../../tests/fixtures/behavior/authored/php/server/helpers/ProductInput.php"
            ),
        ),
        (
            "rb",
            "Settings",
            include_str!("../../tests/fixtures/behavior/authored/rb/server/helpers/Settings.rb"),
        ),
    ];
    crate::annotation_layout::validate(Path::new("server/models/OrderInput.fs"), "[<Checked>]\ntype OrderInput() =\n    [<Required>]\n    member val name : string = \"\" with get, set\n").expect("F# schema data");
    let failures: Vec<_> = cases
        .into_iter()
        .filter_map(|(extension, name, source)| {
            let path = format!("server/models/{name}.{extension}");
            crate::annotation_layout::validate(Path::new(&path), source)
                .err()
                .map(|error| format!("{path}: {error}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn schema_models_still_refuse_behavior_and_arbitrary_constructors() {
    for source in [
        "[Checked]\npublic class OrderInput { public string name; public OrderInput() {} }",
        "[Checked]\npublic class OrderInput { public string name; private OrderInput() { name = \"x\"; } }",
        "[Checked]\npublic class OrderInput { public string read() { return \"x\"; } }",
        "[Checked]\npublic class OrderInput { private OrderInput() { throw new System.Exception($\"{Danger()}\"); } }",
        "[Service]\n[Checked]\npublic class OrderInput { public string name; }",
    ] {
        assert!(
            crate::annotation_layout::validate(Path::new("server/models/OrderInput.cs"), source)
                .is_err(),
            "{source}"
        );
    }
    assert!(crate::annotation_layout::validate(Path::new("server/models/OrderInput.py"), "@checked\nclass OrderInput:\n    name: str = ''\n    def read(self):\n        return self.name\n").is_err());
}
