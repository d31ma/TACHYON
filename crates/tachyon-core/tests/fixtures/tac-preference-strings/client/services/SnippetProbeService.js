/** @import { SnippetFile } from '/models/SnippetFile.ts' */
@Service
export default class SnippetProbeService {
  visible = false
  /** @type {SnippetFile[]} */
  files = [
    { name: 'client/services/PythonExample.py', code: 'Python snippet' },
    { name: 'client/services/KotlinExample.kt', code: 'Kotlin snippet' },
    { name: 'client/components/probe/tac.html', code: 'Shared view' },
  ]
}
