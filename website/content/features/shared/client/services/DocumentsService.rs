#[Service]
#[NeedsAll]
struct DocumentsService {
    order: Vec<String>,
}

impl DocumentsService {
    fn guides(&self) -> usize {
        self.order.len()
    }
}
