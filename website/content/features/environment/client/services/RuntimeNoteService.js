@Service
export default class RuntimeNoteService {
  /** @type {boolean} */
  development = document.body.dataset.tachyonEnvironment === 'development'
  runtime = 'JavaScript, in the browser'
}