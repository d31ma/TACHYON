export default class Endpoint {
  /** @type {string} */
  @Setting("${yon.active.profile:development}")
  profile;
}
