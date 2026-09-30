use crate::models::ShortDetail;
#[derive(Clone)]
pub struct Summary {
pub id: i64,
pub name: String,
pub detail: ShortDetail,
pub items: Vec<ShortDetail>,
pub kind: String,
}
