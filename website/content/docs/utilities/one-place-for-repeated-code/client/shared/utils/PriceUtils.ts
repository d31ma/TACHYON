export default class PriceUtils {
  static label(cents: number): string {
    return `$${PriceUtils.dollars(cents)}`
  }

  private static dollars(cents: number): string {
    return (cents / 100).toFixed(2)
  }
}
