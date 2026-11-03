use super::super::Method;

pub(super) struct JavaTypedClient<'a> {
    method: &'a Method,
}

impl<'a> JavaTypedClient<'a> {
    const TAC: &'static str = "tac";
    const EXCHANGE: &'static str = "httpExchange";
    const RAW_ANSWERS: [&'static str; 2] = ["Object", "java.lang.Object"];
    const NO_ANSWER: &'static str = "void";
    const LIST_STARTS: [&'static str; 2] = ["List<", "java.util.List<"];
    const PLAIN: [&'static str; 11] = [
        "String", "int", "long", "double", "float", "boolean", "Integer", "Long", "Double",
        "Float", "Boolean",
    ];
    const ANSWER: &'static str = "__ty_answer";
    const RETURN: &'static str = "return ";

    pub(super) fn body(
        method: &'a Method,
        environment: &str,
        body: &str,
    ) -> Result<String, String> {
        Self::typed(method, environment)
            .map_or_else(|| Ok(body.to_owned()), |typed| typed.rewritten(body))
    }

    pub(super) fn typed(method: &'a Method, environment: &str) -> Option<Self> {
        let declared = method.return_type.trim();
        let typed = [
            environment == Self::TAC,
            method.metadata.contains_key(Self::EXCHANGE),
            !Self::RAW_ANSWERS.contains(&declared),
        ] == [true; 3];
        typed.then_some(Self { method })
    }

    fn rewritten(&self, body: &str) -> Result<String, String> {
        let (prefix, returned) = body
            .rsplit_once(Self::RETURN)
            .ok_or_else(|| self.unreadable(self.method.return_type.trim()))?;
        let call = returned.trim_end().trim_end_matches(';');
        Ok(format!("{prefix}{}", self.returned(call)?))
    }

    fn returned(&self, call: &str) -> Result<String, String> {
        let declared = self.method.return_type.trim();
        if declared == Self::NO_ANSWER {
            return Ok(format!("{call};"));
        }
        self.answered(declared, call)
    }

    fn answered(&self, declared: &str, call: &str) -> Result<String, String> {
        let reading = crate::native::JavaWebServices::answer_reading(declared, Self::ANSWER)
            .ok_or_else(|| self.unreadable(declared))?;
        Ok(format!(
            "return Behaviors.answer({call}, \"{}\", {} -> {reading});",
            Self::shape(declared),
            Self::ANSWER
        ))
    }

    fn shape(declared: &str) -> &'static str {
        let listed = Self::LIST_STARTS
            .iter()
            .any(|start| declared.starts_with(start));
        match (listed, Self::PLAIN.contains(&declared)) {
            (true, _) => "list",
            (false, true) => "plain",
            (false, false) => "object",
        }
    }

    fn unreadable(&self, declared: &str) -> String {
        format!(
            "Java HTTPClient method {} returns {declared}. A Tac Java client method returns a model from client/models, a List of models, String, a number, boolean, Object or void.",
            self.method.name
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;

    struct TypedClientSource;

    impl TypedClientSource {
        const CLIENT: &'static str = "@HTTPClient({\"path\":\"/api\",\"setup\":\"GreetingSetup\"})\n@CacheSettings({\"name\":\"greetings\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\ninterface GreetingClient {\n    @Cached\n    @GetRequest(\"/greeting\")\n    Greeting read();\n    @GetRequest(\"/greetings\")\n    java.util.List<Greeting> all();\n    @GetRequest(\"/count\")\n    int count();\n    @GetRequest(\"/raw\")\n    Object raw();\n    @ClearCache({\"method\":\"read\"})\n    @PostRequest(\"/clear\")\n    void clear();\n}\n";

        fn lowered(environment: &str) -> String {
            crate::behavior::lower(
                Path::new("client/remotes/GreetingClient.java"),
                Self::CLIENT,
                environment,
            )
            .expect("Java client lowering")
            .expect("lowered client")
        }
    }

    #[test]
    fn a_tac_client_reads_each_answer_into_its_declared_type() {
        let lowered = TypedClientSource::lowered("tac");
        for expected in [
            "return Behaviors.answer(this.behaviors.invoke(\"GreetingClient\", \"read\"",
            "\"object\", __ty_answer -> TacModels.readGreeting(__ty_answer));",
            "\"list\", __ty_answer -> TacValues.list(__ty_answer, TacModels::readGreeting));",
            "\"plain\", __ty_answer -> TacValues.integer(__ty_answer));",
            "return this.behaviors.invoke(\"GreetingClient\", \"raw\"",
            "this.behaviors.mutate(\"GreetingClient\", \"read\"",
        ] {
            assert!(
                lowered.contains(expected),
                "missing {expected} in {lowered}"
            );
        }
        assert!(
            !lowered.contains("return this.behaviors.mutate("),
            "a void client method returns nothing: {lowered}"
        );
    }

    #[test]
    fn a_yon_client_still_needs_object_answers() {
        let failure = crate::behavior::lower(
            Path::new("server/remotes/GreetingClient.java"),
            TypedClientSource::CLIENT,
            "yon",
        )
        .expect_err("typed Yon client");
        assert!(
            failure.to_string().contains("must return Object"),
            "{failure}"
        );
    }
}
