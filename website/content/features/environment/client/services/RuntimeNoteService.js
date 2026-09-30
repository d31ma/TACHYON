@Service
export default class RuntimeNoteService {
  runtime = 'JavaScript, in the browser'

  /**
   * @param {boolean} development
   * @returns {boolean}
   */
  showsNotes(development) { return development }
}
