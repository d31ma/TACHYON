import type { Tab } from '/models/Tab.ts'

@Service
export default class BasketService {
  count: number = 0
  label: string = ''
  private selected: string = ''
  tabs: Tab[] = [{ id: 'a', name: 'A' }]

  increment(): void {
    this.count += 1
  }

  select(_event: Event, id: string): void {
    this.selected = id
  }
}
