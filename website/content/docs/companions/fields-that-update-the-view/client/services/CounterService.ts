@Service
export default class CounterService {
  count: number = 0
  @SessionState
  visits: number = 0
  @LocalState
  theme: string = 'light'

  increment(): void { this.count += 1 }
}
