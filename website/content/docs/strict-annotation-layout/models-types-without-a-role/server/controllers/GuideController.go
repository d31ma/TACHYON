// @RequestRoute("/guide")
// @Public
type GuideController struct{}

// @GetRoute
func (controller *GuideController) GetGuide() Guide {
	return Guide{Title: "Models hold types", Pages: 3}
}
