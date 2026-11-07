@Delegate
export default class TaxDelegate {
  @Needs
  taxService!: TaxService;

  rate(region: string): number {
    return this.taxService.rate(region) * 2;
  }

  total(count: number, extra: number): number {
    return this.taxService.total(count) + extra;
  }
}
