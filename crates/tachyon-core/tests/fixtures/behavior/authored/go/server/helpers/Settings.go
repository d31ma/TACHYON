// @Settings({"prefix":"YON_DATABASE_","defaults":{"port":5432,"enabled":false,"ratio":0.5}})
type Settings struct {
	host string
	// @Min(1)
	// @Max(65535)
	port int64
	enabled bool
	// @Min(0)
	// @Max(1)
	ratio float64
}
