@Checked
class OrderInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    var name: Any? = null
    @Required
    @Min(1)
    @Max(9)
    var level: Any? = null
}
