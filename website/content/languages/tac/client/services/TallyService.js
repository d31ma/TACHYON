@Service
export default class TallyService {
  count = 6
  runtime = 'JavaScript, everywhere'

  /** @returns {number} */
  doubled() {
    return this.count * 2
  }
}