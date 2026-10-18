use super::{Input, Registration};

pub(super) struct OwnContext;

impl OwnContext {
    const ASSIGNED: &'static str = "this.behaviors";
    const EXPORT: &'static str = "\nexport const __tachyon_own_context__ = (owner, signal) => Behaviors.expectOwnContext(owner, signal);\n";

    pub(super) fn append_export(
        input: &Input<'_>,
        registrations: &[Registration],
        output: &mut String,
    ) {
        let opens = registrations
            .iter()
            .find(|registration| super::default_export(input, registration))
            .is_some_and(|registration| Self::opened_by(input, registration));
        if opens {
            output.push_str(Self::EXPORT);
        }
    }

    pub(super) fn opened_by(input: &Input<'_>, registration: &Registration) -> bool {
        let body = &input.code[registration.open + 1..registration.close];
        body.match_indices(Self::ASSIGNED).any(|(at, found)| {
            let rest = body[at + found.len()..].trim_start();
            rest.starts_with('=') && !rest.starts_with("==")
        })
    }
}
