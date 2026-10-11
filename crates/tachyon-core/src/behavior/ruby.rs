use super::Input;
use crate::Failure;
use crate::events::{EventMethod, EventParameter};

pub(super) struct RubyBehaviors;

impl RubyBehaviors {
    pub(super) fn event_target(
        input: &Input<'_>,
        at: usize,
        field: bool,
    ) -> Result<EventMethod, Failure> {
        if field {
            return Err(input.failure(
                at,
                "A Ruby service creates its publisher itself: `@events = ApplicationEventPublisher.new`",
            ));
        }
        let owner = Self::owner(&input.code, at).ok_or_else(|| {
            input.failure(
                at,
                "Event annotations belong on an instance method of a top-level class",
            )
        })?;
        let line_end = input.code[at..]
            .find('\n')
            .map_or(input.code.len(), |end| at + end);
        let line = &input.code[at..line_end];
        let Some(rest) = line.strip_prefix("def ") else {
            return Err(input.failure(at, "Event annotations belong directly above a `def` line"));
        };
        if rest.starts_with("self.") {
            return Err(input.failure(at, "Event annotations require instance methods"));
        }
        let name = Self::word(rest);
        let after_name = &rest[name.len()..];
        let parameters = Self::parameter_names(after_name);
        let body = Self::body(&input.code, at, line_end);
        let comments = crate::ruby_source::Yard::comments_before(input.source, at);
        Ok(EventMethod {
            owner: owner.to_owned(),
            name: name.to_owned(),
            parameters: parameters
                .into_iter()
                .map(|parameter| EventParameter {
                    name: parameter.to_owned(),
                    type_source: crate::ruby_source::Yard::parameter_type(&comments, parameter),
                })
                .collect(),
            signature: at..line_end,
            body,
            return_type: crate::ruby_source::Yard::return_type(&comments),
            asynchronous: false,
        })
    }

    fn owner(code: &str, at: usize) -> Option<&str> {
        let before = &code[..at];
        for line in before.lines().rev() {
            if line.starts_with([' ', '\t']) || line.trim().is_empty() {
                continue;
            }
            let name = Self::word(line.strip_prefix("class ")?);
            return (!name.is_empty()).then_some(name);
        }
        None
    }

