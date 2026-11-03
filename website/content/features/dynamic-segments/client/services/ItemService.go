import "strings"

// @Service
type ItemService struct{}

func (item *ItemService) IdFrom(path string) string {
	return path[strings.LastIndex(path, "/")+1:]
}
