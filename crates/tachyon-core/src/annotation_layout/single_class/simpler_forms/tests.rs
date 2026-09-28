use super::SimplerForms;
use super::form_scan::FormScan;
use std::path::Path;

fn apply(path: &str, source: &str) -> (Vec<u16>, String) {
    let mut forms = FormScan::forms(Path::new(path), source);
    let codes = forms.iter().map(|form| form.pattern.code()).collect();
    forms.sort_by_key(|form| std::cmp::Reverse(form.start));
    let text = forms.iter().fold(source.to_owned(), |text, form| {
        format!(
            "{}{}{}",
            &text[..form.start],
            form.replacement,
            &text[form.end..]
        )
    });
    (codes, text)
}

fn check(path: &str, before: &str, code: u16, after: &str) {
    let (codes, text) = apply(path, before);
    assert_eq!(codes, vec![code], "codes for {path}:\n{before}");
    assert_eq!(text, after, "rewrite for {path}");
    let (again, _) = apply(path, &text);
    assert!(
        !again.contains(&code),
        "{code} is suggested again for {path}:\n{text}"
    );
}

fn none(path: &str, before: &str) {
    let (codes, _) = apply(path, before);
    assert!(
        codes.is_empty(),
        "unexpected {codes:?} for {path}:\n{before}"
    );
}

fn js(body: &str) -> String {
    format!("export class Shop {{\n  run(a, b, c, items, names, name, user) {{\n{body}  }}\n}}\n")
}

fn python(body: &str) -> String {
    format!("class Shop:\n    def run(self, a, b, items, cache, key, parts):\n{body}")
}

fn ruby(body: &str) -> String {
    format!("class Shop\n  def run(a, b, items, cache, key)\n{body}  end\nend\n")
}

fn java(body: &str) -> String {
    format!(
        "public class Shop {{\n    boolean run(boolean ready, int a, int b, String text, String word, List<String> items) {{\n{body}    }}\n}}\n"
    )
}

#[test]
fn boolean_return_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    if (a > b) {\n      return true;\n    }\n    return false;\n"),
        1340,
        &js("    return a > b;\n"),
    );
    check(
        "server/services/Shop.js",
        &js("    if (a === b) {\n      return false;\n    } else {\n      return true;\n    }\n"),
        1340,
        &js("    return a !== b;\n"),
    );
    check(
        "server/services/shop.py",
        &python("        if a > b:\n            return True\n        return False\n"),
        1340,
        &python("        return a > b\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    if a > b\n      return true\n    else\n      return false\n    end\n"),
        1340,
        &ruby("    return a > b\n"),
    );
    check(
        "server/services/Shop.java",
        &java("        if (ready) {\n            return true;\n        }\n        return false;\n"),
        1340,
        &java("        return ready;\n"),
    );
}

#[test]
fn boolean_return_near_misses_are_quiet() {
    none(
        "server/services/Shop.js",
        &js("    if (user) {\n      return true;\n    }\n    return false;\n"),
    );
    none(
        "server/services/shop.py",
        &python("        if items:\n            return True\n        return False\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    if (a > b) {\n      return true;\n    }\n    return true;\n"),
    );
}

#[test]
fn boolean_assignment_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js(
            "    if (a > b) {\n      this.big = true;\n    } else {\n      this.big = false;\n    }\n",
        ),
        1355,
        &js("    this.big = a > b;\n"),
    );
    none(
        "server/services/Shop.js",
        &js(
            "    if (a > b) {\n      this.big = true;\n    } else {\n      this.small = false;\n    }\n",
        ),
    );
}

#[test]
fn boolean_choice_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    const big = a > b ? true : false;\n    return big;\n"),
        1342,
        &js("    const big = a > b;\n    return big;\n"),
    );
    check(
        "server/services/Shop.js",
        &js("    const small = a > b ? false : true;\n    return small;\n"),
        1342,
        &js("    const small = !(a > b);\n    return small;\n"),
    );
    check(
        "server/services/shop.py",
        &python("        big = True if a > b else False\n        return big\n"),
        1342,
        &python("        big = a > b\n        return big\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    big = a > b ? true : false\n    big\n"),
        1342,
        &ruby("    big = a > b\n    big\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    const known = user ? true : false;\n    return known;\n"),
    );
}

