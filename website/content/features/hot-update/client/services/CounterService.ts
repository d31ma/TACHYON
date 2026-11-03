@Service
export default class CounterService {
  count: number = 0

  increment(): void { this.count += 1 }

  hotState(): { count: number } { return { count: this.count } }
  restoreHotState(state: { count: number }): void { this.count = state.count }
  hotDispose(): void {}
}
