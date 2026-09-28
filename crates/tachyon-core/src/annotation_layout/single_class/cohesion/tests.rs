#![allow(clippy::expect_used)]

use super::CohesionCheck;
use super::RuleSwitch;
use std::path::Path;

struct Language {
    file: &'static str,
    template: &'static str,
    accepted: &'static str,
    refused: &'static [(&'static str, &'static str)],
    crowded_helper: &'static str,
    chained_helpers: &'static str,
}

fn messages(file: &str, source: &str) -> Vec<String> {
    CohesionCheck::problems(Path::new(file), source)
        .expect("readable source")
        .into_iter()
        .map(|problem| {
            assert_eq!(problem.code.number(), 1314);
            problem.message
        })
        .collect()
}

fn service(language: &Language, body: &str) -> String {
    language.template.replace("{body}", body)
}

fn check_language(language: &Language) {
    let accepted = service(language, language.accepted);
    assert_eq!(
        messages(language.file, &accepted),
        Vec::<String>::new(),
        "{accepted}"
    );
    for (body, word) in language.refused {
        let source = service(language, body);
        let found = messages(language.file, &source);
        let expected = format!("`{word}`");
        assert!(
            found
                .iter()
                .any(|message| message.starts_with("Public method") && message.contains(&expected)),
            "{} should refuse {word} in:\n{source}\nfound {found:?}",
            language.file
        );
    }
    let crowded = messages(language.file, language.crowded_helper);
    assert!(
        crowded
            .iter()
            .any(|message| message.starts_with("Private method") && message.contains("holds 2")),
        "{} crowded helper: {crowded:?}",
        language.file
    );
    let chained = messages(language.file, language.chained_helpers);
    assert!(
        chained.iter().any(|message| message.contains("calls `")),
        "{} chained helpers: {chained:?}",
        language.file
    );
}

#[test]
fn javascript_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.js",
        template: "@Service\nexport default class ItemService {\n  @Needs\n  store\n  count = 0\n  load(items) {\n    {body}\n  }\n  #pick(items) {\n    if (items.length > 0) return items[0]\n    return null\n  }\n}\n",
        accepted: "const found = this.store.read(items)\n    const label = \"if && || ? :\"\n    const text = `${items} for`\n    const pattern = /if|for/\n    // if (found) for\n    const ids = found.map((item) => item.id)\n    return this.#pick(found?.at(0))",
        refused: &[
            ("if (items) return 1", "if"),
            ("return items ? 1 : 2", "? :"),
            ("return items && items[0]", "&&"),
            ("return items || []", "||"),
            ("return items ?? []", "??"),
            ("for (const item of items) this.store.save(item)", "for"),
            ("while (items.length) items.pop()", "while"),
            ("switch (items.length) { default: return 1 }", "switch"),
            ("try { return 1 } catch (error) { return 2 }", "try"),
            (
                "return items.filter((item) => item.ready && item.shown)",
                "filter",
            ),
            (
                "items.forEach(function (item) { if (item) this.store.save(item) })",
                "forEach",
            ),
            (
                "setTimeout(() => { if (items) this.store.save(items) }, 0)",
                "if",
            ),
        ],
        crowded_helper: "export default class ItemService {\n  #pick(items) {\n    if (items) return 1\n    for (const item of items) this.save(item)\n  }\n}\n",
        chained_helpers: "export default class ItemService {\n  #pick(items) {\n    if (items) return this.#other(items)\n  }\n  #other(items) {\n    return items ? 1 : 2\n  }\n}\n",
    });
}