#[test]
fn any_instead_of_filter_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    return items.filter((item) => item.ready).length > 0;\n"),
        1343,
        &js("    return items.some((item) => item.ready);\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    items.select { |item| item.ready }.length > 0\n"),
        1343,
        &ruby("    items.any? { |item| item.ready }\n"),
    );
    check(
        "server/services/Shop.kt",
        "class Shop {\n    fun run(items: List<Item>): Boolean {\n        return items.filter { it.ready }.isNotEmpty()\n    }\n}\n",
        1343,
        "class Shop {\n    fun run(items: List<Item>): Boolean {\n        return items.any { it.ready }\n    }\n}\n",
    );
    none(
        "server/services/Shop.js",
        &js("    return items.filter((item) => item.ready).length > 1;\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    return 1 + items.filter((item) => item.ready).length > 0;\n"),
    );
}

#[test]
fn contains_instead_of_index_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    return names.indexOf(name) !== -1;\n"),
        1344,
        &js("    return names.includes(name);\n"),
    );
    check(
        "server/services/Shop.js",
        &js("    return names.indexOf(name) === -1;\n"),
        1344,
        &js("    return !names.includes(name);\n"),
    );
    check(
        "server/services/shop.py",
        &python("        return key.find(a) != -1\n"),
        1344,
        &python("        return a in key\n"),
    );
    check(
        "server/services/shop.py",
        &python("        return key.find(a) == -1\n"),
        1344,
        &python("        return a not in key\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    items.index(key) != nil\n"),
        1344,
        &ruby("    items.include?(key)\n"),
    );
    check(
        "server/services/shop.go",
        "package services\n\nfunc (s *Shop) Run(text string, word string) bool {\n\treturn strings.Index(text, word) >= 0\n}\n",
        1344,
        "package services\n\nfunc (s *Shop) Run(text string, word string) bool {\n\treturn strings.Contains(text, word)\n}\n",
    );
    check(
        "server/services/Shop.php",
        "<?php\nclass Shop {\n    public function run($text, $word) {\n        return strpos($text, $word) !== false;\n    }\n}\n",
        1344,
        "<?php\nclass Shop {\n    public function run($text, $word) {\n        return str_contains($text, $word);\n    }\n}\n",
    );
    none(
        "server/services/Shop.js",
        &js("    return names.indexOf(name) > 0;\n"),
    );
    none(
        "server/services/Shop.java",
        &java("        return text.indexOf('a') != -1;\n"),
    );
}

#[test]
fn else_after_exit_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js(
            "    if (a > b) {\n      return a;\n    } else {\n      b += 1;\n      return b;\n    }\n",
        ),
        1345,
        &js("    if (a > b) {\n      return a;\n    }\n    b += 1;\n    return b;\n"),
    );
    check(
        "server/services/Shop.js",
        &js(
            "    if (a) {\n      return 1;\n    } else if (b) {\n      c.run();\n    }\n    return 2;\n",
        ),
        1345,
        &js(
            "    if (a) {\n      return 1;\n    }\n    if (b) {\n      c.run();\n    }\n    return 2;\n",
        ),
    );
    check(
        "server/services/shop.py",
        &python(
            "        if a > b:\n            return a\n        else:\n            b += 1\n            return b\n",
        ),
        1345,
        &python("        if a > b:\n            return a\n        b += 1\n        return b\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    if a > b\n      return a\n    else\n      puts b\n    end\n"),
        1345,
        &ruby("    if a > b\n      return a\n    end\n    puts b\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    if (a > b) {\n      c.run();\n    } else {\n      c.stop();\n    }\n"),
    );
    none(
        "server/services/Shop.js",
        &js(
            "    if (a > b) {\n      return a;\n    } else {\n      const d = b;\n      c.run(d);\n    }\n",
        ),
    );
}

#[test]
fn double_negation_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    if (!!user) {\n      c.run();\n    }\n"),
        1346,
        &js("    if (user) {\n      c.run();\n    }\n"),
    );
    check(
        "server/services/shop.py",
        &python("        if not not items:\n            print(a)\n"),
        1346,
        &python("        if items:\n            print(a)\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    const known = !!user;\n    return known;\n"),
    );
}

