use crate::route_types::{DartValues, Shape, TypeText};

pub(super) struct DartModelShape;

impl DartModelShape {
    const EXTENSION: &'static str = "dart";
    const FIELD_VALUE: &'static str = "values.first";

    pub(super) fn field_value(type_name: Option<&str>, models: &[String]) -> String {
        Self::of(type_name, models).map_or_else(
            || super::convert(type_name, Self::FIELD_VALUE),
            |shape| DartValues::read(&shape, Self::FIELD_VALUE),
        )
    }

    pub(super) fn of(type_name: Option<&str>, models: &[String]) -> Option<Shape> {
        type_name.and_then(|type_name| Self::model_type_shape(type_name, models))
    }

    pub(super) fn bindable(type_name: Option<&str>, models: &[String]) -> bool {
        type_name.is_none_or(|type_name| {
            [
                super::json_type(type_name),
                super::is_model_result(type_name, models),
            ]
            .contains(&true)
        })
    }

    fn model_type_shape(type_name: &str, models: &[String]) -> Option<Shape> {
        super::is_model_result(type_name, models)
            .then(|| TypeText::shape(Self::EXTENSION, type_name).ok())
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::DartModelShape;

    fn models() -> Vec<String> {
        vec![String::from("Product")]
    }

    #[test]
    fn a_model_field_is_read_with_its_model_reader() {
        assert_eq!(
            DartModelShape::field_value(Some("Product"), &models()),
            "__yon_read_Product(values.first)"
        );
        assert_eq!(
            DartModelShape::field_value(Some("List<Product>"), &models()),
            "(values.first as List).map((item) => __yon_read_Product(item)).toList()"
        );
        assert_eq!(
            DartModelShape::field_value(Some("int"), &models()),
            "__TacDartBridge.integer(values.first)"
        );
    }

    #[test]
    fn json_values_models_and_lists_of_models_bind_but_collaborators_do_not() {
        for type_name in [None, Some("String"), Some("Product"), Some("List<Product>")] {
            assert!(
                DartModelShape::bindable(type_name, &models()),
                "{type_name:?}"
            );
        }
        assert!(!DartModelShape::bindable(Some("TallyService"), &models()));
    }
}
