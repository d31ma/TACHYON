@Service
export default class SvgNamespaceService {
  /** @type {boolean} */
  visible = true
  /** @type {number[]} */
  points = [4, 8]
  /** @type {string} */
  mode = 'first'
  /** @type {string} */
  viewBox = '0 0 24 24'
  /** @type {string} */
  label = 'Initial'
  /** @type {string | null} */
  reference = '#moon'
  /** @type {string} */
  language = 'en'

  /** @param {{target: {value: string}}} event @returns {void} */
  edit(event) {
    this.label = event.target.value
    this.viewBox = '0 0 32 32'
    this.reference = '#gradient'
    this.language = 'fr'
  }
}
