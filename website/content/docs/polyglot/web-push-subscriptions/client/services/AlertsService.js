@Service
export default class AlertsService {
  static serverKey = 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U';
  /** @type {TacPush} */
  @Needs
  push;
  endpoint = '';
  problem = '';

  turnOn() {
    this.push.subscribe(AlertsService.serverKey);
  }

  turnOff() {
    this.push.unsubscribe();
  }

  /** @param {PushSubscribed} subscribed */
  @OnEvent
  subscribed(subscribed) {
    this.endpoint = subscribed.endpoint;
  }

  /** @param {PushUnsubscribed} unsubscribed */
  @OnEvent
  unsubscribed(unsubscribed) {
    this.endpoint = '';
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
  }
}
