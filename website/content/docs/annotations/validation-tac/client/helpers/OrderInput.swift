@Checked
final class OrderInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    var name: BehaviorJSON = .null
    @Required
    @Min(1)
    @Max(9)
    var level: BehaviorJSON = .null
}
