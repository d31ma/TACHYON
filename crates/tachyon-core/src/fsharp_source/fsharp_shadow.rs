use super::{FsharpLexer, ShadowWriter};

pub(crate) struct FsharpShadow {
    text: String,
    shadow_starts: Vec<usize>,
    origins: Vec<usize>,
    source_starts: Vec<usize>,
    source_lengths: Vec<usize>,
}

impl FsharpShadow {
    pub(crate) fn of(source: &str) -> Self {
        let plain = FsharpLexer::without_comments(source);
        let code = FsharpLexer::code_mask(&plain).code;
        let written = ShadowWriter::write(&plain, &code);
        let (source_starts, source_lengths) = Self::source_lines(source);
        let (text, shadow_starts, origins) = Self::joined(written);
        Self {
            text,
            shadow_starts,
            origins,
            source_starts,
            source_lengths,
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn source_offset(&self, shadow_offset: usize) -> usize {
        let line = self
            .shadow_starts
            .partition_point(|start| *start <= shadow_offset)
            .saturating_sub(1);
        let column = shadow_offset - self.shadow_starts[line];
        let origin = self.origins[line];
        self.source_starts[origin] + column.min(self.source_lengths[origin])
    }

    fn source_lines(source: &str) -> (Vec<usize>, Vec<usize>) {
        let mut starts = Vec::new();
        let mut lengths = Vec::new();
        let mut start = 0;
        for line in source.split('\n') {
            starts.push(start);
            lengths.push(line.len());
            start += line.len() + 1;
        }
        (starts, lengths)
    }

    fn joined(written: Vec<(String, usize)>) -> (String, Vec<usize>, Vec<usize>) {
        let mut text = String::new();
        let mut starts = Vec::with_capacity(written.len());
        let mut origins = Vec::with_capacity(written.len());
        for (line, origin) in written {
            starts.push(text.len());
            origins.push(origin);
            text.push_str(&line);
            text.push('\n');
        }
        (text, starts, origins)
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpShadow;

    const CONTROLLER: &str = "open Models\n\n[<RequestRoute(\"/orders\")>]\n[<Public>]\ntype OrdersController(orders: OrderService) =\n    [<GetRoute>]\n    member this.GetOrders() : Order list =\n        orders.All()\n\n    [<PostRoute>]\n    member this.PostOrder([<FromBody>] order: Order) : Order = order\n";

    #[test]
    fn a_controller_reads_as_a_csharp_controller() {
        let shadow = FsharpShadow::of(CONTROLLER);
        let lines: Vec<&str> = shadow.text().lines().collect();
        assert_eq!(
            lines[..],
            [
                "using Models;",
                "",
                "[RequestRoute(\"/orders\")]",
                "[Public]",
                "[Needs(OrderService)]",
                "public class OrdersController { public OrdersController(OrderService orders) { }",
                "    [GetRoute]",
                "    public FSharpList<Order> GetOrders() { ",
                "        orders.All() }",
                "",
                "    [PostRoute]",
                "    public Order PostOrder([FromBody] Order order) {  order } }",
                "",
            ]
        );
    }

    #[test]
    fn shadow_offsets_map_back_to_the_same_fsharp_line() {
        let shadow = FsharpShadow::of(CONTROLLER);
        let at = shadow
            .text()
            .find("PostOrder")
            .unwrap_or_else(|| panic!("method"));
        let source = shadow.source_offset(at);
        let line_start = CONTROLLER[..source].rfind('\n').map_or(0, |at| at + 1);
        assert!(CONTROLLER[line_start..].starts_with("    member this.PostOrder"));
        let needs = shadow
            .text()
            .find("[Needs")
            .unwrap_or_else(|| panic!("needs"));
        let source = shadow.source_offset(needs);
        assert!(CONTROLLER[source..].starts_with("type OrdersController"));
    }

    #[test]
    fn records_unions_and_services_have_csharp_shapes() {
        let source = "type Order =\n    { Id: int\n      Total: float }\n\ntype Status =\n    | Open\n    | Closed\n\n[<Service>]\ntype TallyService() =\n    member val Count = 6 with get, set\n    member this.Doubled() : int = this.Count * 2\n";
        let shadow = FsharpShadow::of(source);
        let lines: Vec<&str> = shadow.text().lines().collect();
        assert_eq!(
            lines[..],
            [
                "public class Order {",
                "    public int Id { get; init; }",
                "      public double Total { get; init; } }",
                "",
                "public enum Status {",
                "    Open,",
                "    Closed, }",
                "",
                "[Service]",
                "public class TallyService {",
                "    public int Count { get; set; } = 6;",
                "    public int Doubled() {  this.Count * 2 } }",
                "",
            ]
        );
    }
}
