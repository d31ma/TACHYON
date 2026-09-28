@Service
export default class ItemService {
  /** @type {string | undefined} */
  id = location.pathname.split('/').pop()
}