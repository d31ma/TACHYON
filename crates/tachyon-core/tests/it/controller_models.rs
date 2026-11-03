#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct BasketProject {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl BasketProject {
    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        files: &[
            (
                "server/models/Line.js",
                r"/**
 * @typedef {Object} Line
 * @property {string} name
 * @property {number} count
 */
export {}
",
            ),
            (
                "server/models/Basket.js",
                r"/**
 * @typedef {Object} Basket
 * @property {string} owner
 * @property {Line[]} lines
 * @property {Line=} gift
 */
export {}
",
            ),
            (
                "server/controllers/BasketsController.js",
                r#"@RequestRoute("/baskets")
@Public
class BasketsController {
  /**
   * @param {Basket} basket
   * @returns {Basket}
   */
  @PostRoute
  postBasket(@FromBody basket) {
    const first = basket.lines[0]
    return { owner: basket.owner, lines: basket.lines, gift: { name: first.name, count: first.count + 1 } }
  }

  /**
   * @param {Line[]} lines
   * @returns {Line[]}
   */
  @PutRoute
  putLines(@FromBody lines) {
    return [{ name: lines[1].name, count: lines[0].count }]
  }

  /**
   * @param {string} owner
   * @returns {Basket}
   */
  @GetRoute("/{owner}")
  getBasket(@FromPath owner) {
    return { owner, lines: [], gift: null }
  }
}
"#,
            ),
        ],
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "bun",
        files: &[
            (
                "server/models/Line.ts",
                "export interface Line {\n  name: string\n  count: number\n}\n",
            ),
            (
                "server/models/Basket.ts",
                "import type { Line } from './Line.ts'\n\nexport interface Basket {\n  owner: string\n  lines: Line[]\n  gift?: Line | null\n}\n",
            ),
            (
                "server/controllers/BasketsController.ts",
                r#"import type { Basket } from '/models/Basket.ts'
import type { Line } from '/models/Line.ts'

@RequestRoute("/baskets")
@Public
class BasketsController {
  @PostRoute
  postBasket(@FromBody basket: Basket): Basket {
    const first = basket.lines[0]
    return { owner: basket.owner, lines: basket.lines, gift: { name: first.name, count: first.count + 1 } }
  }

  @PutRoute
  putLines(@FromBody lines: Line[]): Line[] {
    return [{ name: lines[1].name, count: lines[0].count }]
  }

  @GetRoute("/{owner}")
  getBasket(@FromPath owner: string): Basket {
    return { owner, lines: [], gift: null }
  }
}
"#,
            ),
        ],
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Line.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Line:\n    name: str\n    count: int\n",
            ),
            (
                "server/models/Basket.py",
                "from dataclasses import dataclass\nfrom typing import Optional\n\nfrom models import Line\n\n\n@dataclass\nclass Basket:\n    owner: str\n    lines: list[Line]\n    gift: Optional[Line] = None\n",
            ),
            (
                "server/controllers/BasketsController.py",
                r#"from typing import Annotated
from models import Basket, Line


@request_route("/baskets")
@public
class BasketsController:
    @post_route
    def post_basket(self, basket: Annotated[Basket, from_body]) -> Basket:
        first = basket.lines[0]
        return Basket(basket.owner, basket.lines, Line(first.name, first.count + 1))

    @put_route
    def put_lines(self, lines: Annotated[list[Line], from_body]) -> list[Line]:
        return [Line(lines[1].name, lines[0].count)]

    @get_route("/{owner}")
    def get_basket(self, owner: Annotated[str, from_path]) -> Basket:
        return Basket(owner, [], None)
"#,
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        files: &[
            (
                "server/models/Line.rb",
                "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] count\n#   @return [Integer]\nLine = Data.define(:name, :count)\n",
            ),
            (
                "server/models/Basket.rb",
                "# @!attribute [r] owner\n#   @return [String]\n# @!attribute [r] lines\n#   @return [Array<Line>]\n# @!attribute [r] gift\n#   @return [Line, nil]\nBasket = Data.define(:owner, :lines, :gift)\n",
            ),
            (
                "server/controllers/BasketsController.rb",
                r#"#[RequestRoute("/baskets")]
#[Public]
class BasketsController
  # @param basket [Basket]
  # @return [Basket]
  #[PostRoute]
  def post_basket(
    #[FromBody]
    basket
  )
    first = basket.lines[0]
    Basket.new(owner: basket.owner, lines: basket.lines, gift: Line.new(name: first.name, count: first.count + 1))
  end

  # @param lines [Array<Line>]
  # @return [Array<Line>]
  #[PutRoute]
  def put_lines(
    #[FromBody]
    lines
  )
    [Line.new(name: lines[1].name, count: lines[0].count)]
  end

  # @param owner [String]
  # @return [Basket]
  #[GetRoute("/{owner}")]
  def get_basket(
    #[FromPath]
    owner
  )
    Basket.new(owner: owner, lines: [], gift: nil)
  end
end
"#,
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/models/Line.php",
                "<?php\nclass Line {\n    public function __construct(public string $name, public int $count) {}\n}\n",
            ),
            (
                "server/models/Basket.php",
                "<?php\nclass Basket {\n    public function __construct(\n        public string $owner,\n        /** @var list<Line> */\n        public array $lines,\n        public ?Line $gift = null,\n    ) {}\n}\n",
            ),
            (
                "server/controllers/BasketsController.php",
                r#"<?php
use Models\Basket;
use Models\Line;

#[RequestRoute("/baskets")]
#[Public]
class BasketsController {
    #[PostRoute]
    public function postBasket(#[FromBody] Basket $basket): Basket {
        $first = $basket->lines[0];
        return new Basket($basket->owner, $basket->lines, new Line($first->name, $first->count + 1));
    }

    /**
     * @param list<Line> $lines
     * @return list<Line>
     */
    #[PutRoute]
    public function putLines(#[FromBody] array $lines): array {
        return [new Line($lines[1]->name, $lines[0]->count)];
    }

    #[GetRoute("/{owner}")]
    public function getBasket(#[FromPath] string $owner): Basket {
        return new Basket($owner, [], null);
    }
}
"#,
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/models/Line.java",
                "record Line(String name, long count) {}\n",
            ),
            (
                "server/models/Basket.java",
                "import java.util.List;\nimport java.util.Optional;\n\nrecord Basket(String owner, List<Line> lines, Optional<Line> gift) {}\n",
            ),
            (
                "server/controllers/BasketsController.java",
                r#"import java.util.List;
import java.util.Optional;
import models.Basket;
import models.Line;

@RequestRoute("/baskets")
@Public
class BasketsController {
    @PostRoute
    Basket postBasket(@FromBody Basket basket) {
        Line first = basket.lines().get(0);
        return new Basket(basket.owner(), basket.lines(), Optional.of(new Line(first.name(), first.count() + 1)));
    }

    @PutRoute
    List<Line> putLines(@FromBody List<Line> lines) {
        return List.of(new Line(lines.get(1).name(), lines.get(0).count()));
    }

    @GetRoute("/{owner}")
    Basket getBasket(@FromPath String owner) {
        return new Basket(owner, List.of(), Optional.empty());
    }
}
"#,
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "server/models/Line.kt",
                "data class Line(val name: String, val count: Long)\n",
            ),
            (
                "server/models/Basket.kt",
                "data class Basket(val owner: String, val lines: List<Line>, val gift: Line? = null)\n",
            ),
            (
                "server/controllers/BasketsController.kt",
                r#"import models.Basket
import models.Line

@RequestRoute("/baskets")
@Public
class BasketsController {
    @PostRoute
    fun postBasket(@FromBody basket: Basket): Basket {
        val first = basket.lines[0]
        return basket.copy(gift = Line(first.name, first.count + 1))
    }

    @PutRoute
    fun putLines(@FromBody lines: List<Line>): List<Line> = listOf(Line(lines[1].name, lines[0].count))

    @GetRoute("/{owner}")
    fun getBasket(@FromPath owner: String): Basket = Basket(owner, emptyList(), null)
}
"#,
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Line.cs",
                "public record Line(string Name, long Count);\n",
            ),
            (
                "server/models/Basket.cs",
                "using System.Collections.Generic;\n\npublic record Basket(string Owner, List<Line> Lines, Line? Gift);\n",
            ),
            (
                "server/controllers/BasketsController.cs",
                r#"using System.Collections.Generic;
using Models;

[RequestRoute("/baskets")]
[Public]
class BasketsController {
    [PostRoute]
    public Basket PostBasket([FromBody] Basket basket) {
        Line first = basket.Lines[0];
        return basket with { Gift = new Line(first.Name, first.Count + 1) };
    }

    [PutRoute]
    public List<Line> PutLines([FromBody] List<Line> lines) {
        return new List<Line> { new Line(lines[1].Name, lines[0].Count) };
    }

    [GetRoute("/{owner}")]
    public Basket GetBasket([FromPath] string owner) {
        return new Basket(owner, new List<Line>(), null);
    }
}
"#,
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        files: &[
            (
                "server/models/Line.rs",
                "pub struct Line {\n    pub name: String,\n    pub count: i64,\n}\n",
            ),
            (
                "server/models/Basket.rs",
                "use crate::models::Line;\n\npub struct Basket {\n    pub owner: String,\n    pub lines: Vec<Line>,\n    pub gift: Option<Line>,\n}\n",
            ),
            (
                "server/controllers/BasketsController.rs",
                r#"use crate::models::{Basket, Line};

#[RequestRoute("/baskets")]
#[Public]
struct BasketsController {}
impl BasketsController {
    fn new() -> Self { Self {} }

    #[PostRoute]
    fn post_basket(&self, #[FromBody] basket: Basket) -> Basket {
        let gift = Line { name: basket.lines[0].name.clone(), count: basket.lines[0].count + 1 };
        Basket { owner: basket.owner, lines: basket.lines, gift: Some(gift) }
    }

    #[PutRoute]
    fn put_lines(&self, #[FromBody] lines: Vec<Line>) -> Vec<Line> {
        vec![Line { name: lines[1].name.clone(), count: lines[0].count }]
    }

    #[GetRoute("/{owner}")]
    fn get_basket(&self, #[FromPath] owner: String) -> Basket {
        Basket { owner, lines: Vec::new(), gift: None }
    }
}
"#,
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Line.dart",
                "class Line {\n  final String name;\n  final int count;\n\n  const Line(this.name, this.count);\n}\n",
            ),
            (
                "server/models/Basket.dart",
                "import 'Line.dart';\n\nclass Basket {\n  final String owner;\n  final List<Line> lines;\n  final Line? gift;\n\n  const Basket(this.owner, this.lines, this.gift);\n}\n",
            ),
            (
                "server/controllers/BasketsController.dart",
                r#"import 'models/Basket.dart';
import 'models/Line.dart';

@RequestRoute("/baskets")
@Public
class BasketsController {
  @PostRoute
  Basket postBasket(@FromBody Basket basket) {
    final first = basket.lines[0];
    return Basket(basket.owner, basket.lines, Line(first.name, first.count + 1));
  }

  @PutRoute
  List<Line> putLines(@FromBody List<Line> lines) {
    return [Line(lines[1].name, lines[0].count)];
  }

  @GetRoute("/{owner}")
  Basket getBasket(@FromPath String owner) {
    return Basket(owner, [], null);
  }
}
"#,
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        files: &[
            (
                "server/models/Line.go",
                "type Line struct {\n\tName  string `json:\"name\"`\n\tCount int64  `json:\"count\"`\n}\n",
            ),
            (
                "server/models/Basket.go",
                "type Basket struct {\n\tOwner string `json:\"owner\"`\n\tLines []Line `json:\"lines\"`\n\tGift  *Line  `json:\"gift\"`\n}\n",
            ),
            (
                "server/controllers/BasketsController.go",
                r#"// @RequestRoute("/baskets")
// @Public
type BasketsController struct {
}

// @PostRoute
func (controller *BasketsController) PostBasket(
	// @FromBody
	basket Basket,
) Basket {
	first := basket.Lines[0]
	basket.Gift = &Line{Name: first.Name, Count: first.Count + 1}
	return basket
}

// @PutRoute
func (controller *BasketsController) PutLines(
	// @FromBody
	lines []Line,
) []Line {
	return []Line{{Name: lines[1].Name, Count: lines[0].Count}}
}

// @GetRoute("/{owner}")
func (controller *BasketsController) GetBasket(
	// @FromPath
	owner string,
) Basket {
	return Basket{Owner: owner, Lines: []Line{}}
}
"#,
            ),
        ],
    };
}

struct BookProject;

impl BookProject {
    const JAVA: BasketProject = BasketProject {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/models/Item.java",
                "public class Item {\n    public String name;\n}\n",
            ),
            (
                "server/models/Book.java",
                "public class Book extends Item {\n    public long pages;\n}\n",
            ),
            (
                "server/controllers/BooksController.java",
                "import models.Book;\n\n@RequestRoute(\"/books\")\n@Public\nclass BooksController {\n    @PostRoute\n    Book postBook(@FromBody Book book) {\n        book.pages = book.pages + 1;\n        return book;\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: BasketProject = BasketProject {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Item.cs",
                "public class Item {\n    public string Name { get; set; } = \"\";\n}\n",
            ),
            (
                "server/models/Book.cs",
                "public class Book : Item {\n    public long Pages { get; set; }\n}\n",
            ),
            (
                "server/controllers/BooksController.cs",
                "using Models;\n\n[RequestRoute(\"/books\")]\n[Public]\nclass BooksController {\n    [PostRoute]\n    public Book PostBook([FromBody] Book book) {\n        book.Pages += 1;\n        return book;\n    }\n}\n",
            ),
        ],
    };

    const PYTHON: BasketProject = BasketProject {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Item.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Item:\n    name: str\n",
            ),
            (
                "server/models/Book.py",
                "from dataclasses import dataclass\n\nfrom models import Item\n\n\n@dataclass\nclass Book(Item):\n    pages: int\n",
            ),
            (
                "server/controllers/BooksController.py",
                "from typing import Annotated\nfrom models import Book\n\n\n@request_route(\"/books\")\n@public\nclass BooksController:\n    @post_route\n    def post_book(self, book: Annotated[Book, from_body]) -> Book:\n        return Book(book.name, book.pages + 1)\n",
            ),
        ],
    };

    const PHP: BasketProject = BasketProject {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/models/Item.php",
                "<?php\nclass Item {\n    public string $name;\n}\n",
            ),
            (
                "server/models/Book.php",
                "<?php\nclass Book extends Item {\n    public int $pages;\n}\n",
            ),
            (
                "server/controllers/BooksController.php",
                "<?php\nuse Models\\Book;\n\n#[RequestRoute(\"/books\")]\n#[Public]\nclass BooksController {\n    #[PostRoute]\n    public function postBook(#[FromBody] Book $book): Book {\n        $book->pages = $book->pages + 1;\n        return $book;\n    }\n}\n",
            ),
        ],
    };

    const DART: BasketProject = BasketProject {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Item.dart",
                "class Item {\n  final String name;\n\n  const Item(this.name);\n}\n",
            ),
            (
                "server/models/Book.dart",
                "import 'Item.dart';\n\nclass Book extends Item {\n  final int pages;\n\n  const Book(super.name, this.pages);\n}\n",
            ),
            (
                "server/controllers/BooksController.dart",
                "import 'models/Book.dart';\n\n@RequestRoute(\"/books\")\n@Public\nclass BooksController {\n  @PostRoute\n  Book postBook(@FromBody Book book) => Book(book.name, book.pages + 1);\n}\n",
            ),
        ],
    };

    const GO: BasketProject = BasketProject {
        extension: "go",
        program: "go",
        files: &[
            (
                "server/models/Item.go",
                "type Item struct {\n\tName string `json:\"name\"`\n}\n",
            ),
            (
                "server/models/Book.go",
                "type Book struct {\n\tItem\n\tPages int64 `json:\"pages\"`\n}\n",
            ),
            (
                "server/controllers/BooksController.go",
                "// @RequestRoute(\"/books\")\n// @Public\ntype BooksController struct {\n}\n\n// @PostRoute\nfunc (controller *BooksController) PostBook(\n\t// @FromBody\n\tbook Book,\n) Book {\n\tbook.Pages = book.Pages + 1\n\treturn book\n}\n",
            ),
        ],
    };
}

