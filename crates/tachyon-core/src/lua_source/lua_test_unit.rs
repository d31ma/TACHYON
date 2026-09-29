use super::LuaLowering;
use crate::behavior::UnitTable;
use std::fmt::Write as _;

pub(super) struct LuaTestUnit<'u> {
    unit: &'u UnitTable,
}

impl<'u> LuaTestUnit<'u> {
    const ASSERT: &'static str = include_str!("lua_assert.lua");

    pub(super) fn new(unit: &'u UnitTable) -> Self {
        Self { unit }
    }

    pub(super) fn program(&self, authored: &str, lowering: &LuaLowering<'_>) -> Result<String, String> {
        let unit = &self.unit.name;
        let resolve = lowering.provider_index(unit)?;
        let mut output = format!("{authored}\n{}\n{}", Self::ASSERT, lowering.providers()?);
        let _ = writeln!(
            output,
            "function {unit}:construct(role)\n  local built = YonGenerated.providers[role]({{}})\n  for _, name in ipairs({fakes}) do\n    if self[name] ~= nil and rawget(built, name) ~= nil then\n      built[name] = self[name]\n    end\n  end\n  return built\nend\n",
            fakes = self.fake_list()
        );
        let _ = write!(
            output,
            "Handler = {{ YON_FRAMES = true }}\n\nfunction Handler.POST(request)\n  local unit = YonGenerated.resolve_{resolve}({{}})\n  local results = {{}}\n  for _, row in ipairs(YonJson.decode({cases})) do\n{clear}    {setup}.configure({execution})\n    local ok, failure = pcall(unit[row[2]], unit, table.unpack(row[3] or {{}}))\n    local result = {{ name = row[1], passed = ok, called = {{}} }}\n    if not ok then\n      result.error = YonFailures.message(failure)\n    end\n    results[#results + 1] = result\n  end\n  return {{ unit = {name}, cases = results }}\nend\n",
            cases = Self::quoted(&self.unit.cases.to_string()),
            clear = self.clear_fakes(),
            setup = self.unit.configuration,
            execution = self.execution(),
            name = Self::quoted(unit),
        );
        Ok(output)
    }

    fn fake_list(&self) -> String {
        let names = self
            .unit
            .fakes
            .iter()
            .map(|name| Self::quoted(name))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{ {names} }}")
    }

    fn clear_fakes(&self) -> String {
        self.unit.fakes.iter().fold(String::new(), |mut lines, name| {
            let _ = writeln!(lines, "    unit.{name} = nil");
            lines
        })
    }

    fn execution(&self) -> String {
        format!(
            "{{ partition = {}, profiles = YonJson.decode({}), properties = YonJson.decode({}) }}",
            Self::quoted(&self.unit.partition),
            Self::quoted(&self.unit.profiles.to_string()),
            Self::quoted(&self.unit.properties.to_string())
        )
    }

    fn quoted(text: &str) -> String {
        let mut level = 0;
        while text.contains(&format!("]{}]", "=".repeat(level))) {
            level += 1;
        }
        let equals = "=".repeat(level);
        format!("[{equals}[{text}]{equals}]")
    }
}
