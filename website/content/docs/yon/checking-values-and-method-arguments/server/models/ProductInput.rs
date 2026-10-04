#[Checked]
pub struct ProductInput {
    #[NotEmpty]
    #[Length({"min":1,"max":80})]
    name: String,

    #[Required]
    #[Min(1)]
    #[Max(9)]
    level: f64,
}
