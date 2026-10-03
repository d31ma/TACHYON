@Checked
class ProductInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    var name: String? = null

    @Required
    @Min(1)
    @Max(9)
    var level: Any? = null
}
