type ApplicationValues struct {
	// @Setting({"path":"yon.database.port","type":"integer","default":5432,"min":1,"max":65535})
	database uint16
	// @Setting("${yon.database.label:primary}")
	label string
}
