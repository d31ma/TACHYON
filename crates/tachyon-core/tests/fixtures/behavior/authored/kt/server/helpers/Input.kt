@Checked
class Input {
    @Required
    var id: Any? = null
    @NotEmpty
    @Length({"min":2,"max":8})
    var name: String? = null
    @Min(0)
    @Max(10)
    var price: Any? = null
}

