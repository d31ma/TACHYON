@Service
export default class BasketService {
  count = 6
  label = 'JavaScript'

  /** @returns {number} */
  doubled() { return this.count * 2 }
}