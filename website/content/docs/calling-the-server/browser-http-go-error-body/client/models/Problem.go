type Problem struct {
	Title  string         `json:"title"`
	Status int            `json:"status"`
	Errors []ProblemEntry `json:"errors"`
}
