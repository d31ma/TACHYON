@Service
export default class EditorService {
  @SessionState
  draft: string = ''
  @LocalState
  theme: string = 'light'
}
