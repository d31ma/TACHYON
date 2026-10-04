import "os"

// @RequestRoute("/api/region")
// @Public
type RegionController struct {
}

// @GetRoute
func (controller *RegionController) GetRegion() map[string]string {
	return map[string]string{
		"region": os.Getenv("HD_REGION"),
	}
}
