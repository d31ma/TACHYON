// @Service
// @NeedsAll
type DocumentsService struct {
	// @Required
	Order []string
}

func (documents *DocumentsService) Guides() int {
	return len(documents.Order)
}
