[Checked]
class ProductInput {
    [NotEmpty]
    [Length({"min":1,"max":80})]
    public BehaviorJSON name;

    [Required]
    [Min(1)]
    [Max(9)]
    public BehaviorJSON level;
}