#[test]
fn typescript_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.ts",
        template: "@Service\nexport default class ItemService {\n  @Needs\n  store!: Store\n  load(items: Item[], extra?: number): Item | null {\n    {body}\n  }\n  private pick(items: Item[]): Item | null {\n    if (items.length > 0) return items[0]\n    return null\n  }\n}\n",
        accepted: "const found: Item[] = this.store.read(items)\n    const size = found?.length\n    return this.pick(found)",
        refused: &[
            ("if (items) return null", "if"),
            ("return items.length ? items[0] : null", "? :"),
            ("return items[0] ?? null", "??"),
            ("for (const item of items) this.store.save(item)", "for"),
            (
                "return items.find((item) => item.id === 1 || item.id === 2) ?? null",
                "||",
            ),
        ],
        crowded_helper: "export default class ItemService {\n  private pick(items: Item[]) {\n    if (items) return 1\n    return items.length > 0 && items[0]\n  }\n}\n",
        chained_helpers: "export default class ItemService {\n  private pick(items: Item[]) {\n    if (items) return this.other(items)\n  }\n  private other(items: Item[]) {\n    while (items.length) items.pop()\n    return items ? 1 : 2\n  }\n}\n",
    });
}

#[test]
fn python_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/services/ItemService.py",
        template: "@Service\nclass ItemService:\n    store: Store = Needs()\n\n    def load(self, items):\n        {body}\n\n    def _pick(self, items):\n        if items:\n            return items[0]\n        return None\n",
        accepted: "found = self.store.read(items)\n        label = \"if and or\"\n        # if found for\n        ids = [item.id, found]\n        return self._pick(found)",
        refused: &[
            ("return 1 if items else 2", "if"),
            ("return items and items[0]", "and"),
            ("return items or []", "or"),
            ("return [item.id for item in items]", "for"),
            (
                "return list(map(lambda item: item.id if item else 0, items))",
                "map",
            ),
            (
                "try:\n            return 1\n        except ValueError:\n            return 2",
                "try",
            ),
            ("while items:\n            items.pop()", "while"),
            (
                "match items:\n            case []:\n                return 1",
                "match",
            ),
        ],
        crowded_helper: "class ItemService:\n    def _pick(self, items):\n        if items:\n            return 1\n        for item in items:\n            self.save(item)\n",
        chained_helpers: "class ItemService:\n    def _pick(self, items):\n        if items:\n            return self._other(items)\n\n    def _other(self, items):\n        return items or []\n",
    });
}

#[test]
fn go_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.go",
        template: "// @Service\ntype ItemService struct {\n\t// @Needs\n\tStore Store\n}\n\nfunc (service *ItemService) Load(items []Item) *Item {\n\t{body}\n}\n\nfunc (service *ItemService) pick(items []Item) *Item {\n\tif len(items) > 0 {\n\t\treturn &items[0]\n\t}\n\treturn nil\n}\n",
        accepted: "found := service.Store.Read(items)\n\tlabel := \"if && ||\"\n\traw := `for`\n\t// if found\n\treturn service.pick(found)",
        refused: &[
            (
                "if items == nil {\n\t\treturn nil\n\t}\n\treturn &items[0]",
                "if",
            ),
            (
                "for _, item := range items {\n\t\tservice.Store.Save(item)\n\t}\n\treturn nil",
                "for",
            ),
            (
                "switch len(items) {\n\tdefault:\n\t\treturn nil\n\t}",
                "switch",
            ),
            ("return service.pick(items != nil && len(items) > 0)", "&&"),
            ("return service.pick(items == nil || len(items) == 0)", "||"),
            ("select {}", "select"),
        ],
        crowded_helper: "func (service *ItemService) pick(items []Item) int {\n\tif items == nil {\n\t\treturn 0\n\t}\n\tfor range items {\n\t}\n\treturn 1\n}\n",
        chained_helpers: "func (service *ItemService) pick(items []Item) int {\n\tif items == nil {\n\t\treturn service.other(items)\n\t}\n\treturn 1\n}\n\nfunc (service *ItemService) other(items []Item) int {\n\tswitch len(items) {\n\tdefault:\n\t\treturn 0\n\t}\n}\n",
    });
}

