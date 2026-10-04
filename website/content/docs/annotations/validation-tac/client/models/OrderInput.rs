#[Checked]
pub struct OrderInput {
    #[NotEmpty]
    #[Length({"min":1,"max":80})]
    pub name: String,
    #[Required]
    #[Min(1)]
    #[Max(9)]
    pub level: f64,
}
