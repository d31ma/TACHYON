class ApplicationValues
  # @return [Integer]
  #[Setting("${yon.server.port:8080}")]
  attr_reader :port

  # @return [Integer]
  #[Setting({"path":"yon.catalog.size","type":"integer","default":5,"min":1,"max":100})]
  attr_reader :size

  # @return [String]
  #[Setting("${yon.redis.url:redis://127.0.0.1:6379}")]
  attr_reader :redis
end
