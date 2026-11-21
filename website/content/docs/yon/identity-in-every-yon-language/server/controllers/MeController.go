// @RequestRoute("/me")
// @SignedIn
type MeController struct {
}

// @GetRoute
func (controller *MeController) GetMe(
	// @FromIdentity
	caller Identity,
) string {
	return caller.User
}