#[test]
fn negated_equality_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    if (!(a === b)) {\n      c.run();\n    }\n"),
        1347,
        &js("    if (a !== b) {\n      c.run();\n    }\n"),
    );
    check(
        "server/services/shop.py",
        &python("        if not a == b:\n            print(a)\n"),
        1347,
        &python("        if a != b:\n            print(a)\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    if (!(a < b)) {\n      c.run();\n    }\n"),
    );
}

#[test]
fn extra_return_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    c.run();\n    return;\n"),
        1348,
        &js("    c.run();\n"),
    );
    check(
        "server/services/shop.py",
        &python("        print(a)\n        return None\n"),
        1348,
        &python("        print(a)\n"),
    );
    none("server/services/shop.rb", &ruby("    puts a\n    return\n"));
    none(
        "server/services/shop.py",
        &python("        if a:\n            return a\n        print(b)\n        return None\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    if (a) {\n      c.run();\n      return;\n    }\n    c.stop();\n"),
    );
}

#[test]
fn joined_if_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    if (a) {\n      if (b || c) {\n        user.run();\n      }\n    }\n"),
        1349,
        &js("    if (a && (b || c)) {\n      user.run();\n    }\n"),
    );
    check(
        "server/services/shop.py",
        &python("        if a:\n            if b:\n                print(a)\n"),
        1349,
        &python("        if a and b:\n            print(a)\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    if a\n      if b\n        puts a\n      end\n    end\n"),
        1349,
        &ruby("    if a && b\n      puts a\n    end\n"),
    );
    none(
        "server/services/Shop.js",
        &js(
            "    if (a) {\n      if (b) {\n        user.run();\n      }\n      user.stop();\n    }\n",
        ),
    );
}

#[test]
fn compound_assignment_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js("    let total = a;\n    total = total + b;\n    return total;\n"),
        1350,
        &js("    let total = a;\n    total += b;\n    return total;\n"),
    );
    check(
        "server/services/shop.py",
        &python("        count = 0\n        count = count + 1\n        return count\n"),
        1350,
        &python("        count = 0\n        count += 1\n        return count\n"),
    );
    none(
        "server/services/shop.py",
        &python("        items = items + [a]\n        return items\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    let total = a;\n    total = b + total;\n    return total;\n"),
    );
}

