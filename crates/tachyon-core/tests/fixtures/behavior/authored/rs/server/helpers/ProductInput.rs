#[Checked]
struct ProductInput {
    #[Required]
    #[NotEmpty]
    #[Length({"min":2,"max":20})]
    title: String,
    #[Min(1)]
    #[Max(100)]
    quantity: f64,
}

