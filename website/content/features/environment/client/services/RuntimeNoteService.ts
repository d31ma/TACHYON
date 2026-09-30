@Service
export default class RuntimeNoteService {
  runtime: string = 'TypeScript, in the browser'

  showsNotes(development: boolean): boolean { return development }
}
