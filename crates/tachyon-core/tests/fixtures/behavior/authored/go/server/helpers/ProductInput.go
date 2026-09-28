// @Checked
type ProductInput struct {
	// @Required
	// @NotEmpty
	// @Length({"min":2,"max":20})
	title string
	// @Min(1)
	// @Max(100)
	quantity float64
}