#[test]
fn java_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/services/ItemService.java",
        template: "@Service\npublic class ItemService {\n    @Needs\n    private Store store;\n\n    @GetRoute(\"/items\")\n    public Item load(List<Item> items) {\n        {body}\n    }\n\n    private Item pick(List<Item> items) {\n        if (items.isEmpty()) {\n            return null;\n        }\n        return items.get(0);\n    }\n}\n",
        accepted: "List<?> any = items;\n        String label = \"if && ||\";\n        List<Integer> ids = items.stream().map(Item::id).toList();\n        List<Integer> sizes = items.stream().map(item -> item.size()).toList();\n        return pick(store.read(items));",
        refused: &[
            (
                "if (items == null) return null;\n        return items.get(0);",
                "if",
            ),
            ("return items.isEmpty() ? null : items.get(0);", "? :"),
            (
                "return pick(items != null && items.isEmpty() ? null : items);",
                "&&",
            ),
            (
                "for (Item item : items) store.save(item);\n        return null;",
                "for",
            ),
            ("switch (items.size()) { default: return null; }", "switch"),
            (
                "try { return store.read(items); } catch (Exception error) { return null; }",
                "try",
            ),
            (
                "items.forEach(item -> { if (item != null) store.save(item); });\n        return null;",
                "forEach",
            ),
        ],
        crowded_helper: "public class ItemService {\n    private int pick(List<Item> items) {\n        if (items == null) return 0;\n        while (items.isEmpty()) items.remove(0);\n        return 1;\n    }\n}\n",
        chained_helpers: "public class ItemService {\n    private int pick(List<Item> items) {\n        if (items == null) return other(items);\n        return 1;\n    }\n    private int other(List<Item> items) {\n        return items.isEmpty() ? 0 : 1;\n    }\n}\n",
    });
}

#[test]
fn kotlin_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.kt",
        template: "@Service\nclass ItemService(@Needs private val store: Store) {\n    fun load(items: List<Item>): Item? {\n        {body}\n    }\n\n    private fun pick(items: List<Item>): Item? = if (items.isEmpty()) null else items[0]\n}\n",
        accepted: "val found: List<Item>? = store.read(items)\n        val size = found?.size\n        val ids = items.map { it.id }\n        val names = items.map(::name)\n        return pick(items)",
        refused: &[
            (
                "if (items.isEmpty()) return null\n        return items[0]",
                "if",
            ),
            (
                "return when (items.size) { 0 -> null else -> items[0] }",
                "when",
            ),
            ("return pick(store.read(items) ?: emptyList())", "?:"),
            (
                "return pick(items.filter { it.ready && it.shown })",
                "filter",
            ),
            (
                "for (item in items) store.save(item)\n        return null",
                "for",
            ),
            (
                "try { store.save(items) } catch (error: Exception) { }\n        return null",
                "try",
            ),
            (
                "repeat(3) { if (items.isEmpty()) store.save(items) }\n        return null",
                "repeat",
            ),
        ],
        crowded_helper: "class ItemService {\n    private fun pick(items: List<Item>): Int {\n        if (items.isEmpty()) return 0\n        return if (items.size > 1) 2 else 1\n    }\n}\n",
        chained_helpers: "class ItemService {\n    private fun pick(items: List<Item>): Int = if (items.isEmpty()) other(items) else 1\n\n    private fun other(items: List<Item>): Int = when (items.size) { 0 -> 0 else -> 1 }\n}\n",
    });
}

