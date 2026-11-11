@Service
export default class AlertsService {
  static serverKey: string = 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U';
  @Needs
  push!: TacPush;
  endpoint: string = '';
  problem: string = '';

  turnOn(): void {
    this.push.subscribe(AlertsService.serverKey);
  }

  turnOff(): void {
    this.push.unsubscribe();
  }

  @OnEvent
  subscribed(subscribed: PushSubscribed): void {
    this.endpoint = subscribed.endpoint;
  }

  @OnEvent
  unsubscribed(unsubscribed: PushUnsubscribed): void {
    this.endpoint = '';
  }

  @OnEvent
  failed(failed: BrowserCallFailed): void {
    this.problem = failed.message;
  }
}
