pub struct BehaviorTypedAnswer;

impl BehaviorTypedAnswer {
    const RESPONSE: &'static str = "HTTP_RESPONSE";
    const WRONG_SHAPE: &'static str = "The server answer does not have the shape of the model.";

    pub fn object<T: TacModel>(value: BehaviorValue) -> BehaviorResult<T> {
        Self::shaped(value, Self::is_object)
    }

    pub fn list<T: TacModel>(value: BehaviorValue) -> BehaviorResult<T> {
        Self::shaped(value, Self::is_list)
    }

    pub fn plain<T: TacModel>(value: BehaviorValue) -> BehaviorResult<T> {
        Self::shaped(value, Self::is_plain)
    }

    pub fn optional<T: TacModel>(value: BehaviorValue) -> BehaviorResult<T> {
        Self::shaped(value, Self::is_anything)
    }

    fn shaped<T: TacModel>(value: BehaviorValue, fits: fn(&BehaviorValue) -> bool) -> BehaviorResult<T> {
        fits(&value)
            .then(|| T::read(&Self::tac(value)))
            .ok_or_else(|| behavior_error(Self::RESPONSE, Self::WRONG_SHAPE))
    }

    fn is_object(value: &BehaviorValue) -> bool {
        matches!(value, BehaviorValue::Object(_))
    }

    fn is_list(value: &BehaviorValue) -> bool {
        matches!(value, BehaviorValue::Array(_))
    }

    fn is_plain(value: &BehaviorValue) -> bool {
        !matches!(value, BehaviorValue::Array(_) | BehaviorValue::Object(_))
    }

    fn is_anything(_value: &BehaviorValue) -> bool {
        true
    }

    fn tac(value: BehaviorValue) -> TacValue {
        match value {
            BehaviorValue::Null => TacValue::Null,
            BehaviorValue::Bool(flag) => TacValue::Flag(flag),
            BehaviorValue::Number(number) => TacValue::Float(number),
            BehaviorValue::String(text) => TacValue::Text(text),
            BehaviorValue::Array(items) => TacValue::List(items.into_iter().map(Self::tac).collect()),
            BehaviorValue::Object(entries) => TacValue::Object(
                entries
                    .into_iter()
                    .map(|(key, item)| (key, Self::tac(item)))
                    .collect(),
            ),
        }
    }
}
