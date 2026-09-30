use super::LuaLowering;
use crate::behavior::UnitTable;
use std::fmt::Write as _;

pub(crate) struct LuaTestUnit<'u> {
    unit: &'u UnitTable,
}

impl<'u> LuaTestUnit<'u> {
    const ASSERT: &'static str = include_str!("lua_assert.lua");

    pub(crate) fn new(unit: &'u UnitTable) -> Self {
        Self { unit }
    }

    pub(super) fn program(
        &self,
        authored: &str,
        lowering: &LuaLowering<'_>,
    ) -> Result<String, String> {
        let resolve = lowering.provider_index(&self.unit.name)?;
        Ok(format!(
            "{authored}\n{}\n{}{}Handler = {{ YON_FRAMES = true }}\n\nfunction Handler.POST(request)\n{}end\n",
            Self::ASSERT,
            lowering.providers()?,
            self.construct("YonGenerated.providers[role]({})"),
            self.runner(&format!("YonGenerated.resolve_{resolve}({{}})"))
        ))
    }

    pub(crate) fn tac_program(&self, authored: &str) -> String {
        format!(
            "{}\n{authored}\n{}print(YonJson.encode((function()\n{}end)()))\n",
            Self::ASSERT,
            self.construct("TacServices.construct(role)"),
            self.runner(&format!("TacServices.construct({})", self.unit.name))
        )
    }

    fn construct(&self, builder: &str) -> String {
        format!(
            "function {unit}:construct(role)\n  local built = {builder}\n  for _, name in ipairs({fakes}) do\n    if self[name] ~= nil and rawget(built, name) ~= nil then\n      built[name] = self[name]\n    end\n  end\n  return built\nend\n\n",
            unit = self.unit.name,
            fakes = self.fake_list()
        )
    }

    fn runner(&self, unit: &str) -> String {
        format!(
            "  local unit = {unit}\n  local results = {{}}\n  for _, row in ipairs(YonJson.decode({cases})) do\n{clear}    {setup}.configure({execution})\n    BehaviorCalls.take()\n    local ok, failure = pcall(unit[row[2]], unit, table.unpack(row[3] or {{}}))\n    local result = {{ name = row[1], passed = ok, called = BehaviorCalls.take() }}\n    if not ok then\n      result.error = tostring(type(failure) == \"table\" and failure.message or failure)\n    end\n    results[#results + 1] = result\n  end\n  return {{ unit = {name}, cases = results }}\n",
            cases = Self::quoted(&self.unit.cases.to_string()),
            clear = self.clear_fakes(),
            setup = self.unit.configuration,
            execution = self.execution(),
            name = Self::quoted(&self.unit.name),
        )
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
        self.unit
            .fakes
            .iter()
            .fold(String::new(), |mut lines, name| {
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
        super::LuaSource::long_string(text)
    }
}
