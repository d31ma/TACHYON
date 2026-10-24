const developmentOrigin = document.querySelector('meta[name="tachyon-development-origin"]')?.content || ''
const endpoint = `${developmentOrigin}/.tachyon/hot`
const diagnosticsEndpoint = `${developmentOrigin}/.tachyon/diagnostics`
const overlayId = 'tachyon-hot-diagnostics'
const errorPage = document.body.hasAttribute('data-tachyon-error-page')
let applying = Promise.resolve()

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

const clearDiagnostics = () => document.getElementById(overlayId)?.remove()

const dismissButton = () => {
  const button = element('button', 'Dismiss')
  button.type = 'button'
  Object.assign(button.style, {
    font: 'inherit', padding: '6px 14px', borderRadius: '6px', cursor: 'pointer',
    color: '#1f1113', background: '#fee2e2', border: '1px solid #fecaca'
  })
  button.addEventListener('click', clearDiagnostics)
  return button
}

const overlay = () => {
  clearDiagnostics()
  const root = element('aside')
  root.id = overlayId
  root.setAttribute('role', 'alertdialog')
  root.setAttribute('aria-labelledby', `${overlayId}-title`)
  root.setAttribute('aria-describedby', `${overlayId}-list`)
  Object.assign(root.style, {
    position: 'fixed', inset: '16px', zIndex: '2147483647', overflow: 'auto',
    padding: '18px', border: '1px solid #ef4444', borderRadius: '8px',
    color: '#fee2e2', background: '#1f1113', font: '13px/1.5 ui-monospace, monospace',
    whiteSpace: 'pre-wrap', boxShadow: '0 16px 48px #0008'
  })
  root.addEventListener('keydown', (event) => { if (event.key === 'Escape') clearDiagnostics() })
  const title = element('h2', 'Tachyon found a problem')
  title.id = `${overlayId}-title`
  Object.assign(title.style, { font: 'inherit', fontSize: '15px', fontWeight: '700', margin: '0 0 6px' })
  const note = element('p', 'The preview keeps running. Open pages update by themselves when the problem is fixed.')
  Object.assign(note.style, { margin: '0 0 14px' })
  const list = element('ul')
  list.id = `${overlayId}-list`
  Object.assign(list.style, { listStyle: 'none', margin: '0 0 14px', padding: '0' })
  const button = dismissButton()
  root.append(title, note, list, button)
  document.body.append(root)
  button.focus()
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
