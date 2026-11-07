import type { CartTotal } from '/models/CartTotal.ts'

@Service
export default class CartService {
  applicationEventPublisher!: ApplicationEventPublisher
  total: number = 0
  add(): void {
    this.total += 1
    const changed: CartTotal = { total: this.total }
    this.applicationEventPublisher.publishEvent(changed, true)
  }
}