    fn parameter_names(after_name: &str) -> Vec<&str> {
        let Some(inside) = after_name.strip_prefix('(') else {
            return Vec::new();
        };
        let inside = inside.split(')').next().unwrap_or_default();
        inside
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn body(code: &str, at: usize, line_end: usize) -> std::ops::Range<usize> {
        let indent = at - code[..at].rfind('\n').map_or(0, |start| start + 1);
        let opening = (line_end + 1).min(code.len());
        let mut cursor = opening;
        for line in code[opening..].split_inclusive('\n') {
            let text = line.trim();
            if !text.is_empty() && line.len() - line.trim_start().len() <= indent {
                break;
            }
            cursor += line.len();
        }
        opening..cursor
    }

    fn visible_code(source: &str, visible: &[bool]) -> String {
        let mut bytes = source.as_bytes().to_vec();
        for (byte, shown) in bytes.iter_mut().zip(visible) {
            if !shown && !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
        String::from_utf8(bytes).unwrap_or_default()
    }

    pub(super) fn target_signatures(
        source: &str,
        class: &str,
        method: &str,
    ) -> Vec<super::gateway::TargetSignature> {
        let mask = crate::lexical::code_mask("rb", source);
        let code = Self::visible_code(source, &mask.code);
        let Some(start) = Self::class_start(&code, class) else {
            return Vec::new();
        };
        let class_code = &code[start..Self::class_end(&code, start)];
        let members = crate::ruby_source::RubyMembers::read(class_code);
        if !members.methods.iter().any(|name| name == method) {
            return Vec::new();
        }
        let mut signatures = Vec::new();
        let mut offset = 0;
        for line in class_code.split_inclusive('\n') {
            let text = line.trim_start();
            let at = offset + line.len() - text.len();
            offset += line.len();
            let Some(rest) = text.strip_prefix("def ") else {
                continue;
            };
            if Self::word(rest) != method {
                continue;
            }
            let after_name = at + "def ".len() + method.len();
            let parameters = Self::parameter_names(&class_code[after_name..]);
            signatures.push(super::gateway::TargetSignature {
                parameter_types: vec![String::new(); parameters.len()],
                result_type: String::new(),
                is_static: false,
                has_body: true,
                has_supported_receiver: true,
            });
        }
        signatures
    }

    fn class_start(code: &str, class: &str) -> Option<usize> {
        let mut offset = 0;
        for line in code.split_inclusive('\n') {
            if let Some(rest) = line.strip_prefix("class ")
                && Self::word(rest) == class
            {
                return Some(offset);
            }
            offset += line.len();
        }
        None
    }

    fn class_end(code: &str, start: usize) -> usize {
        let mut offset = start;
        for line in code[start..].split_inclusive('\n') {
            offset += line.len();
            if line.trim_end() == "end" {
                return offset;
            }
        }
        code.len()
    }

    fn word(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        &text[..length]
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RubyBehaviors;
    use std::path::Path;

    #[test]
    fn method_and_class_names_read_the_same_leading_ascii_word() {
        let alphabet = (0..=0x7f_u8)
            .map(char::from)
            .chain(['é', 'Ü', 'ß', '٣'])
            .collect::<Vec<_>>();
        let mut texts = vec![String::new()];
        for first in &alphabet {
            texts.push(first.to_string());
            texts.extend(alphabet.iter().map(|second| format!("{first}{second}")));
        }
        for text in texts {
            let length = text
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            assert_eq!(RubyBehaviors::word(&text), &text[..length], "{text:?}");
        }
        let source = "class CatalogService < Base\n  def list(page, size)\n  end\n\n  def list_all\n  end\nend\n";
        let signatures = RubyBehaviors::target_signatures(source, "CatalogService", "list");
        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].parameter_types.len(), 2);
        assert!(RubyBehaviors::target_signatures(source, "Catalog", "list").is_empty());
        assert!(RubyBehaviors::target_signatures(source, "CatalogService", "lis").is_empty());
    }

    #[test]
    fn a_test_unit_becomes_a_controller_that_runs_its_cases() {
        let source = "#[Test(CatalogTestSetup)]\n#[UseProfiles(\"test\")]\nclass CatalogServiceTest\n  def test_counts\n  end\n\n  def helper\n  end\n\n  def test_fails(value = 1)\n  end\nend\n";
        let lowered = crate::behavior::lower(
            Path::new("server/tests/CatalogServiceTest.rb"),
            source,
            "yon",
        )
        .expect("lowered")
        .expect("changed");
        let lines: Vec<&str> = lowered.lines().collect();
        assert_eq!(lines[0], "#[RequestRoute]", "{lowered}");
        assert_eq!(lines[1].trim(), "", "{lowered}");
        assert_eq!(lines[2], "class CatalogServiceTest", "{lowered}");
        assert!(
            lowered.contains("@behaviors = Behaviors.create(CatalogTestSetup, "),
            "{lowered}"
        );
        assert!(
            lowered.contains(
                "cases = JSON.parse('[[\"test_counts\",\"test_counts\",[]],[\"test_fails\",\"test_fails\",[]]]').map do |name, method, input|"
            ),
            "{lowered}"
        );
        assert_eq!(lowered.lines().take(12).count(), source.lines().count());
    }

    #[test]
    fn a_cached_method_runs_through_its_behavior_context_on_the_same_lines() {
        let source = "#[CacheSettings({\"name\":\"prices\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})]\nclass PriceService\n  #[Cached]\n  # @param sku [String]\n  def price(sku)\n    return 0 if sku.empty?\n    sku.size\n  end\nend\n";
        let lowered =
            crate::behavior::lower(Path::new("server/services/PriceService.rb"), source, "yon")
                .expect("lowered")
                .expect("changed");
        let lines: Vec<&str> = lowered.lines().collect();
        assert_eq!(
            lines[4],
            "  def price(sku); @behaviors.invoke(self, \"PriceService\", \"price\", [sku], lambda do |sku|",
            "{lowered}"
        );
        assert_eq!(lines[5], "    return 0 if sku.empty?", "{lowered}");
        assert_eq!(lines[7], "  end); end", "{lowered}");
        assert!(lowered.contains("Behaviors.register('"), "{lowered}");
    }

    #[test]
    fn a_ruby_behavior_annotation_is_checked_by_the_shared_rule_table() {
        let failure = crate::behavior::lower(
            Path::new("server/services/PriceService.rb"),
            "#[Service]\nclass PriceService\n  #[Cached]\n  def price\n  end\nend\n",
            "yon",
        )
        .expect_err("Cached needs CacheSettings on its class");
        assert!(
            failure
                .to_string()
                .contains("Cached requires its class Cache policy"),
            "{failure}"
        );
    }
}
