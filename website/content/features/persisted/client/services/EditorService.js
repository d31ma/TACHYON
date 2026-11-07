@Service
export default class EditorService {
  @SessionState
  draft = ''
  @LocalState
  theme = 'light'
}