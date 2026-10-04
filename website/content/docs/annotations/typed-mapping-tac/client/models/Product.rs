use crate::models::Detail;
#[derive(Clone)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub detail: Detail,
    pub items: Vec<Detail>,
    pub secret: String,
}