#[test]
fn csharp_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/controllers/ItemController.cs",
        template: "[RequestRoute(\"/items\")]\npublic class ItemController\n{\n    [Needs]\n    private Store store;\n\n    [GetRoute]\n    public Item Load(List<Item> items)\n    {\n        {body}\n    }\n\n    private Item Pick(List<Item> items)\n    {\n        if (items.Count == 0) return null;\n        return items[0];\n    }\n\n    private Item Last(List<Item> items) => items.Count > 0 ? items[items.Count - 1] : null;\n}\n",
        accepted: "string? label = null;\n        var count = items?.Count;\n        var text = $\"if {items.Count} && ||\";\n        var ids = items.Select(item => item.Id).ToList();\n        return Pick(store.Read(items));",
        refused: &[
            (
                "if (items == null) return null;\n        return items[0];",
                "if",
            ),
            ("return items.Count > 0 ? items[0] : null;", "? :"),
            ("return Pick(items ?? new List<Item>());", "??"),
            (
                "foreach (var item in items) store.Save(item);\n        return null;",
                "foreach",
            ),
            ("switch (items.Count) { default: return null; }", "switch"),
            (
                "try { return Pick(items); } catch (Exception) { return null; }",
                "try",
            ),
            (
                "return items.Where(item => item.Ready || item.Shown).First();",
                "Where",
            ),
            (
                "return (from item in items select item).First();",
                "from … in",
            ),
        ],
        crowded_helper: "public class ItemController\n{\n    private int Pick(List<Item> items)\n    {\n        if (items == null) return 0;\n        return items.Count > 1 ? 2 : 1;\n    }\n}\n",
        chained_helpers: "public class ItemController\n{\n    private int Pick(List<Item> items)\n    {\n        if (items == null) return Other(items);\n        return 1;\n    }\n    private int Other(List<Item> items) => items.Count > 1 ? 2 : 1;\n}\n",
    });
}

#[test]
fn fsharp_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/controllers/ItemController.fs",
        template: "[<RequestRoute(\"/items\")>]\ntype ItemController(store: Store) =\n    [<GetRoute>]\n    member this.Load(items: Item list) : Item =\n        {body}\n\n    member private this.Pick(items: Item list) : Item =\n        if List.isEmpty items then Unchecked.defaultof<Item> else List.head items\n",
        accepted: "let text = \"if match && ||\"\n        this.Pick(store.Read(items))",
        refused: &[
            (
                "if List.isEmpty items then Unchecked.defaultof<Item> else List.head items",
                "if",
            ),
            (
                "match items with\n        | [] -> Unchecked.defaultof<Item>\n        | first :: _ -> first",
                "match",
            ),
        ],
        crowded_helper: "type ItemController() =\n    member private this.Pick(items: Item list) : int =\n        if List.isEmpty items then 0 else match items.Length with | 1 -> 1 | _ -> 2\n",
        chained_helpers: "type ItemController() =\n    member private this.Pick(items: Item list) : int =\n        if List.isEmpty items then this.Other(items) else 1\n\n    member private this.Other(items: Item list) : int =\n        match items.Length with\n        | 1 -> 1\n        | _ -> 2\n",
    });
}

#[test]
fn php_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/controllers/ItemController.php",
        template: "<?php\n\n#[RequestRoute('/items')]\nclass ItemController\n{\n    #[Needs]\n    private Store $store;\n\n    #[GetRoute]\n    public function load(array $items): ?Item\n    {\n        {body}\n    }\n\n    private function pick(array $items): ?Item\n    {\n        if (count($items) === 0) {\n            return null;\n        }\n        return $items[0];\n    }\n}\n",
        accepted: "$found = $this->store->read($items);\n        $label = 'if && || and or';\n        $size = $this?->store;\n        $ids = array_map(fn($item) => $item->id, $items);\n        return $this->pick($found);",
        refused: &[
            (
                "if ($items === []) {\n            return null;\n        }\n        return $items[0];",
                "if",
            ),
            ("return $items ? $items[0] : null;", "? :"),
            ("return $items[0] ?: null;", "?:"),
            ("return $items[0] ?? null;", "??"),
            ("return $this->pick($items and $this->store);", "and"),
            (
                "foreach ($items as $item) {\n            $this->store->save($item);\n        }\n        return null;",
                "foreach",
            ),
            ("return match (count($items)) { default => null };", "match"),
            (
                "try {\n            return $this->pick($items);\n        } catch (Exception $error) {\n            return null;\n        }",
                "try",
            ),
        ],
        crowded_helper: "<?php\nclass ItemController\n{\n    private function pick(array $items): int\n    {\n        if ($items === []) {\n            return 0;\n        }\n        return count($items) > 1 ? 2 : 1;\n    }\n}\n",
        chained_helpers: "<?php\nclass ItemController\n{\n    private function pick(array $items): int\n    {\n        if ($items === []) {\n            return $this->other($items);\n        }\n        return 1;\n    }\n\n    private function other(array $items): int\n    {\n        return $items ? 1 : 2;\n    }\n}\n",
    });
}

