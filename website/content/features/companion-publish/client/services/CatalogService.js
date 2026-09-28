/** @import { CatalogCount } from '/models/CatalogCount.ts' */
@Service
export default class CatalogService {
  count = 0;
  /** @param {CatalogCount} value */
  @OnEvent({"replay":true})
  receive(value) { this.count = value.count; }
}