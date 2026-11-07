use crate::place::Place;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Duplicate {
    pub first: Place,
    pub second: Place,
    pub tokens: usize,
}
