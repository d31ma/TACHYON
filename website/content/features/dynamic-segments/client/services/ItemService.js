@Service
export default class ItemService {
  /**
   * @param {string} path
   * @returns {string}
   */
  idFrom(path) { return path.slice(path.lastIndexOf('/') + 1) }
}
