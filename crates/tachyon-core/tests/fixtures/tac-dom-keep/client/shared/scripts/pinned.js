class PinnedChanges {
  static renderedEvent = 'tachyon:rendered'
  static state = { renderedAtLoad: document.documentElement.dataset.tachyonRendered === 'client', applied: {} }

  static box() {
    return document.querySelector('.box')
  }

  static mark(name) {
    const box = PinnedChanges.box()
    if (!box) return
    box.setAttribute(`data-pinned-${name}`, 'yes')
    const child = document.createElement('p')
    child.id = `pinned-child-${name}`
    child.textContent = `added by pinned script (${name})`
    box.append(child)
    PinnedChanges.state.applied[name] = true
  }

  static markTitle() {
    const title = document.getElementById('title')
    if (!title) return
    title.setAttribute('data-pinned-e', 'yes')
    PinnedChanges.state.applied.e = true
  }

  static onFirstRender(task) {
    if (PinnedChanges.state.renderedAtLoad) { task(); return }
    document.addEventListener(PinnedChanges.renderedEvent, (event) => { if (event.detail.first) task() })
  }

  static watchBox() {
    if (PinnedChanges.box()) { PinnedChanges.mark('b'); return }
    const observer = new MutationObserver(() => {
      if (!PinnedChanges.box()) return
      observer.disconnect()
      PinnedChanges.mark('b')
    })
    observer.observe(document.body, { childList: true, subtree: true })
  }

  static watchRenderedMark() {
    const observer = new MutationObserver(() => {
      observer.disconnect()
      PinnedChanges.mark('c')
      PinnedChanges.markTitle()
    })
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-tachyon-rendered'] })
  }

  static start() {
    globalThis.__pinned = PinnedChanges.state
    PinnedChanges.onFirstRender(() => PinnedChanges.mark('a'))
    PinnedChanges.watchBox()
    if (PinnedChanges.state.renderedAtLoad) { PinnedChanges.mark('c'); PinnedChanges.markTitle() } else PinnedChanges.watchRenderedMark()
    setTimeout(() => PinnedChanges.mark('d'), 500)
  }
}

PinnedChanges.start()
