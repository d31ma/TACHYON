@Checked
export default class ProductInput {
  /** @type {string} */
  @NotEmpty
  @Length({"min":1,"max":80})
  name;

  /** @type {number} */
  @Required
  @Min(1)
  @Max(9)
  level;
}
