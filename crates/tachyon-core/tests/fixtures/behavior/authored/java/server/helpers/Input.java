@Checked
class Input {
    @Required
    Object id;
    @NotEmpty
    @Length({"min":2,"max":8})
    String name;
    @Min(0)
    @Max(10)
    Object price;
}