#[test]
fn ruby_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.rb",
        template: "#[Service]\nclass ItemService\n  #[Needs]\n  attr_reader :store\n\n  def load(items)\n    {body}\n  end\n\n  private\n\n  def pick(items)\n    return nil if items.empty?\n    items.first\n  end\nend\n",
        accepted: "found = store.read(items)\n    ids = found.map(&:id)\n    names = found.map { |item| item.name }\n    label = \"if #{items.size} and\"\n    pick(found)",
        refused: &[
            ("return 1 if items", "if"),
            ("return 1 unless items", "unless"),
            (
                "items.each do |item|\n      store.save(item) if item\n    end",
                "each",
            ),
            ("items.map { |item| item.ready && item.shown }", "map"),
            ("pick(items || [])", "||"),
            ("pick(items or [])", "or"),
            ("items.empty? ? nil : items.first", "? :"),
            ("case items.size\n    when 0 then nil\n    end", "case"),
            (
                "begin\n      store.save(items)\n    rescue StandardError\n      nil\n    end",
                "begin",
            ),
            ("while items.any?\n      items.pop\n    end", "while"),
        ],
        crowded_helper: "class ItemService\n  private\n\n  def pick(items)\n    return 0 if items.empty?\n    return 1 unless items.first\n    2\n  end\nend\n",
        chained_helpers: "class ItemService\n  private\n\n  def pick(items)\n    return other(items) if items.empty?\n    1\n  end\n\n  def other(items)\n    items.first ? 1 : 2\n  end\nend\n",
    });
}

#[test]
fn rust_public_methods_only_coordinate() {
    check_language(&Language {
        file: "server/services/ItemService.rs",
        template: "#[Service]\nstruct ItemService {\n    store: Store,\n}\n\nimpl ItemService {\n    pub fn load(&self, items: &[Item]) -> Option<Item> {\n        {body}\n    }\n\n    fn pick(&self, items: &[Item]) -> Option<Item> {\n        if items.is_empty() {\n            return None;\n        }\n        items.first().cloned()\n    }\n}\n",
        accepted: "let found = self.store.read(items)?;\n        let pair = &&found;\n        let make = || 1;\n        let text = r#\"if && ||\"#;\n        let names: Vec<String> = found.iter().map(|item| item.name.clone()).collect();\n        self.pick(&found)",
        refused: &[
            (
                "if items.is_empty() {\n            return None;\n        }\n        None",
                "if",
            ),
            (
                "match items.len() {\n            0 => None,\n            _ => None,\n        }",
                "match",
            ),
            (
                "for item in items {\n            self.store.save(item);\n        }\n        None",
                "for",
            ),
            (
                "loop {\n            break;\n        }\n        None",
                "loop",
            ),
            (
                "self.pick(items).filter(|item| item.ready && item.shown)",
                "filter",
            ),
            (
                "let ready = items.is_empty() || self.store.ready();\n        None",
                "||",
            ),
        ],
        crowded_helper: "impl ItemService {\n    fn pick(items: &[Item]) -> usize {\n        if items.is_empty() {\n            return 0;\n        }\n        match items.len() {\n            1 => 1,\n            _ => 2,\n        }\n    }\n}\n",
        chained_helpers: "impl ItemService {\n    fn pick(&self, items: &[Item]) -> usize {\n        if items.is_empty() {\n            return self.other(items);\n        }\n        1\n    }\n\n    fn other(&self, items: &[Item]) -> usize {\n        match items.len() {\n            0 => 0,\n            _ => 1,\n        }\n    }\n}\n",
    });
}

