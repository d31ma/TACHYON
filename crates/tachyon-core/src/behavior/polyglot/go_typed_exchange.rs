use super::{Method, Parameter};

pub(super) struct GoTypedExchange;

impl GoTypedExchange {
    const TAC: &'static str = "tac";
    const RAW_ANSWER: &'static str = "BehaviorValue";
    const RESULTS_START: char = '(';
    const RESULTS_END: &'static str = ",error)";
    const PARAMETERS: [&'static str; 5] = ["BehaviorValue", "string", "int", "float64", "bool"];
    const EXCHANGE: &'static str = "httpExchange";
    const GENERATED: &'static str = "generated";

    pub(super) fn applies(method: &Method, environment: &str) -> bool {
        let answer = Self::answer(method);
        [
            environment == Self::TAC,
            Self::generated(method),
            answer.is_some_and(|answer| answer != Self::RAW_ANSWER),
        ] == [true; 3]
    }

    pub(super) fn validate(method: &Method) -> Result<(), String> {
        method
            .parameters
            .iter()
            .find(|parameter| Self::refused(parameter))
            .map_or(Ok(()), |parameter| {
                Err(format!(
                    "Go HTTPClient method {} takes {} as {}; use BehaviorValue, string, int, float64 or bool",
                    method.name, parameter.name, parameter.type_source
                ))
            })
    }

    pub(super) fn body(method: &Method, policy: &str, receiver: &str) -> String {
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let answer = Self::answer(method).unwrap_or(Self::RAW_ANSWER);
        format!(
            " __tachyon_value, __tachyon_error := {receiver}.behaviors.Invoke({policy}, []BehaviorValue{{{arguments}}}, func([]BehaviorValue) (BehaviorValue, error) {{ return map[string]any{{}}, nil }}); return BehaviorsAnswer[{answer}](__tachyon_value, __tachyon_error) "
        )
    }

    fn generated(method: &Method) -> bool {
        method
            .metadata
            .get(Self::EXCHANGE)
            .and_then(|exchange| exchange.get(Self::GENERATED))
            .is_some()
    }

    fn refused(parameter: &Parameter) -> bool {
        !Self::PARAMETERS.contains(&parameter.type_source.trim())
    }

    fn answer(method: &Method) -> Option<&str> {
        let results = method.return_type.trim();
        let compact = results.strip_prefix(Self::RESULTS_START)?;
        let (answer, rest) = compact.split_once(',')?;
        let ends = rest.replace(' ', "") == Self::RESULTS_END[1..];
        ends.then_some(answer.trim())
    }
}
