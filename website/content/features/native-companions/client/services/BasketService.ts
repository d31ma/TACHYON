@Service
export default class BasketService {
  count: number = 6
  label: string = 'TypeScript'

  doubled(): number { return this.count * 2 }
}