struct TickProject;

impl TickProject {
    const CSHARP: BasketProject = BasketProject {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Tick.cs",
                "public record Tick(string Sku, int Level);\n",
            ),
            (
                "server/controllers/TicksController.cs",
                "using System.Collections.Generic;\nusing Models;\n\n[RequestRoute(\"/ticks\")]\n[Public]\nclass TicksController {\n    [StreamRoute]\n    public IAsyncEnumerable<Tick> StreamTicks() {\n        return Countdown(2);\n    }\n\n    private static async IAsyncEnumerable<Tick> Countdown(int start) {\n        for (var level = start; level > 0; level--) {\n            await System.Threading.Tasks.Task.Yield();\n            yield return new Tick(\"ANVIL-1\", level);\n        }\n    }\n}\n",
            ),
        ],
    };

    const FSHARP: BasketProject = BasketProject {
        extension: "fs",
        program: "dotnet",
        files: &[
            (
                "server/models/Tick.fs",
                "type Tick = { Sku: string; Level: int }\n",
            ),
            (
                "server/controllers/TicksController.fs",
                "open Models\n\n[<RequestRoute(\"/ticks\")>]\n[<Public>]\ntype TicksController() =\n    [<StreamRoute>]\n    member this.StreamTicks() : seq<Tick> =\n        TicksController.Countdown(2)\n\n    static member private Countdown(start: int) : seq<Tick> =\n        seq {\n            for level in start .. -1 .. 1 do\n                yield { Sku = \"ANVIL-1\"; Level = level }\n        }\n",
            ),
        ],
    };

    const DART: BasketProject = BasketProject {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Tick.dart",
                "class Tick {\n  final String sku;\n  final int level;\n\n  const Tick(this.sku, this.level);\n}\n",
            ),
            (
                "server/controllers/TicksController.dart",
                "import 'models/Tick.dart';\n\n@RequestRoute(\"/ticks\")\n@Public\nclass TicksController {\n  @StreamRoute\n  Stream<Tick> streamTicks() async* {\n    yield* _countdown(2);\n  }\n\n  Stream<Tick> _countdown(int start) async* {\n    for (var level = start; level > 0; level--) {\n      yield Tick('ANVIL-1', level);\n    }\n  }\n}\n",
            ),
        ],
    };

    const PYTHON: BasketProject = BasketProject {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Tick.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Tick:\n    sku: str\n    level: int\n",
            ),
            (
                "server/controllers/TicksController.py",
                "from models import Tick\n\n\n@request_route(\"/ticks\")\n@public\nclass TicksController:\n    @stream_route\n    def stream_ticks(self):\n        yield from self._countdown(2)\n\n    def _countdown(self, start):\n        for level in range(start, 0, -1):\n            yield Tick(\"ANVIL-1\", level)\n",
            ),
        ],
    };
}

