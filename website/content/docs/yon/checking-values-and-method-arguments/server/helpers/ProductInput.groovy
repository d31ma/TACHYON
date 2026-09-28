@Checked
class ProductInput {
    @NotEmpty
    @Length({"min":1,"max":80})
    String name

    @Required
    @Min(1)
    @Max(9)
    Object level
}
