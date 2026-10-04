@Checked
struct OrderInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    var name: String = ""
    @Required
    @Min(1)
    @Max(9)
    var level: Double = 0
}