struct BasketCheck;

impl BasketCheck {
    fn available(program: &str) -> bool {
        let flag = match program {
            "kotlinc" => "-version",
            "go" => "version",
            _ => "--version",
        };
        ProgramProbe::answers(program, flag)
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::write(path, source).expect("source");
    }

    async fn call(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        route: &str,
        method: HttpMethod,
        arguments: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        let mut request = HandlerRequest::route("baskets", route, method);
        request.arguments = Some(arguments);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from("{}"),
        });
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("invocation");
        assert!(response.error.is_none(), "{:?}", response.error);
        let body = response.body.expect("body").data;
        serde_json::from_str(&body).unwrap_or_else(|_| panic!("JSON answer: {body}"))
    }

    fn skipped(project: &BasketProject) -> bool {
        if Self::available(project.program) {
            return false;
        }
        assert!(
            std::env::var_os("CI").is_none(),
            "{} is required in CI",
            project.program
        );
        eprintln!(
            "skipping {}: {} is unavailable",
            project.extension, project.program
        );
        true
    }

    fn start(
        root: &Path,
        project: &BasketProject,
        controller: &str,
    ) -> (HandlerSupervisor, HandlerSource) {
        for (relative, source) in project.files {
            Self::write(root, relative, source);
        }
        let entry = format!("server/controllers/{controller}.{}", project.extension);
        let source = HandlerSource::discover(root, &entry)
            .unwrap_or_else(|failure| panic!("{}: {failure}", project.extension));
        let mut options = HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(2),
            ..HandlerSupervisorOptions::default()
        };
        if project.extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        (supervisor, source)
    }

    async fn assert_books(project: &BasketProject) {
        if Self::skipped(project) {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let (supervisor, source) = Self::start(root.path(), project, "BooksController");
        let book = Self::call(
            &supervisor,
            &source,
            "/books",
            HttpMethod::Post,
            vec![serde_json::json!({"name": "Dune", "pages": 3})],
        )
        .await;
        assert_eq!(
            book,
            serde_json::json!({"name": "Dune", "pages": 4}),
            "{}",
            project.extension
        );
    }

    async fn assert_ticks(project: &BasketProject) {
        if Self::skipped(project) {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let (supervisor, source) = Self::start(root.path(), project, "TicksController");
        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        supervisor
            .invoke_streaming(
                &source,
                &HandlerRequest::route("ticks", "/ticks", HttpMethod::Get),
                sender,
            )
            .await
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", project.extension));
        for level in [2, 1] {
            let frame = receiver.recv().await.expect("tick frame");
            let tick: serde_json::Value = serde_json::from_str(&frame.data)
                .unwrap_or_else(|_| panic!("JSON tick: {}", frame.data));
            assert_eq!(
                tick,
                serde_json::json!({"sku": "ANVIL-1", "level": level}),
                "{}",
                project.extension
            );
        }
    }

    async fn assert_baskets(project: &BasketProject) {
        if Self::skipped(project) {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let (supervisor, source) = Self::start(root.path(), project, "BasketsController");
        let tea = serde_json::json!({"name": "tea", "count": 2});
        let jam = serde_json::json!({"name": "jam", "count": 5});
        let posted = Self::call(
            &supervisor,
            &source,
            "/baskets",
            HttpMethod::Post,
            vec![serde_json::json!({"owner": "ann", "lines": [tea, jam]})],
        )
        .await;
        assert_eq!(
            posted,
            serde_json::json!({
                "owner": "ann",
                "lines": [tea, jam],
                "gift": {"name": "tea", "count": 3}
            }),
            "{}",
            project.extension
        );
        let put = Self::call(
            &supervisor,
            &source,
            "/baskets",
            HttpMethod::Put,
            vec![serde_json::json!([tea, jam])],
        )
        .await;
        assert_eq!(
            put,
            serde_json::json!([{"name": "jam", "count": 2}]),
            "{}",
            project.extension
        );
        let empty = Self::call(
            &supervisor,
            &source,
            "/baskets/_owner",
            HttpMethod::Get,
            vec![serde_json::json!("bob")],
        )
        .await;
        assert_eq!(
            empty,
            serde_json::json!({"owner": "bob", "lines": [], "gift": null}),
            "{}",
            project.extension
        );
    }
}

#[tokio::test]
async fn a_javascript_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::JAVASCRIPT).await;
}