#[test]
fn dictionary_get_is_suggested_and_applied() {
    check(
        "server/services/shop.py",
        &python("        value = cache[key] if key in cache else None\n        return value\n"),
        1351,
        &python("        value = cache.get(key, None)\n        return value\n"),
    );
    check(
        "server/services/shop.py",
        &python(
            "        if key in cache:\n            value = cache[key]\n        else:\n            value = 0\n        return value\n",
        ),
        1351,
        &python("        value = cache.get(key, 0)\n        return value\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby("    value = cache.key?(key) ? cache[key] : 0\n    value\n"),
        1351,
        &ruby("    value = cache.fetch(key, 0)\n    value\n"),
    );
    none(
        "server/services/shop.py",
        &python(
            "        value = cache[key] if key in cache else self.load(key)\n        return value\n",
        ),
    );
}

#[test]
fn total_loop_is_suggested_and_applied() {
    check(
        "server/services/shop.py",
        &python(
            "        text = \"\"\n        for part in parts:\n            text += part\n        return text\n",
        ),
        1352,
        &python("        text = \"\".join(parts)\n        return text\n"),
    );
    check(
        "server/services/Shop.php",
        "<?php\nclass Shop {\n    public function run($prices) {\n        $total = 0;\n        foreach ($prices as $price) {\n            $total += $price;\n        }\n        return $total;\n    }\n}\n",
        1352,
        "<?php\nclass Shop {\n    public function run($prices) {\n        $total = array_sum($prices);\n        return $total;\n    }\n}\n",
    );
    none(
        "server/services/shop.py",
        &python(
            "        total = 0\n        for part in parts:\n            total += part\n        return total\n",
        ),
    );
}

#[test]
fn is_empty_is_suggested_and_applied() {
    check(
        "server/services/shop.rb",
        &ruby("    items.length == 0\n"),
        1353,
        &ruby("    items.empty?\n"),
    );
    check(
        "server/services/Shop.kt",
        "class Shop {\n    fun run(items: List<Item>): Boolean {\n        return items.size > 0\n    }\n}\n",
        1353,
        "class Shop {\n    fun run(items: List<Item>): Boolean {\n        return items.isNotEmpty()\n    }\n}\n",
    );
    none(
        "server/services/Shop.js",
        &js("    return items.length === 0;\n"),
    );
}

#[test]
fn for_each_is_suggested_and_applied() {
    check(
        "server/services/Shop.js",
        &js(
            "    for (let i = 0; i < items.length; i++) {\n      const item = items[i];\n      c.run(item);\n    }\n",
        ),
        1354,
        &js("    for (const item of items) {\n      c.run(item);\n    }\n"),
    );
    check(
        "server/services/shop.py",
        &python(
            "        for i in range(len(items)):\n            item = items[i]\n            print(item)\n",
        ),
        1354,
        &python("        for item in items:\n            print(item)\n"),
    );
    check(
        "server/services/shop.rb",
        &ruby(
            "    (0...items.length).each do |i|\n      item = items[i]\n      puts item\n    end\n",
        ),
        1354,
        &ruby("    items.each do |item|\n      puts item\n    end\n"),
    );
    none(
        "server/services/Shop.js",
        &js(
            "    for (let i = 0; i < items.length; i++) {\n      const item = items[i];\n      c.run(item, i);\n    }\n",
        ),
    );
}

#[test]
fn the_realm_switch_turns_suggestions_off() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("client/services"))?;
    std::fs::write(
        root.path().join("client/manifest.json"),
        "{\"tac\": {\"simplify\": {\"suggest\": false}}}",
    )?;
    let source = js("    if (a > b) {\n      return true;\n    }\n    return false;\n");
    std::fs::write(root.path().join("client/services/Shop.js"), &source)?;
    assert!(SimplerForms::edits(root.path(), "client/services/Shop.js", &source).is_empty());
    assert_eq!(
        SimplerForms::edits(root.path(), "server/services/Shop.js", &source).len(),
        1
    );
    assert!(SimplerForms::report(root.path()).is_empty());
    Ok(())
}

#[test]
fn the_report_shows_the_rewritten_line() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("server/services"))?;
    let source = js("    if (a > b) {\n      return true;\n    }\n    return false;\n");
    std::fs::write(root.path().join("server/services/Shop.js"), &source)?;
    let report = SimplerForms::report(root.path());
    assert!(report.contains("suggestion[TY1340]"), "{report}");
    assert!(
        report.contains("  --> server/services/Shop.js:3"),
        "{report}"
    );
    assert!(report.contains("  help: return a > b;"), "{report}");
    assert!(report.contains("ty migrate fix --write"), "{report}");
    Ok(())
}

#[test]
fn migrate_fix_applies_every_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("server/services"))?;
    let source = js("    if (a > b) {\n      return true;\n    }\n    return false;\n");
    std::fs::write(root.path().join("server/services/Shop.js"), &source)?;
    let plan = crate::migrate::MigrationFix::plan(root.path())?;
    plan.write(root.path())?;
    let written = std::fs::read_to_string(root.path().join("server/services/Shop.js"))?;
    assert_eq!(written, js("    return a > b;\n"));
    let again = crate::migrate::MigrationFix::plan(root.path())?;
    assert_eq!(again.change_count(), 0);
    Ok(())
}

#[test]
fn boolean_compare_is_suggested_and_applied() {
    check(
        "server/services/Shop.java",
        &java("        return ready == true;\n"),
        1341,
        &java("        return ready;\n"),
    );
    check(
        "server/services/Shop.java",
        &java("        return ready == false;\n"),
        1341,
        &java("        return !ready;\n"),
    );
    none(
        "server/services/Shop.js",
        &js("    return user === true;\n"),
    );
}