#[test]
fn dart_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.dart",
        template: "@Service\nclass ItemService {\n  @Needs\n  late Store store;\n\n  Item? load(List<Item> items) {\n    {body}\n  }\n\n  Item? _pick(List<Item> items) => items.isEmpty ? null : items.first;\n}\n",
        accepted: "String? label;\n    final text = 'if ${items.length} &&';\n    final ids = items.map((item) => item.id).toList();\n    return _pick(store.read(items));",
        refused: &[
            (
                "if (items.isEmpty) return null;\n    return items.first;",
                "if",
            ),
            ("return items.isEmpty ? null : items.first;", "? :"),
            ("return _pick(store.read(items) ?? []);", "??"),
            (
                "for (final item in items) store.save(item);\n    return null;",
                "for",
            ),
            (
                "items.forEach((item) { if (item.ready) store.save(item); });\n    return null;",
                "forEach",
            ),
            ("return _pick([if (items.isEmpty) items.first]);", "if"),
        ],
        crowded_helper: "class ItemService {\n  int _pick(List<Item> items) {\n    if (items.isEmpty) return 0;\n    return items.length > 1 ? 2 : 1;\n  }\n}\n",
        chained_helpers: "class ItemService {\n  int _pick(List<Item> items) {\n    if (items.isEmpty) return _other(items);\n    return 1;\n  }\n\n  int _other(List<Item> items) => items.length > 1 ? 2 : 1;\n}\n",
    });
}

#[test]
fn swift_public_methods_only_coordinate() {
    check_language(&Language {
        file: "client/services/ItemService.swift",
        template: "@Service\nfinal class ItemService {\n    @Needs var store: Store\n\n    func load(_ items: [Item]) -> Item? {\n        {body}\n    }\n\n    private func pick(_ items: [Item]) -> Item? {\n        guard let first = items.first else { return nil }\n        return first\n    }\n}\n",
        accepted: "let maybe: Item? = nil\n        try? store.save(items)\n        let ids = items.map { $0.id }\n        let text = \"if \\(items.count) &&\"\n        return pick(store.read(items))",
        refused: &[
            (
                "if items.isEmpty { return nil }\n        return items.first",
                "if",
            ),
            (
                "guard let first = items.first else { return nil }\n        return first",
                "guard",
            ),
            ("return items.isEmpty ? nil : items.first", "? :"),
            ("return items.first ?? nil", "??"),
            (
                "for item in items { store.save(item) }\n        return nil",
                "for",
            ),
            (
                "switch items.count {\n        default: return nil\n        }",
                "switch",
            ),
            (
                "do { try store.save(items) } catch { }\n        return nil",
                "catch",
            ),
            ("return items.first { $0.ready && $0.shown }", "first"),
        ],
        crowded_helper: "final class ItemService {\n    private func pick(_ items: [Item]) -> Int {\n        guard let first = items.first else { return 0 }\n        return first.ready ? 1 : 2\n    }\n}\n",
        chained_helpers: "final class ItemService {\n    private func pick(_ items: [Item]) -> Int {\n        if items.isEmpty { return other(items) }\n        return 1\n    }\n\n    private func other(_ items: [Item]) -> Int {\n        return items.count > 1 ? 2 : 1\n    }\n}\n",
    });
}

#[test]
fn a_helper_may_hold_one_decision_with_a_combined_condition() {
    let source = "export default class ItemService {\n  load(items) {\n    return this.#pick(items)\n  }\n  #pick(items) {\n    if (items && items.length > 0) {\n      return items[0]\n    } else if (items) {\n      return null\n    }\n    return null\n  }\n}\n";
    assert_eq!(
        messages("client/services/ItemService.js", source),
        Vec::<String>::new()
    );
}

