pub(super) struct RustTaskMember;

impl RustTaskMember {
    const FALLIBLE_FORMS: [(&'static str, &'static str); 2] =
        [("BehaviorResult<", ">"), ("Result<", ",BehaviorError>")];
    const NO_ANSWER: &'static str = "()";

    pub(super) fn crosses(returns: Option<&str>) -> bool {
        let (_, answer) = Self::answer(returns);
        [
            answer == Self::NO_ANSWER,
            super::RustModelType::crosses(&answer),
        ]
        .contains(&true)
    }

    pub(super) fn entry(
        key: &str,
        name: &str,
        returns: Option<&str>,
        parameters: &[String],
    ) -> String {
        let (bindings, passed) = super::rust_arguments(parameters);
        let (fallible, answer) = Self::answer(returns);
        let call = format!("service.{name}({passed}).await");
        let value = Self::value(fallible, &answer, &call);
        format!(
            "(\"{key}\", TacMember::Task({{let it = instance.clone(); Box::new(move |arguments: Vec<TacValue>| {{ #[allow(unused_mut, unused_variables)] let mut arguments = arguments.into_iter(); {bindings}let owner = it.clone(); Box::pin(async move {{ #[allow(unused_mut)] let mut service = owner.try_borrow_mut().map_err(|_| TacTaskError::busy())?; {value} }}) }})}})),"
        )
    }

    fn value(fallible: bool, answer: &str, call: &str) -> String {
        if fallible {
            return format!(
                "match {call} {{ Ok(value) => Ok({}), Err(error) => Err(TacTaskError {{ code: error.code, message: error.message }}) }}",
                super::rust_read(answer, "value")
            );
        }
        format!("Ok({})", super::rust_read(answer, call))
    }

    fn answer(returns: Option<&str>) -> (bool, String) {
        let compact = returns
            .unwrap_or(Self::NO_ANSWER)
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        Self::FALLIBLE_FORMS
            .iter()
            .find_map(|(start, end)| compact.strip_prefix(start)?.strip_suffix(end))
            .map_or_else(
                || (false, compact.clone()),
                |inner| (true, inner.to_owned()),
            )
    }
}
