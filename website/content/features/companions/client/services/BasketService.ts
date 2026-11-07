@Service
export default class BasketService {
  count: number = 0

  doubled(): number {
    return this.count * 2
  }

  @OnStart
  initialize(): void { this.count = 6 }
}
