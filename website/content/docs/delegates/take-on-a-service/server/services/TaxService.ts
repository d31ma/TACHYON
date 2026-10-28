@Service
export default class TaxService {
  rate(region: string): number {
    return 2;
  }

  total(count: number): number {
    return count * 10;
  }
}
