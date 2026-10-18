// @HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})
type PageProgramClient interface {
	// @GetRequest("/read")
	Read() (ProgramState, error)
	// @PostRequest("/increment")
	Increment() (ProgramState, error)
}
