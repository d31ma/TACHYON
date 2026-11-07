class Endpoint
  # @return [Integer]
  #[Setting("${yon.server.port}")]
  attr_reader :port

  # @return [String]
  #[Setting("${yon.redis.url:redis://127.0.0.1:6379}")]
  attr_reader :redis
end
