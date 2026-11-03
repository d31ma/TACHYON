export default class PriceUtils {
  static withTax(cents: number): number {
    PriceUtils.requirePositive(cents)
    return Math.round(cents * 1.2)
  }

  private static requirePositive(cents: number): void {
    if (cents < 0) throw new Error('A price is never negative.')
  }
}
