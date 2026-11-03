@Checked
class ProductInput {
  @Required
  id;
  @NotEmpty
  @Length({"min":2,"max":8})
  name;
  @Min(0)
  @Max(10)
  price;
}

