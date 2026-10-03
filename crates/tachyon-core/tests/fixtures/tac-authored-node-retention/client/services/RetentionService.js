@Service()
export default class RetentionService {
  /** @type {string} */
  static _ROW_ID = 'same-id';
  /** @type {number} */
  updates = 0;
  /** @type {number} */
  selections = 0;
  /** @type {boolean} */
  selected = false;
  /** @type {string} */
  draft = '';
  /** @type {string} */
  mode = 'before';
  /** @type {string} */
  slotName = 'title';
  /** @type {string} */
  connector = 'M 0 0 L 32 32';
  /** @type {boolean} */
  visible = true;
  /** @type {{id: string, label: string}[]} */
  rows = [{ id: RetentionService._ROW_ID, label: 'Original row' }];

  /** @returns {void} */
  update() { this.#increment(); }
  /** @returns {void} */
  choose() { this.#select(); }
  /** @returns {void} */
  replaceRows() { this.#replace(); }
  /** @returns {void} */
  changeObserved() { this.#observe(); }
  /** @returns {void} */
  changeSlot() { this.#assignSlot(); }
  /** @returns {void} */
  hide() { this.#remove(); }

  /** @returns {void} */
  #increment() { this.updates += 1; }
  /** @returns {void} */
  #select() {
    this.selections += 1;
    this.selected = true;
  }
  /** @returns {void} */
  #replace() { this.rows = [{ id: RetentionService._ROW_ID, label: 'Replacement row' }]; }
  /** @returns {void} */
  #observe() { this.mode = 'after'; }
  /** @returns {void} */
  #assignSlot() { this.slotName = 'actions'; }
  /** @returns {void} */
  #remove() { this.visible = false; }
}
