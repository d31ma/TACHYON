// @Checked
type ProductInput struct {
	// @NotEmpty
	// @Length({"min":1,"max":80})
	name string

	// @Required
	// @Min(1)
	// @Max(9)
	level float64
}
