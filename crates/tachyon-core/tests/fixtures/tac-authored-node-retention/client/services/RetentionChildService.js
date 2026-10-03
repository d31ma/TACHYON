@Service()
@Lifetime({"value":"prototype"})
export default class RetentionChildService {
  /** @type {number} */
  starts = 0;
  /** @type {number} */
  updates = 0;

  /** @returns {void} */
  @OnStart()
  start() { this.#start(); }
  /** @returns {void} */
  update() { this.#update(); }

  /** @returns {void} */
  #start() { this.starts += 1; }
  /** @returns {void} */
  #update() { this.updates += 1; }
}