#[tokio::test]
async fn a_typescript_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::TYPESCRIPT).await;
}

#[tokio::test]
async fn a_python_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::PYTHON).await;
}

#[tokio::test]
async fn a_ruby_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::RUBY).await;
}

#[tokio::test]
async fn a_php_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::PHP).await;
}

#[tokio::test]
async fn a_java_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::JAVA).await;
}

#[tokio::test]
async fn a_kotlin_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::KOTLIN).await;
}

#[tokio::test]
async fn a_csharp_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::CSHARP).await;
}

#[tokio::test]
async fn a_rust_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::RUST).await;
}

#[tokio::test]
async fn a_dart_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::DART).await;
}

#[tokio::test]
async fn a_go_controller_takes_and_returns_models() {
    BasketCheck::assert_baskets(&BasketProject::GO).await;
}

#[tokio::test]
async fn a_java_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::JAVA).await;
}

#[tokio::test]
async fn a_csharp_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::CSHARP).await;
}

#[tokio::test]
async fn a_python_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::PYTHON).await;
}

#[tokio::test]
async fn a_php_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::PHP).await;
}

#[tokio::test]
async fn a_dart_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::DART).await;
}

#[tokio::test]
async fn a_go_model_keeps_its_parent_fields() {
    BasketCheck::assert_books(&BookProject::GO).await;
}