#[test]
fn rust_public_means_pub_or_a_trait_method() {
    let source = "impl ItemService {\n    fn helper(&self, items: &[Item]) -> usize {\n        if items.is_empty() {\n            return 0;\n        }\n        1\n    }\n}\n\nimpl Default for ItemService {\n    fn default() -> Self {\n        if READY {\n            return Self::new();\n        }\n        Self::new()\n    }\n}\n";
    let found = messages("server/services/ItemService.rs", source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("Public method `default`"));
}

#[test]
fn a_problem_points_at_the_construct_and_names_the_opt_out() {
    let source =
        "export default class ItemService {\n  load(items) {\n    return items || []\n  }\n}\n";
    let problems = CohesionCheck::problems(Path::new("client/services/ItemService.js"), source)
        .expect("readable");
    let span = &problems[0].spans[0];
    assert_eq!(
        &source[usize::try_from(span.start).expect("start")..][..2],
        "||"
    );
    assert!(
        problems[0]
            .help
            .as_deref()
            .is_some_and(|help| help.contains("tac.cohesion.check"))
    );
    let yon = CohesionCheck::problems(Path::new("server/services/ItemService.js"), source)
        .expect("readable");
    assert!(
        yon[0]
            .help
            .as_deref()
            .is_some_and(|help| help.contains("server/application.json"))
    );
}

#[test]
fn only_services_and_controllers_are_checked_and_each_realm_can_opt_out() {
    let source =
        b"export default class ItemService {\n  load(items) {\n    return items || []\n  }\n}\n";
    let every = RuleSwitch::every_realm();
    let checked =
        |path: &str, settings| CohesionCheck::check_file(Path::new(path), source, settings);
    assert!(checked("client/services/ItemService.js", every).is_err());
    assert!(checked("server/controllers/ItemController.js", every).is_err());
    assert!(checked("client/stores/ItemStore.js", every).is_ok());
    assert!(checked("client/services/nested/ItemService.js", every).is_ok());
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("client")).expect("client");
    std::fs::create_dir_all(root.path().join("server")).expect("server");
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"cohesion":{"check":false}}}"#,
    )
    .expect("manifest");
    std::fs::write(
        root.path().join("server/application.json"),
        r#"{"cohesion":{"check":true}}"#,
    )
    .expect("application");
    let directory =
        cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).expect("dir");
    let stored = RuleSwitch::cohesion(&directory);
    assert!(checked("client/services/ItemService.js", stored).is_ok());
    assert!(checked("server/services/ItemService.js", stored).is_err());
}

#[test]
fn migration_lists_every_place_to_change() {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("server/controllers")).expect("controllers");
    std::fs::write(
        root.path().join("server/controllers/ItemController.py"),
        "class ItemController:\n    def get(self, items):\n        return items or []\n\n    def _pick(self, items):\n        if items:\n            return 1\n        for item in items:\n            print(item)\n",
    )
    .expect("controller");
    let findings = CohesionCheck::migration_findings(root.path());
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert!(
        findings[0]
            .detail
            .starts_with("Line 3: Public method `get`")
    );
    assert!(
        findings[1]
            .detail
            .starts_with("Line 8: Private method `_pick`")
    );
}

#[test]
#[ignore = "reports every problem in the files that COHESION_FILES lists, one path per line"]
fn report_listed_files() {
    let list = std::env::var("COHESION_FILES").expect("COHESION_FILES names a file list");
    let root = std::env::var("COHESION_ROOT").unwrap_or_else(|_| String::from("."));
    let names = std::fs::read_to_string(list).expect("file list");
    for name in names.lines().filter(|line| !line.trim().is_empty()) {
        let source = std::fs::read_to_string(Path::new(&root).join(name)).unwrap_or_default();
        let found = CohesionCheck::problems(Path::new(name), &source);
        for problem in found.unwrap_or_default() {
            let start = usize::try_from(problem.spans[0].start).expect("start");
            let line = source[..start].matches('\n').count() + 1;
            println!("{name}:{line}: {}", problem.message);
        }
    }
}

