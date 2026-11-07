[Checked]
public sealed class AnnotatedInput {
    [NotEmpty]
    [Length({"min":2,"max":8})]
    public BehaviorJSON name;
    [Required]
    [Min(1)]
    [Max(10)]
    public BehaviorJSON amount;
    [Length({"min":1,"max":2})]
    public BehaviorJSON optional;
    private AnnotatedInput() {
        throw new System.Exception("must not instantiate");
    }
}

