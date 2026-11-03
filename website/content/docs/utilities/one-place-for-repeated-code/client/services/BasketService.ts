@Service
export default class BasketService {
  private total: number = 1250

  totalLabel(): string {
    return PriceUtils.label(this.total)
  }
}
