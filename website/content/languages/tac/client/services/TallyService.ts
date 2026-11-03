@Service
export default class TallyService {
  count: number = 6
  runtime: string = 'TypeScript, everywhere'

  doubled(): number {
    return this.count * 2
  }
}