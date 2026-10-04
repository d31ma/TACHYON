export default class Endpoint {
  /** @type {number} */
  @Setting("${yon.server.port}")
  port;
  /** @type {string} */
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis;
}
