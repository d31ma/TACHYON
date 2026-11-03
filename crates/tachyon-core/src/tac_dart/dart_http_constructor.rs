pub(super) struct DartHttpConstructor;

impl DartHttpConstructor {
    pub(super) fn required<'a>(
        parameters: &'a [String],
        declared: &[(String, Option<String>)],
    ) -> &'a [String] {
        let optional = Self::capture_parameter(parameters);
        let captured = Self::capture_type(declared);
        Self::choose(parameters, (optional, captured))
    }

    fn choose(parameters: &[String], shape: (bool, bool)) -> &[String] {
        match shape {
            (true, true) => &[],
            _ => parameters,
        }
    }

    fn capture_parameter(parameters: &[String]) -> bool {
        matches!(parameters, [parameter] if parameter == "[this._suppliedContext]")
    }

    fn capture_type(declared: &[(String, Option<String>)]) -> bool {
        declared.iter().any(Self::nullable_capture)
    }

    fn nullable_capture((field, kind): &(String, Option<String>)) -> bool {
        (field.as_str(), kind.as_deref()) == ("_suppliedContext", Some("BehaviorContext?"))
    }
}
