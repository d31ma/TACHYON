@Service
export default class BasketService {
  count: number = 6

  doubled(): number {
    return this.count * 2
  }

  @OnStart
  initialize(): void { document.title = 'Basket' }
}