@Service
export default class CounterService {
  count = 0

  increment() { this.count += 1 }

  /** @returns {{ count: number }} */
  hotState() { return { count: this.count } }
  /** @param {{ count: number }} state */
  restoreHotState(state) { this.count = state.count }
  hotDispose() {}
}
