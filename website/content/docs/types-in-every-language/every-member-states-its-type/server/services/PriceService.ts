import type { Price } from '/models/Price.ts'

@Service
export class PriceService {
  static CURRENCY = 'EUR'

  priceFor(amount: number, count: number): Price {
    return { amount: amount * count, currency: PriceService.CURRENCY }
  }
}
