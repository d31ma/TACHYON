@Service
export default class CounterService {
  count = 0
  @SessionState
  visits = 0
  @LocalState
  theme = 'light'

  increment() { this.count += 1 }
}