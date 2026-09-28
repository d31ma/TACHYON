/** @import { CartTotal } from '/models/CartTotal.ts' */
@Service
export default class LiveTotalService {
  seen = 0;
  /** @param {CartTotal} changed */
  @OnEvent({"replay":true})
  changed(changed) { this.seen = changed.total; }
}