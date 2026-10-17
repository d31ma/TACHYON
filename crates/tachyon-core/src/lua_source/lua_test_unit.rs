use super::LuaLowering;
use crate::behavior::UnitTable;
use std::fmt::Write as _;

pub(crate) struct LuaTestUnit<'u> {
    unit: &'u UnitTable,
    behaviors: bool,
}

impl<'u> LuaTestUnit<'u> {
    const ASSERT: &'static str = include_str!("lua_assert.lua");
    const STOPS: [&'static str; 2] = [
        "",
        "    unit.__behaviors:dispose()\n    unit.__behaviors = nil\n",
    ];

    pub(crate) fn new(unit: &'u UnitTable) -> Self {
        Self {
            unit,
            behaviors: false,
        }
    }

    pub(crate) fn with_behaviors(self, behaviors: bool) -> Self {
        Self { behaviors, ..self }
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
            "function {unit}:construct(role)\n  local built = {builder}\n  for _, name in ipairs({fakes}) do\n    if self[name] ~= nil and rawget(built, name) ~= nil then\n      built[name] = self[name]\n    end\n  end\n  built.behaviors = rawget(self, \"__behaviors\") or built.behaviors\n  return built\nend\n\n",
            unit = self.unit.name,
            fakes = self.fake_list()
        )
    }

    fn runner(&self, unit: &str) -> String {
        format!(
            "  local unit = {unit}\n  local results = {{}}\n  for _, row in ipairs(YonJson.decode({cases})) do\n{clear}{start}    BehaviorCalls.take()\n    local ok, failure = pcall(unit[row[2]], unit, YonJson.arguments(row[3] or {{}}))\n{stop}    local result = {{ name = row[1], passed = ok, called = BehaviorCalls.take() }}\n    if not ok then\n      result.error = tostring(type(failure) == \"table\" and failure.message or failure)\n    end\n    results[#results + 1] = result\n  end\n  return {{ unit = {name}, cases = results }}\n",
            cases = Self::quoted(&self.unit.cases.to_string()),
            clear = self.clear_fakes(),
            start = self.start(),
            stop = Self::STOPS[usize::from(self.behaviors)],
            name = Self::quoted(&self.unit.name),
        )
    }

    fn start(&self) -> String {
        let setup = &self.unit.configuration;
        let execution = self.execution();
        [
            format!("    {setup}.configure({execution})\n"),
            format!(
                "    unit.__behaviors = Behaviors.create({setup}, {})\n",
                execution.replacen('{', "{ signal = BehaviorSignal.new(),", 1)
            ),
        ]
        .into_iter()
        .nth(usize::from(self.behaviors))
        .unwrap_or_default()
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
            Self::quoted(&self.flat_properties().to_string())
        )
    }

    fn flat_properties(&self) -> serde_json::Value {
        let flat = self
            .unit
            .properties
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(path, value)| {
                let text = match value {
                    serde_json::Value::String(text) => text.clone(),
                    serde_json::Value::Null => return None,
                    other => other.to_string(),
                };
                Some((path.replace('.', "_").to_ascii_uppercase(), text.into()))
            })
            .collect();
        serde_json::Value::Object(flat)
    }

    fn quoted(text: &str) -> String {
        super::LuaSource::long_string(text)
    }
}
