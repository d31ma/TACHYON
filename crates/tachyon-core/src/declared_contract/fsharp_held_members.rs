use super::fsharp_contract::FsharpContract;
use super::fsharp_declaration::FsharpDeclaration;
use crate::fsharp_source::MemberLine;

pub(super) struct FsharpHeldMembers;

impl FsharpHeldMembers {
    pub(super) const HELD: [&'static str; 2] = ["streamrequest", "socketrequest"];
    pub(super) const SOCKET: [&'static str; 1] = ["socketrequest"];
    const ATTRIBUTE_OPEN: &'static str = "[<";
    const RECEIVED: &'static str = "()";
    const SEND: &'static str = "{indent}member _.Send(message: obj) : System.Threading.Tasks.Task = BehaviorBrowserSocket.Send(\"{owner}\", message)\n";

    pub(super) fn member_lines(
        code: &[String],
        declaration: &FsharpDeclaration,
        names: &[&str],
    ) -> Vec<usize> {
        declaration
            .body
            .iter()
            .copied()
            .filter(|index| FsharpContract::member_of(&code[*index]).is_marked_with(names))
            .filter_map(|marker| Self::member_after(code, declaration, marker))
            .collect()
    }

    fn member_after(
        code: &[String],
        declaration: &FsharpDeclaration,
        marker: usize,
    ) -> Option<usize> {
        declaration
            .body
            .iter()
            .copied()
            .filter(|index| *index > marker)
            .find(|index| !code[*index].trim_start().starts_with(Self::ATTRIBUTE_OPEN))
    }

    pub(super) fn concrete(line: &str) -> Option<String> {
        let ending = &line[line.trim_end().len()..];
        MemberLine::concrete(line.trim_end(), Self::RECEIVED).map(|text| format!("{text}{ending}"))
    }

    pub(super) fn send(owner: &str, indent: &str, line: &str) -> String {
        let joiner = ["\n", ""][usize::from(line.ends_with('\n'))];
        let text = Self::SEND
            .replace("{indent}", indent)
            .replace("{owner}", owner);
        format!("{joiner}{text}")
    }
}