#[test]
fn a_controller_return_type_that_is_not_json_is_refused_at_build_time() {
    let cases = [
        (
            "java",
            "@RequestRoute(\"/files\")\n@Public\nclass FilesController {\n    @GetRoute\n    java.io.InputStream getFile() {\n        return null;\n    }\n}\n",
            "'InputStream' is not a model",
        ),
        (
            "go",
            "// @RequestRoute(\"/files\")\n// @Public\ntype FilesController struct {\n}\n\n// @GetRoute\nfunc (controller *FilesController) GetFile() chan int {\n\treturn nil\n}\n",
            "'chan int' is not supported here",
        ),
        (
            "rs",
            "#[RequestRoute(\"/files\")]\n#[Public]\nstruct FilesController {}\nimpl FilesController {\n    fn new() -> Self { Self {} }\n\n    #[GetRoute]\n    fn get_file(&self) -> std::fs::File {\n        todo!()\n    }\n}\n",
            "'File' is not a model",
        ),
    ];
    for (extension, source, expected) in cases {
        let root = tempfile::tempdir().expect("project");
        let entry = format!("server/controllers/FilesController.{extension}");
        BasketCheck::write(root.path(), &entry, source);
        let Err(failure) = HandlerSource::discover(root.path(), &entry) else {
            panic!("{extension}: a return type that is not JSON must be refused");
        };
        let message = failure.to_string();
        assert!(
            message.contains("The return type of"),
            "{extension}: {message}"
        );
        assert!(message.contains(expected), "{extension}: {message}");
    }
}

#[tokio::test]
async fn a_csharp_stream_sends_models() {
    BasketCheck::assert_ticks(&TickProject::CSHARP).await;
}

#[tokio::test]
async fn an_fsharp_stream_sends_models() {
    BasketCheck::assert_ticks(&TickProject::FSHARP).await;
}

#[tokio::test]
async fn a_dart_stream_sends_models() {
    BasketCheck::assert_ticks(&TickProject::DART).await;
}

#[tokio::test]
async fn a_python_stream_sends_models() {
    BasketCheck::assert_ticks(&TickProject::PYTHON).await;
}
