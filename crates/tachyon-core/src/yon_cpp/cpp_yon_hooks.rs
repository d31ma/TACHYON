use super::CppYonController;
use crate::Failure;
use crate::tac_cpp::{CppClass, CppFailure, CppListener, CppMember, CppMemberKind};
use std::path::Path;

pub(crate) struct CppYonHooks {
    pub(crate) listeners: Vec<CppListener>,
    pub(crate) starts: Vec<String>,
    pub(crate) stops: Vec<String>,
}

impl CppYonHooks {
    const LISTENER: &'static str = "OnEvent";
    const START: &'static str = "OnStart";
    const STOP: &'static str = "OnStop";
    const SYNCHRONOUS: &'static str = "A Yon C++ [[OnEvent]], [[OnStart]] or [[OnStop]] method runs synchronously, so it returns no Task: return void, or a model to publish it again";
    const NO_PARAMETERS: &'static str =
        "An [[OnStart]] or [[OnStop]] method takes no parameters, for example `void start()`";

    pub(crate) fn read(path: &Path, class: &CppClass) -> Result<Self, Failure> {
        class
            .members
            .iter()
            .try_for_each(|member| Self::check(path, member))?;
        let listeners = class
            .members
            .iter()
            .filter(|member| member.marked(Self::LISTENER))
            .map(|member| CppListener::read(path, member))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            listeners,
            starts: Self::marked(class, Self::START),
            stops: Self::marked(class, Self::STOP),
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        [
            self.listeners.is_empty(),
            self.starts.is_empty(),
            self.stops.is_empty(),
        ] == [true, true, true]
    }

    fn marked(class: &CppClass, marker: &str) -> Vec<String> {
        class
            .members
            .iter()
            .filter(|member| {
                [member.kind == CppMemberKind::Method, member.marked(marker)] == [true, true]
            })
            .map(|member| member.name.clone())
            .collect()
    }

    fn check(path: &Path, member: &CppMember) -> Result<(), Failure> {
        let lifecycle = [member.marked(Self::START), member.marked(Self::STOP)].contains(&true);
        let hooked = [lifecycle, member.marked(Self::LISTENER)].contains(&true);
        let problems = [
            (
                [hooked, member.is_task()] == [true, true],
                Self::SYNCHRONOUS,
            ),
            (
                [lifecycle, !member.parameters.is_empty()] == [true, true],
                Self::NO_PARAMETERS,
            ),
        ];
        let problem = problems.into_iter().find(|(found, _)| *found);
        problem.map_or(Ok(()), |(_, message)| {
            Err(CppFailure::coded(
                CppYonController::CODE,
                path,
                member.name_at,
                message,
            ))
        })
    }
}