#[test]
#[ignore = "lists the methods that the reader finds in the files that COHESION_FILES lists"]
fn list_methods_of_listed_files() {
    let list = std::env::var("COHESION_FILES").expect("COHESION_FILES names a file list");
    let root = std::env::var("COHESION_ROOT").unwrap_or_else(|_| String::from("."));
    for name in std::fs::read_to_string(list).expect("file list").lines() {
        let source = std::fs::read_to_string(Path::new(&root).join(name)).unwrap_or_default();
        let path = Path::new(name);
        let input = super::Input::new(path, &source);
        let Ok(tokens) = super::Tokens::read(&input) else {
            println!("{name}: unreadable");
            continue;
        };
        let words = super::Words::read(&tokens, &source);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(spelling) = super::Spelling::of(extension) else {
            continue;
        };
        let methods = CohesionCheck::methods(&words, &spelling);
        let names: Vec<String> = methods
            .iter()
            .map(|method| format!("{}{}", if method.public { "+" } else { "-" }, method.name))
            .collect();
        println!("{name}: {}", names.join(" "));
    }
}

#[test]
fn discovery_refuses_a_branching_service_when_the_check_is_on() {
    let root = tempfile::tempdir().expect("project");
    let write = |relative: &str, contents: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, contents).expect("file");
    };
    write("client/pages/tac.html", "<p>Home</p>");
    write(
        "client/services/ItemService.js",
        "@Service\nexport default class ItemService {\n  load(items) {\n    return items || []\n  }\n}\n",
    );
    let on_by_default = crate::ProjectDiscovery::discover(root.path()).map(|_| ());
    assert_eq!(on_by_default.is_ok(), !RuleSwitch::ON_BY_DEFAULT);
    write(
        "client/manifest.json",
        r#"{"tac":{"cohesion":{"check":true}}}"#,
    );
    let refused = crate::ProjectDiscovery::discover(root.path()).expect_err("TY1314");
    assert!(
        refused
            .diagnostics()
            .iter()
            .any(|found| found.code.number() == 1314)
    );
    write(
        "client/manifest.json",
        r#"{"tac":{"cohesion":{"check":false}}}"#,
    );
    assert!(crate::ProjectDiscovery::discover(root.path()).is_ok());
}

#[test]
fn a_ruby_endless_method_ends_at_its_line() {
    let source = "class ItemService\n  def label = \"items\"\n\n  def total=(value)\n    @total = value\n  end\n\n  def load(items)\n    items || []\n  end\nend\n";
    let found = messages("server/services/ItemService.rb", source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("Public method `load`"));
}

#[test]
fn a_class_written_inside_a_method_has_its_own_methods() {
    let source = "export default class FocusService {\n  constructor() {\n    this.#define(this.#shellClass())\n  }\n  #define(element) {\n    if (customElements.get('focus-shell')) return\n    customElements.define('focus-shell', element)\n  }\n  #shellClass() {\n    return class extends HTMLElement {\n      connectedCallback() {\n        this.#wrapOnce()\n      }\n      #wrapOnce() {\n        if (this.querySelector('slot')) return\n        this.append(document.createElement('slot'))\n      }\n    }\n  }\n}\n";
    assert_eq!(
        messages("client/services/FocusService.js", source),
        Vec::<String>::new()
    );
}

#[test]
fn promise_callbacks_and_forwarded_blocks_are_not_decisions() {
    let script = "export default class ItemService {\n  load(items) {\n    return this.store.read(items).then((value) => value, () => null).catch(() => {})\n  }\n}\n";
    assert_eq!(
        messages("client/services/ItemService.js", script),
        Vec::<String>::new()
    );
    let ruby = "class TicketController\n  def stream_events(levels)\n    tickets_for(levels) { |ticket| yield ticket }\n  end\nend\n";
    assert_eq!(
        messages("server/controllers/TicketController.rb", ruby),
        Vec::<String>::new()
    );
}