#[test]
fn large_and_odd_sources_are_scanned_quickly() {
    let started = std::time::Instant::now();
    let mut nested = String::from("export class Shop {\n  run(a) {\n");
    nested.push_str(&"    if (a) {\n".repeat(400));
    nested.push_str(&"    }\n".repeat(400));
    nested.push_str("  }\n}\n");
    let mut flat = String::from("export class Shop {\n  run(a, b) {\n");
    flat.push_str(
        &"    if (a > b) { b.run(!!a ? a : b, [1, 2, 3].indexOf(a) !== -1); }\n".repeat(4000),
    );
    flat.push_str("  }\n}\n");
    let unbalanced = "{".repeat(20_000) + &"(".repeat(20_000) + "if a ? true : false";
    let objects = format!(
        "export const table = {{\n{}}};\n",
        "  key: { a: { b: { c: () => { return; } } } },\n".repeat(3000)
    );
    for (path, source) in [
        ("client/services/Nested.js", nested.as_str()),
        ("client/services/Flat.js", flat.as_str()),
        ("client/shared/scripts/odd.js", unbalanced.as_str()),
        ("client/shared/scripts/table.js", objects.as_str()),
        ("server/services/odd.py", unbalanced.as_str()),
        ("server/services/odd.rb", unbalanced.as_str()),
    ] {
        let _ = FormScan::forms(Path::new(path), source);
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn other_languages_are_covered() {
    check(
        "server/services/shop.go",
        "package services\n\nfunc (s *Shop) Run(a int, b int) bool {\n\tif a > b {\n\t\treturn true\n\t}\n\treturn false\n}\n",
        1340,
        "package services\n\nfunc (s *Shop) Run(a int, b int) bool {\n\treturn a > b\n}\n",
    );
    check(
        "server/services/shop.rs",
        "impl Shop {\n    fn run(&self, a: i32, b: i32) -> i32 {\n        if a > b {\n            return a;\n        } else {\n            self.save(b);\n            b\n        }\n    }\n}\n",
        1345,
        "impl Shop {\n    fn run(&self, a: i32, b: i32) -> i32 {\n        if a > b {\n            return a;\n        }\n        self.save(b);\n        b\n    }\n}\n",
    );
    check(
        "server/services/Shop.cs",
        "public class Shop {\n    public void Run(List<string> items) {\n        for (int i = 0; i < items.Count; i++) {\n            var item = items[i];\n            Console.WriteLine(item);\n        }\n    }\n}\n",
        1354,
        "public class Shop {\n    public void Run(List<string> items) {\n        foreach (var item in items) {\n            Console.WriteLine(item);\n        }\n    }\n}\n",
    );
    check(
        "client/services/shop_service.dart",
        "class ShopService {\n  bool run(List<String> items) {\n    return items.length == 0;\n  }\n}\n",
        1353,
        "class ShopService {\n  bool run(List<String> items) {\n    return items.isEmpty;\n  }\n}\n",
    );
    check(
        "client/services/ShopService.swift",
        "class ShopService {\n    func run(a: Bool, b: Bool) {\n        if a {\n            if b {\n                print(\"both\")\n            }\n        }\n    }\n}\n",
        1349,
        "class ShopService {\n    func run(a: Bool, b: Bool) {\n        if a && b {\n            print(\"both\")\n        }\n    }\n}\n",
    );
    check(
        "server/services/Shop.kt",
        "class Shop {\n    fun run(prices: List<Int>): Int {\n        var total = 0\n        for (price in prices) {\n            total += price\n        }\n        return total\n    }\n}\n",
        1352,
        "class Shop {\n    fun run(prices: List<Int>): Int {\n        var total = prices.sum()\n        return total\n    }\n}\n",
    );
    check(
        "server/services/Shop.php",
        "<?php\nclass Shop {\n    public function run($a, $b) {\n        if ($a > $b) {\n            return $a;\n        } else {\n            echo $b;\n            return $b;\n        }\n    }\n}\n",
        1345,
        "<?php\nclass Shop {\n    public function run($a, $b) {\n        if ($a > $b) {\n            return $a;\n        }\n        echo $b;\n        return $b;\n    }\n}\n",
    );
    check(
        "client/services/ShopService.ts",
        "export class ShopService {\n  run(a: number, b: number): void {\n    if (!(a === b)) {\n      this.save(a);\n    }\n  }\n}\n",
        1347,
        "export class ShopService {\n  run(a: number, b: number): void {\n    if (a !== b) {\n      this.save(a);\n    }\n  }\n}\n",
    );
}
