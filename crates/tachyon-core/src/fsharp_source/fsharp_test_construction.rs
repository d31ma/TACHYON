use super::{FieldNeeds, FsharpView};
use crate::Failure;
use crate::behavior::HttpClientSource;
use std::fmt::Write as _;

pub(crate) struct FsharpTestConstruction<'a> {
    unit: &'a str,
    fakes: Vec<(String, String)>,
    clients: Vec<String>,
}

impl<'a> FsharpTestConstruction<'a> {
    const NEEDS: &'static str = "Needs";
    const FAKE: &'static str = "Fake";
    const NO_TYPE: &'static str = "Test.construct needs a class with a declared Needs property.";
    const INDENT: &'static str = "        ";

    pub(crate) fn members(
        unit: &'a str,
        contents: &str,
        view: &FsharpView,
    ) -> Result<String, Failure> {
        let owners = FieldNeeds::settable_owners(contents, Self::NEEDS);
        let fakes = Self::fakes(unit, contents);
        let wanted = [owners.is_empty(), fakes.is_empty()] == [false, false];
        let written = wanted.then(|| Self::with_clients((unit, fakes), view, &owners));
        written.transpose().map(Option::unwrap_or_default)
    }

    fn with_clients(
        (unit, fakes): (&'a str, Vec<(String, String)>),
        view: &FsharpView,
        owners: &[(String, Vec<(String, String)>)],
    ) -> Result<String, Failure> {
        let construction = Self {
            unit,
            fakes,
            clients: Self::clients(view, owners)?,
        };
        Ok(construction.written(owners))
    }

    fn fakes(unit: &str, contents: &str) -> Vec<(String, String)> {
        FieldNeeds::marked_properties(contents, Self::FAKE)
            .into_iter()
            .filter(|(owner, _)| owner == unit)
            .flat_map(|(_, fakes)| fakes)
            .collect()
    }

    fn clients(
        view: &FsharpView,
        owners: &[(String, Vec<(String, String)>)],
    ) -> Result<Vec<String>, Failure> {
        let providers = owners
            .iter()
            .flat_map(|(_, needs)| needs.iter().map(|(_, provider)| provider));
        let mut clients = Vec::new();
        for provider in providers {
            let path = HttpClientSource::class_path(view.path(), view.text(), provider)
                .map_err(|failure| view.failure(&failure))?;
            clients.extend(path.map(|_| provider.clone()));
        }
        Ok(clients)
    }

    fn written(&self, owners: &[(String, Vec<(String, String)>)]) -> String {
        let mut members = format!(
            "\ntype {} with\n    member this.construct<'T>() : 'T =\n",
            self.unit
        );
        for (index, (owner, needs)) in owners.iter().enumerate() {
            let keyword = ["elif", "if"][usize::from(index == 0)];
            let _ = writeln!(
                members,
                "{}{keyword} typeof<'T> = typeof<{owner}> then\n{}    box ({owner}({})) :?> 'T",
                Self::INDENT,
                Self::INDENT,
                self.arguments(owner, needs)
            );
        }
        let _ = writeln!(
            members,
            "{}else\n{}    invalidOp {}",
            Self::INDENT,
            Self::INDENT,
            Self::quoted(Self::NO_TYPE)
        );
        members
    }

    fn arguments(&self, owner: &str, needs: &[(String, String)]) -> String {
        let values: Vec<String> = needs
            .iter()
            .map(|(name, provider)| format!("{name} = {}", self.value(owner, name, provider)))
            .collect();
        values.join(", ")
    }

    fn value(&self, owner: &str, name: &str, provider: &str) -> String {
        let matching = self
            .fakes
            .iter()
            .any(|(fake, kind)| [fake == name, kind == provider] == [true, true]);
        if !matching {
            let message =
                format!("Test.construct needs a matching typed Fake named {name} for {owner}.");
            return format!("(invalidOp {} : {provider})", Self::quoted(&message));
        }
        let empty = self.empty_fake(name, provider);
        format!("(if isNull (box this.{name}) then {empty} else this.{name})")
    }

    fn empty_fake(&self, name: &str, provider: &str) -> String {
        let client = self.clients.iter().any(|client| client == provider);
        let message = format!("Test.construct needs an assigned Fake named {name}.");
        let choices = [
            format!("invalidOp {}", Self::quoted(&message)),
            format!("{provider}(this.behaviors)"),
        ];
        choices[usize::from(client)].clone()
    }

    fn quoted(text: &str) -> String {
        serde_json::Value::String(text.to_owned()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpTestConstruction;
    use crate::fsharp_source::FsharpView;
    use std::path::Path;

    const SOURCE: &str = "[<HTTPClient({\"path\":\"/api\",\"setup\":\"GreetingSetup\"})>]\ntype GreetingClient =\n    [<GetRequest(\"/greeting\")>]\n    abstract read: unit -> Task<BehaviorJSON>\n\n[<Service>]\ntype CounterService() =\n    [<Needs>]\n    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set\n\n    [<Needs>]\n    member val Clock : Clock = Unchecked.defaultof<Clock> with get, set\n\n[<Test(typeof<HelloTestSetup>)>]\ntype CounterServiceTest() =\n    [<Fake>]\n    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set\n";

    fn members(source: &str) -> String {
        let path = Path::new("client/tests/CounterServiceTest.fs");
        let view = FsharpView::tac(path, source);
        FsharpTestConstruction::members("CounterServiceTest", source, &view)
            .unwrap_or_else(|failure| panic!("{failure}"))
    }

    #[test]
    fn an_empty_client_fake_is_bound_to_the_test_context() {
        let members = members(SOURCE);
        assert!(members.contains("type CounterServiceTest with\n    member this.construct<'T>() : 'T =\n        if typeof<'T> = typeof<CounterService> then\n"), "{members}");
        assert!(members.contains("GreetingClient = (if isNull (box this.GreetingClient) then GreetingClient(this.behaviors) else this.GreetingClient)"), "{members}");
    }

    #[test]
    fn a_need_without_a_matching_fake_is_refused_when_constructed() {
        let members = members(SOURCE);
        assert!(members.contains("Clock = (invalidOp \"Test.construct needs a matching typed Fake named Clock for CounterService.\" : Clock)"), "{members}");
        assert!(
            members.contains(
                "invalidOp \"Test.construct needs a class with a declared Needs property.\""
            ),
            "{members}"
        );
    }

    #[test]
    fn a_type_with_primary_constructor_parameters_gets_no_construct_branch() {
        let controller = "[<RequestRoute(\"/stock\")>]\ntype StockController(stock: StockService) =\n    [<Needs>]\n    member val Clock : Clock = Unchecked.defaultof<Clock> with get, set\n\n";
        let members = members(&format!("{controller}{SOURCE}"));
        assert!(!members.contains("StockController"), "{members}");
        assert!(members.contains("typeof<CounterService>"), "{members}");
    }

    #[test]
    fn a_unit_without_needs_gets_no_construct() {
        let plain = "[<Test(typeof<HelloTestSetup>)>]\ntype CounterServiceTest() =\n    member this.testOne() : unit = ()\n";
        assert_eq!(members(plain), "");
    }
}
