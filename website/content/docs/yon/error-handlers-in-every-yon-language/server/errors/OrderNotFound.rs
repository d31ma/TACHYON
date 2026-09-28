#[derive(Debug)]
struct OrderNotFound;

impl std::fmt::Display for OrderNotFound {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "order not found")
    }
}

impl std::error::Error for OrderNotFound {}
