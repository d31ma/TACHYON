@Service
export default class RetentionService {
  /** @type {string} */
  static _ROW_ID = 'same-id'
  /** @type {number} */
  updates = 0
  /** @type {number} */
  selections = 0
  /** @type {string} */
  selected = ''
  /** @type {string} */
  draft = ''
  /** @type {string} */
  mode = 'before'
  /** @type {string} */
  slotName = 'title'
  /** @type {string} */
  connector = 'M 0 0 L 32 32'
  /** @type {boolean} */
  visible = true
  /** @type {GraphCard[]} */
  cards = [
    { id: 'card-plan', label: 'Plan', x: 16 },
    { id: 'card-build', label: 'Build', x: 224 },
    { id: 'card-ship', label: 'Ship', x: 432 },
  ]
  /** @type {Row[]} */
  rows = [{ id: RetentionService._ROW_ID, label: 'Original row' }]

  /** @returns {void} */
  update() {
    this.updates += 1
  }

  /** @param {Event} _event @param {string} id @returns {void} */
  choose(_event, id) {
    this.selections += 1
    this.selected = id
  }

  /** @returns {void} */
  replaceRows() {
    this.rows = [{ id: RetentionService._ROW_ID, label: 'Replacement row' }]
  }

  /** @returns {void} */
  changeObserved() {
    this.mode = 'after'
  }

  /** @returns {void} */
  changeSlot() {
    this.slotName = 'actions'
  }

  /** @returns {void} */
  hide() {
    this.visible = false
  }
}
