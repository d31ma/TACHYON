@Checked
export default class OrderInput {
  @NotEmpty
  @Length({"min":1,"max":80})
  name!: string;
  @Required
  @Min(1)
  @Max(9)
  level!: number;
}
