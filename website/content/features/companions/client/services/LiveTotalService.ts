import type { CartTotal } from '/models/CartTotal.ts'

@Service
export default class LiveTotalService {
  seen: number = 0
  @OnEvent({"replay":true})
  changed(changed: CartTotal): void { this.seen = changed.total }
}