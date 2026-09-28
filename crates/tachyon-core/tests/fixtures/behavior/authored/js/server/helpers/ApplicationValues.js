class ApplicationValues {
  @Setting("${tac.test.limit:5}")
  limitText;
  @Setting({"path":"tac.test.limit","type":"integer","default":5,"min":1,"max":100})
  limit;
  @Setting("${tac.test.missing:fallback}")
  missing;
}

