use super::{FsharpType, mapping_shape::MappingShape};

pub(super) struct MappingRecord;

impl MappingRecord {
    pub(super) fn functions(name: &str, source: &str) -> Result<Vec<String>, String> {
        let fields = Self::fields(name, source)?;
        let reads = fields
            .iter()
            .map(|(field, kind)| Self::read(field, kind))
            .collect::<Result<Vec<_>, _>>()?;
        let answers = fields
            .iter()
            .map(|(field, kind)| Self::answer(field, kind))
            .collect::<Result<Vec<_>, _>>()?;
        let writes = fields
            .iter()
            .map(|(field, kind)| Self::write(field, kind))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(vec![
            format!(
                "read{name} (value: BehaviorJSON) : Models.{name} = {{ {} }}",
                reads.join("; ")
            ),
            format!(
                "write{name} (value: Models.{name}) : BehaviorJSON = BehaviorJSON.Object(dict [ {} ])",
                writes.join("; ")
            ),
            format!(
                "answer{name} (value: BehaviorJSON) : Models.{name} = value.Members |> ignore; {{ {} }}",
                answers.join("; ")
            ),
        ])
    }

    fn fields(name: &str, source: &str) -> Result<Vec<(String, String)>, String> {
        let start = source
            .find(&format!("type {name}"))
            .ok_or_else(|| format!("Cannot read F# mapping model {name}"))?;
        let body = source[start..]
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(inside, _)| inside)
            .ok_or_else(|| format!("F# mapping requires a record: {name}"))?;
        Ok(FsharpType::split_top_level(body, ";")
            .into_iter()
            .flat_map(|part| part.lines())
            .filter_map(Self::field)
            .collect())
    }

    fn field(field: &str) -> Option<(String, String)> {
        let (name, kind) = field
            .trim()
            .trim_start_matches("mutable ")
            .split_once(':')?;
        Some((name.trim().to_owned(), kind.trim().to_owned()))
    }

    fn read(field: &str, kind: &str) -> Result<String, String> {
        let value = format!("value.Get(\"{field}\")");
        let read = MappingShape::read(kind, &value, MappingShape::MAPPING_READER)?;
        Ok(format!("{field} = {read}"))
    }

    fn answer(field: &str, kind: &str) -> Result<String, String> {
        let wire = crate::companion::lower_first(field);
        let value = format!("value.Get(\"{wire}\")");
        let read = MappingShape::read(kind, &value, MappingShape::ANSWER_READER)?;
        Ok(format!("{field} = {read}"))
    }

    fn write(field: &str, kind: &str) -> Result<String, String> {
        let write = MappingShape::write(kind, &format!("value.{field}"))?;
        Ok(format!("\"{field}\", {write}"))
    }
}
