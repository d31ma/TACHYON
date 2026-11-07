const developmentOrigin = document.querySelector('meta[name="tachyon-development-origin"]')?.content || ''
const endpoint = `${developmentOrigin}/.tachyon/hot`
const diagnosticsEndpoint = `${developmentOrigin}/.tachyon/diagnostics`
const overlayId = 'tachyon-hot-diagnostics'
const errorPage = document.body.hasAttribute('data-tachyon-error-page')
let applying = Promise.resolve()
let restoreFocus = null

const element = (tag, text) => {
  const node = document.createElement(tag)
  if (text) node.textContent = text
  return node
}

const entryItem = (entry) => {
  const item = element('li')
  Object.assign(item.style, { margin: '0 0 14px' })
  item.append(element('strong', `${entry.code}: ${entry.message}`))
  for (const location of entry.locations || []) item.append(element('div', `at ${location}`))
  if (entry.help) item.append(element('div', `help: ${entry.help}`))
  return item
}

const clearDiagnostics = () => {
  const root = document.getElementById(overlayId)
  if (!root) return
  const focused = root.contains(document.activeElement)
  root.remove()
  if (focused && restoreFocus?.isConnected) restoreFocus.focus({ preventScroll: true })
  restoreFocus = null
}

const containFocus = (root, event) => {
  if (event.key === 'Escape') { clearDiagnostics(); return }
  if (event.key !== 'Tab') return
  const items = [...root.querySelectorAll('button, [href], [tabindex]:not([tabindex="-1"])')]
  const first = items[0]
  const last = items[items.length - 1]
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus() }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus() }
}

const keepFocus = (event) => {
  const root = document.getElementById(overlayId)
  if (root && !root.contains(event.target)) root.querySelector('button')?.focus({ preventScroll: true })
}

const dismissButton = () => {
  const button = element('button', 'Dismiss')
  button.type = 'button'
  Object.assign(button.style, {
    font: 'inherit', minHeight: '44px', minWidth: '44px', padding: '10px 18px', borderRadius: '6px', cursor: 'pointer',
    color: '#1f1113', background: '#fee2e2', border: '1px solid #fecaca'
  })
  button.addEventListener('click', clearDiagnostics)
  return button
}

const overlay = () => {
  const current = document.getElementById(`${overlayId}-list`)
  if (current) { current.replaceChildren(); return current }
  const root = element('aside')
  root.id = overlayId
  root.setAttribute('role', 'alertdialog')
  root.setAttribute('aria-modal', 'true')
  root.setAttribute('aria-labelledby', `${overlayId}-title`)
  root.setAttribute('aria-describedby', `${overlayId}-list`)
  Object.assign(root.style, {
    position: 'fixed', inset: '16px', zIndex: '2147483647', overflow: 'auto',
    padding: '18px', border: '1px solid #ef4444', borderRadius: '8px',
    color: '#fee2e2', background: '#1f1113', font: '13px/1.5 ui-monospace, monospace',
    whiteSpace: 'pre-wrap', boxShadow: '0 16px 48px #0008'
  })
  root.addEventListener('keydown', (event) => containFocus(root, event))
  const title = element('h2', 'Tachyon found a problem')
  title.id = `${overlayId}-title`
  Object.assign(title.style, { font: 'inherit', fontSize: '15px', fontWeight: '700', margin: '0 0 6px' })
  const note = element('p', 'The preview keeps running. Open pages update by themselves when the problem is fixed.')
  Object.assign(note.style, { margin: '0 0 14px' })
  const list = element('ul')
  list.id = `${overlayId}-list`
  Object.assign(list.style, { listStyle: 'none', margin: '14px 0 0', padding: '0' })
  const button = dismissButton()
  root.append(title, note, button, list)
  restoreFocus = document.activeElement
  document.body.append(root)
  button.focus({ preventScroll: true })
  return list
}

const showEntries = (entries) => {
  if (!entries.length) { clearDiagnostics(); return }
  overlay().append(...entries.map(entryItem))
  document.documentElement.dataset.tachyonHot = 'diagnostics'
}

const reportEntries = (report) => (report?.diagnostics || []).map((item) => ({
  code: item.code,
  message: item.message,
  locations: (item.spans || []).map((span) => span.file),
  help: item.help
}))

const currentEntries = async () => {
  const response = await fetch(diagnosticsEndpoint, { cache: 'no-store' })
  if (!response.ok) throw new Error(`Cannot read ${diagnosticsEndpoint}`)
  return response.json()
}

const showDiagnostics = async (report) => {
  const entries = await currentEntries().catch(() => reportEntries(report))
  showEntries(entries.length ? entries : reportEntries(report))
}

const replaceStyles = async (buildId) => {
  const links = [...document.querySelectorAll('link[rel~="stylesheet"][href]')]
    .filter((link) => new URL(link.href, location.href).origin === location.origin)
  await Promise.all(links.map((link) => new Promise((resolve, reject) => {
    const next = link.cloneNode()
    const url = new URL(link.href, location.href)
    url.searchParams.set('tachyon_hot', buildId || String(Date.now()))
    next.href = url.href
    next.addEventListener('load', () => { link.remove(); resolve() }, { once: true })
    next.addEventListener('error', () => { next.remove(); reject(new Error(`Cannot update ${url.pathname}`)) }, { once: true })
    link.after(next)
  })))
}

const updateIslands = async (message) => {
  const tac = globalThis.__tachyonTac
  if (!tac?.hotUpdate) throw new Error('The Tac hot-update runtime is unavailable.')
  await tac.hotUpdate(message.boundaries || [], message.build_id || String(message.sequence))
}

const apply = async (message) => {
  if (message.contract_version !== 1) { location.reload(); return }
  if (message.kind === 'diagnostics' && errorPage) { location.reload(); return }
  if (message.kind === 'diagnostics') { await showDiagnostics(message.diagnostics); return }
  clearDiagnostics()
  if (message.kind === 'css') await replaceStyles(message.build_id)
  else if (message.kind === 'island') await updateIslands(message)
  else { location.reload(); return }
  document.documentElement.dataset.tachyonHot = message.kind
  window.dispatchEvent(new CustomEvent('tachyon:hot-update', { detail: message }))
}

document.addEventListener('focusin', keepFocus)
if (!errorPage) currentEntries().then(showEntries).catch(() => {})

let connectionLost = false
const source = new EventSource(endpoint)
source.addEventListener('error', () => { connectionLost = true })
source.addEventListener('open', () => { if (connectionLost && errorPage) location.reload() })
source.addEventListener('hot', (event) => {
  let message
  try { message = JSON.parse(event.data) }
  catch { location.reload(); return }
  applying = applying.then(() => apply(message)).catch((error) => {
    console.error('[Tachyon hot update]', error)
    location.reload()
  })
})
