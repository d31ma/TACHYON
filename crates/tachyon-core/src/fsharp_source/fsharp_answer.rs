use super::mapping_shape::MappingShape;

pub(crate) struct FsharpAnswer;

impl FsharpAnswer {
    const TASKS: [&'static str; 2] = ["Task", "ValueTask"];
    const JSON_TASK: &'static str = "Task<BehaviorJSON>";
    const PREFIXES: [&'static str; 3] = ["System.Threading.Tasks.", "Models.", "public "];
    const READER: &'static str = "TacMappingModels.answer";
    const LIST_FORMS: [(&'static str, &'static str); 2] =
        [("list<", " list"), ("array<", " array")];

    pub(crate) fn wrapped(result: &str, answer: &str) -> Result<String, String> {
        Self::task_parts(result.trim()).map_or_else(
            || Ok(answer.to_owned()),
            |(base, payload)| Self::task_answer(base, payload, answer),
        )
    }

    pub(crate) fn exchange_text(shadow: &str) -> String {
        let lines: Vec<String> = shadow.split('\n').map(Self::exchange_line).collect();
        lines.join("\n")
    }

    fn exchange_line(line: &str) -> String {
        Self::exchanged(line).unwrap_or_else(|| line.to_owned())
    }

    fn exchanged(line: &str) -> Option<String> {
        line.contains(") { }").then_some(())?;
        let indent = &line[..line.len() - line.trim_start().len()];
        let rest = line.trim_start().strip_prefix(Self::PREFIXES[2])?;
        let rest = rest.strip_prefix("async ").unwrap_or(rest);
        let (kind, after) = rest.split_once(' ')?;
        Self::task_parts(kind)?;
        (kind != Self::JSON_TASK)
            .then(|| format!("{indent}public async {} {after}", Self::JSON_TASK))
    }

    fn task_parts(result: &str) -> Option<(&str, Option<&str>)> {
        let plain = result.strip_prefix(Self::PREFIXES[0]).unwrap_or(result);
        let (base, rest) = plain
            .split_once('<')
            .map_or((plain, None), |(base, rest)| (base, Some(rest)));
        Self::TASKS.contains(&base).then_some(())?;
        let payload = rest.map(|text| text.strip_suffix('>').map(str::trim));
        payload.map_or(Some((base, None)), |inner| {
            inner.map(|inner| (base, Some(inner)))
        })
    }

    fn task_answer(base: &str, payload: Option<&str>, answer: &str) -> Result<String, String> {
        let read = payload.map(Self::read).transpose()?;
        Ok(Self::written(
            base,
            payload,
            read.flatten().as_deref(),
            answer,
        ))
    }

    fn read(payload: &str) -> Result<Option<String>, String> {
        let kind = Self::normalized(payload);
        let json = kind == "BehaviorJSON";
        let read = (!json).then(|| MappingShape::read(&kind, "answer", Self::READER));
        read.transpose().map_err(|_| format!("An F# HTTPClient cannot read an answer of type {payload}; use a model from client/models, a list or array of models, a plain value or BehaviorJSON."))
    }

    fn normalized(payload: &str) -> String {
        let mut kind = payload.trim().replace(Self::PREFIXES[1], "");
        for (prefix, suffix) in Self::LIST_FORMS {
            let inner = kind
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix('>'))
                .map(|inner| format!("{}{suffix}", inner.trim()));
            kind = inner.unwrap_or(kind);
        }
        kind
    }

    fn written(base: &str, payload: Option<&str>, read: Option<&str>, answer: &str) -> String {
        let reading = read
            .map(Self::checked)
            .map(|checked| format!("task {{ let! answer = {answer} in return {checked} }}"));
        let typed = reading.unwrap_or_else(|| answer.to_owned());
        let finished =
            format!("(task {{ let! _ = {answer} in return () }} :> System.Threading.Tasks.Task)");
        let value_task = base == Self::TASKS[1];
        match (value_task, payload) {
            (false, Some(_)) => typed,
            (false, None) => finished,
            (true, Some(kind)) => format!("System.Threading.Tasks.ValueTask<{kind}>({typed})"),
            (true, None) => format!("System.Threading.Tasks.ValueTask({finished})"),
        }
    }

    fn checked(read: &str) -> String {
        format!("(try {read} with _ -> raise (BehaviorError(\"HTTP_RESPONSE\")))")
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpAnswer;

    const ANSWER: &str = "this.behaviors.Invoke()";

    fn wrapped(result: &str) -> String {
        FsharpAnswer::wrapped(result, ANSWER).unwrap_or_else(|message| panic!("{message}"))
    }

    #[test]
    fn a_json_task_keeps_the_answer() {
        assert_eq!(wrapped("Task<BehaviorJSON>"), ANSWER);
        assert_eq!(wrapped("BehaviorJSON"), ANSWER);
    }

    #[test]
    fn a_model_answer_is_read_and_a_wrong_shape_is_http_response() {
        let model = wrapped("System.Threading.Tasks.Task<Models.Greeting>");
        assert_eq!(
            model,
            "task { let! answer = this.behaviors.Invoke() in return (try TacMappingModels.answerGreeting (answer) with _ -> raise (BehaviorError(\"HTTP_RESPONSE\"))) }"
        );
        let list = wrapped("Task<Greeting list>");
        assert!(
            list.contains("(answer).Items |> Array.map (fun item -> TacMappingModels.answerGreeting (item)) |> Array.toList"),
            "{list}"
        );
        assert_eq!(wrapped("Task<list<Greeting>>"), list);
        assert!(wrapped("Task<string>").contains("(answer).Text"));
    }

    #[test]
    fn value_tasks_and_plain_tasks_wrap_the_task() {
        assert!(
            wrapped("ValueTask<Greeting>")
                .starts_with("System.Threading.Tasks.ValueTask<Greeting>(task {")
        );
        assert_eq!(
            wrapped("ValueTask<BehaviorJSON>"),
            "System.Threading.Tasks.ValueTask<BehaviorJSON>(this.behaviors.Invoke())"
        );
        assert_eq!(
            wrapped("Task"),
            "(task { let! _ = this.behaviors.Invoke() in return () } :> System.Threading.Tasks.Task)"
        );
        assert!(wrapped("ValueTask").starts_with("System.Threading.Tasks.ValueTask((task {"));
    }

    #[test]
    fn declared_exchanges_answer_json_for_the_csharp_check() {
        let shadow = "    public async Task<Greeting> read() { } }\n    public async Task<Greeting> Ask() { \n    public ValueTask go(int a) { }\n    public Task touch() { } }";
        assert_eq!(
            FsharpAnswer::exchange_text(shadow),
            "    public async Task<BehaviorJSON> read() { } }\n    public async Task<Greeting> Ask() { \n    public async Task<BehaviorJSON> go(int a) { }\n    public async Task<BehaviorJSON> touch() { } }"
        );
    }

    #[test]
    fn an_unreadable_answer_names_its_type() {
        let refusal = FsharpAnswer::wrapped("Task<Map<string, int>>", ANSWER)
            .err()
            .unwrap_or_default();
        assert!(refusal.contains("Map<string, int>"), "{refusal}");
    }
}
