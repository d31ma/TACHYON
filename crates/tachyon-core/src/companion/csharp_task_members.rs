use super::{
    CsharpModelType, CsharpTaskResult, Member, class_body_named, csharp_arguments,
    csharp_parameters, declared_service, identifier, lower_first, without_type_first_modifiers,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::route_types::ModelTable;

pub(crate) struct CsharpTaskMembers;

impl CsharpTaskMembers {
    pub(crate) fn entries(
        source: &str,
        path: &str,
        models: &ModelTable,
        excluded: &[String],
    ) -> Result<String, Failure> {
        Self::declared_entries(source, path, (models, excluded))
    }

    fn declared_entries(
        source: &str,
        path: &str,
        context: (&ModelTable, &[String]),
    ) -> Result<String, Failure> {
        let wanted = declared_service(path, source);
        let code = crate::annotation_layout::masked_source(std::path::Path::new(path), source)?;
        let body = class_body_named(&code, wanted.as_deref()).map_or("", |(_, body)| body);
        let mut entries = String::new();
        let mut depth = 0_i32;
        for line in body.lines() {
            let entering = depth;
            depth += Self::brace_change(line);
            Self::append(&mut entries, (entering, line), (&code, path), context)?;
        }
        Ok(entries)
    }

    fn brace_change(line: &str) -> i32 {
        Self::brace_count(line, '{') - Self::brace_count(line, '}')
    }

    fn brace_count(line: &str, brace: char) -> i32 {
        line.chars().filter(|character| *character == brace).count() as i32
    }

    fn append(
        entries: &mut String,
        line: (i32, &str),
        origin: (&str, &str),
        context: (&ModelTable, &[String]),
    ) -> Result<(), Failure> {
        for selected in (line.0 == 0).then_some(line.1).into_iter() {
            Self::append_member(entries, selected, origin, context)?;
        }
        Ok(())
    }

    fn append_member(
        entries: &mut String,
        line: &str,
        origin: (&str, &str),
        context: (&ModelTable, &[String]),
    ) -> Result<(), Failure> {
        for member in Self::member(line).into_iter() {
            Self::append_result(entries, &member, (origin, line), context)?;
        }
        Ok(())
    }

    fn append_result(
        entries: &mut String,
        member: &Member,
        origin: ((&str, &str), &str),
        context: (&ModelTable, &[String]),
    ) -> Result<(), Failure> {
        let kind = member.declared_type.as_deref().unwrap_or("");
        let model = CsharpTaskResult::model(kind, context.0)
            .map_err(|message| Self::failure(origin.0, origin.1, message))?;
        if model.is_some() && !context.1.contains(&member.name) {
            entries.push_str(&Self::entry(member));
        }
        Ok(())
    }

    fn member(line: &str) -> Option<Member> {
        let line = line.trim();
        line.strip_prefix("public ")?;
        let (signature, _) = without_type_first_modifiers(line);
        let open = signature.find('<')?;
        let (kind, rest) = CsharpModelType::declared(signature, open)?;
        Self::task_kind(&kind)?;
        let name = identifier(rest);
        let parameters = csharp_parameters(rest[name.len()..].trim_start())?;
        Some(Member {
            name,
            declared_type: Some(kind),
            method: true,
            mutable: false,
            parameters: Some(parameters),
            asynchronous: true,
        })
    }

    fn task_kind(kind: &str) -> Option<()> {
        let base = kind.split('<').next()?;
        [
            "Task",
            "System.Threading.Tasks.Task",
            "ValueTask",
            "System.Threading.Tasks.ValueTask",
        ]
        .contains(&base)
        .then_some(())
    }

    fn entry(member: &Member) -> String {
        let key = lower_first(&member.name);
        let arguments = csharp_arguments(member.parameters.as_deref().unwrap_or_default());
        format!(
            "        [\"{key}\"] = TacMember.TaskMethod(async arguments => TacModels.Plain(await Instance.{}({arguments}))),\n",
            member.name
        )
    }

    fn failure(origin: (&str, &str), line: &str, message: String) -> Failure {
        let at = origin.0.find(line).unwrap_or(0);
        Failure::one(diagnostic(
            1405,
            message,
            None,
            source_span(origin.1, at, at + line.len()),
        ))
    }
}
