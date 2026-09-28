use serde_json::Value;

pub(super) struct SchemaChange;

impl SchemaChange {
    const ANY_OF: &'static str = "anyOf";
    const TYPE: &'static str = "type";
    const ITEMS: &'static str = "items";
    const NUMBER: &'static str = "number";
    const INTEGER: &'static str = "integer";
    const ARRAY: &'static str = "array";

    pub(super) fn accepts(wide: &Value, narrow: &Value) -> bool {
        let same = wide == narrow;
        let any = wide.as_object().is_some_and(serde_json::Map::is_empty);
        same | any
            | Self::a_member_accepts(wide, narrow)
            | Self::accepts_every_member(wide, narrow)
            | Self::number_takes_integer(wide, narrow)
            | Self::items_accept(wide, narrow)
    }

    fn a_member_accepts(wide: &Value, narrow: &Value) -> bool {
        Self::members(wide)
            .iter()
            .any(|member| Self::accepts(member, narrow))
    }

    fn accepts_every_member(wide: &Value, narrow: &Value) -> bool {
        let members = Self::members(narrow);
        !members.is_empty() & members.iter().all(|member| Self::accepts(wide, member))
    }

    fn number_takes_integer(wide: &Value, narrow: &Value) -> bool {
        (Self::kind(wide) == Some(Self::NUMBER)) & (Self::kind(narrow) == Some(Self::INTEGER))
    }

    fn items_accept(wide: &Value, narrow: &Value) -> bool {
        let both_lists =
            (Self::kind(wide) == Some(Self::ARRAY)) & (Self::kind(narrow) == Some(Self::ARRAY));
        both_lists && Self::accepts(&wide[Self::ITEMS], &narrow[Self::ITEMS])
    }

    fn members(schema: &Value) -> &[Value] {
        schema
            .get(Self::ANY_OF)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    fn kind(schema: &Value) -> Option<&str> {
        schema.get(Self::TYPE).and_then(Value::as_str)
    }
}
