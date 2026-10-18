use super::{
    CppYonContainer, CppYonErrors, CppYonGraph, CppYonRuntime, CppYonStores, CppYonTopics,
};
use crate::Failure;
use crate::behavior::TargetSignature;
use crate::tac_cpp::{CppFile, CppMember, CppMemberKind, CppProgram, CppTypeText};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct CppYonGatewayUnit;

impl CppYonGatewayUnit {
    const START: &'static str = "#line 1 \"tachyon-yon-gateway.cpp\"\n";

    pub(crate) fn lower(
        root: &Path,
        unit: &Path,
        target: &str,
        methods: &BTreeMap<String, usize>,
    ) -> Result<String, Failure> {
        let models = CppProgram::server_models(root)?;
        let errors = CppYonErrors::types(root)?;
        let stores = CppYonStores::read(root, &models)?;
        let graph = CppYonGraph::targeted(root, unit, target, &models, &stores)?;
        Ok([
            CppYonRuntime::source(),
            models.model_source(),
            errors.type_source(),
            models.setup_source(),
            CppYonTopics::source(&models.topics()),
            CppYonContainer::declarations(&graph),
            graph.located(),
            CppYonContainer::definitions(&graph),
            Self::dispatcher(target, methods),
        ]
        .concat())
    }

    pub(crate) fn signatures(
        path: &Path,
        source: &str,
        class: &str,
        method: &str,
    ) -> Result<Vec<TargetSignature>, Failure> {
        let file = CppFile::read(path, source)?;
        let owned = [file.class.name.as_str()] == [class];
        Ok(file
            .class
            .members
            .iter()
            .filter(|member| [owned, member.kind == CppMemberKind::Method] == [true, true])
            .filter(|member| [member.name == method, member.public] == [true, true])
            .map(|member| Self::signature(source, member))
            .collect())
    }

    fn signature(source: &str, member: &CppMember) -> TargetSignature {
        TargetSignature {
            parameter_types: member
                .parameters
                .iter()
                .map(|parameter| CppTypeText::plain(&parameter.type_text))
                .collect(),
            result_type: member.plain_type(),
            is_static: member.is_static,
            has_body: source.as_bytes().get(member.terminator) == Some(&b'{'),
            has_supported_receiver: true,
        }
    }

    fn dispatcher(target: &str, methods: &BTreeMap<String, usize>) -> String {
        let branches: String = methods
            .iter()
            .map(|(method, arity)| Self::branch(method, *arity))
            .collect();
        format!(
            "{start}class TachyonYonRoutes {{\npublic:\n    static tachyon::yon::YonAnswer answer(const tachyon::yon::YonFrame& frame) {{\n        tachyon::yon::YonGatewayCall call(frame);\n        tachyon::yon::YonInvocation invocation;\n        {target}& target = tachyon::yon::YonNeeds::of<{target}>();\n        invocation.start();\n        std::string method = call.method();\n{branches}        return tachyon::yon::YonAnswer::empty(400);\n    }}\n}};\n\nint main() {{\n    return tachyon::yon::YonMain::run<TachyonYonRoutes>();\n}}\n",
            start = Self::START,
        )
    }

    fn branch(method: &str, arity: usize) -> String {
        let arguments: Vec<String> = (0..arity)
            .map(|index| format!("call.argument({index})"))
            .collect();
        format!(
            "        if (method == \"{method}\") {{\n            tachyon::json::JsonNode result = target.{method}({});\n            invocation.drain();\n            return tachyon::yon::YonGatewayCall::answer(result);\n        }}\n",
            arguments.join(", ")
        )
    }
}
