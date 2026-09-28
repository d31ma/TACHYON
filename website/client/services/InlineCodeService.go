import (
	"slices"
	"strings"
)

// @Service
type InlineCodeService struct{}

func (service *InlineCodeService) Parts(text string) []InlinePart {
	parts := service.partsOf(service.closedPieces(strings.Split(text, service.backtick())))
	return slices.DeleteFunc(parts, service.isEmpty)
}

func (service *InlineCodeService) closedPieces(pieces []string) []string {
	if len(pieces)%service.pairSize() == 1 {
		return pieces
	}
	last := pieces[len(pieces)-1]
	pieces = pieces[:len(pieces)-1]
	pieces[len(pieces)-1] += service.backtick() + last
	return pieces
}

func (service *InlineCodeService) partsOf(pieces []string) []InlinePart {
	parts := []InlinePart{}
	for index, piece := range pieces {
		parts = append(parts, InlinePart{Text: piece, Code: index%service.pairSize() == 1})
	}
	return parts
}

func (service *InlineCodeService) isEmpty(part InlinePart) bool {
	return part.Text == ""
}

func (InlineCodeService) backtick() string { return "`" }

func (InlineCodeService) pairSize() int { return 2 }
