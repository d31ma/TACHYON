@Service
export default class GuideService {
  /** @type {Guide} */
  _current = { title: 'Models hold types', pages: 3 }

  /** @returns {string} */
  get title() {
    return this._current.title
  }

  /** @returns {number} */
  get pages() {
    return this._current.pages
  }
}
