@Service
export default class BasketService {
  count = 0
  label = ''
  _selected = ''
  /** @type {{ id: string, name: string }[]} */
  tabs = [{ id: 'a', name: 'A' }]

  increment() { this.count += 1 }

  /**
   * @param {Event} _event
   * @param {string} id
   */
  select(_event, id) { this._selected = id }
}