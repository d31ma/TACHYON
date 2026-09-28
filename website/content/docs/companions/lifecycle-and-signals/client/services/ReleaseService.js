@Service
export default class ReleaseService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  release = '';
  announce() {
    /** @type {ReleaseChanged} */
    const changed = { release: 'TACHYON' };
    this.applicationEventPublisher.publishEvent(changed, true);
  }
  /** @param {ReleaseChanged} changed */
  @OnEvent({"replay":true})
  receive(changed) { this.release = changed.release; }
}
