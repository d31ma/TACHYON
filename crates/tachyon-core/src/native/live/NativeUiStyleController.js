class NativeUiStyleController {
  constructor() {
    this.ticket = null
    this.phase = 'idle'
    this.links = []
    this.document = document
  }

  async run(operation, ticket, styles, deadline) {
    const actions = {
      prepare: () => this._prepare(ticket, styles, deadline),
      commit: () => this._commit(ticket),
      rollback: () => this._rollback(ticket),
      finalize: () => this._finalize(ticket),
      status: () => this._status(ticket),
      invalidate: () => this._invalidate(ticket),
    }
    return await this._action(actions, operation)()
  }

  _action(actions, operation) {
    if (!Object.prototype.hasOwnProperty.call(actions, operation)) throw new Error('Unknown stylesheet operation')
    return actions[operation]
  }

  async _prepare(ticket, styles, deadline) {
    this._begin(ticket)
    try {
      this.links = this._links(styles)
      await Promise.all(this.links.map(link => link.prepare(deadline)))
      this._same(ticket)
      this._unchanged()
      this.phase = 'prepared'
      return this.phase
    } catch (error) {
      this._failedPrepare(ticket)
      throw error
    }
  }

  _failedPrepare(ticket) {
    if (this.ticket?.token === ticket.token && this.ticket.epoch === ticket.epoch) this._failedCurrent()
  }

  _failedCurrent() { this._restore(); this.phase = 'rolledback' }

  _commit(ticket) {
    this._same(ticket)
    return this._alreadyCommitted() ?? this._mount()
  }

  _alreadyCommitted() {
    if (this.phase === 'committed') return this.phase
  }

  _mount() {
    try {
      this._mounted()
      this.phase = 'committed'
      return this.phase
    } catch (error) {
      this._restore()
      this.phase = 'rolledback'
      throw error
    }
  }

  _mounted() {
    this._prepared()
    this._unchanged()
    this.links.forEach(link => link.commit())
  }

  _prepared() {
    if (this.phase !== 'prepared') throw new Error('Stylesheets are not prepared')
  }

  _rollback(ticket) {
    this._same(ticket)
    return this._alreadyRolledBack() ?? this._rollingBack()
  }

  _alreadyRolledBack() {
    if (this.phase === 'rolledback') return this.phase
  }

  _rollingBack() {
    this._rollbackAllowed()
    this._rollbackRetained()
    this._restore()
    this.phase = 'rolledback'
    return this.phase
  }

  _rollbackAllowed() {
    if (this.phase === 'finalized') throw new Error('Rollback generation has finalized')
  }

  _rollbackRetained() {
    if (this.phase === 'committed' && !this.links.every(link => link.retained()))
      throw new Error('Committed stylesheet nodes changed')
  }

  _finalize(ticket) {
    this._same(ticket)
    return this._alreadyFinalized() ?? this._finalizing()
  }

  _alreadyFinalized() {
    if (this.phase === 'finalized') return this.phase
  }

  _finalizing() {
    this._committed()
    this._retained()
    this.links.forEach(link => link.finalize())
    this.links = []
    this.phase = 'finalized'
    return this.phase
  }

  _committed() {
    if (this.phase !== 'committed') throw new Error('Previous style transaction cannot finalize')
  }

  _status(ticket) { this._same(ticket); return this.phase }

  _invalidate(ticket) { this._token(ticket); this._restore(); this._clear(); return this.phase }

  _begin(ticket) {
    this._preparable()
    this._document(ticket)
    this.ticket = ticket
    this.phase = 'preparing'
    this.links = []
  }

  _preparable() {
    if (!['idle', 'rolledback', 'finalized'].includes(this.phase)) throw new Error('Another stylesheet transaction is active')
  }

  _same(ticket) { this._token(ticket); this._document(this.ticket) }

  _token(ticket) {
    if (!this.ticket || this.ticket.token !== ticket.token || this.ticket.epoch !== ticket.epoch)
      throw new Error('Stale stylesheet transaction')
  }

  _document(ticket) {
    if (document !== this.document || location.href !== ticket.url) throw new Error('Stylesheet document changed')
  }

  _unchanged() {
    if (!this.links.every(link => link.valid())) throw new Error('Stylesheet nodes changed during preparation')
  }

  _retained() {
    if (!this.links.every(link => link.retained())) throw new Error('Committed stylesheet nodes changed')
  }

  _links(styles) {
    const originals = Array.from(document.querySelectorAll('link[rel~="stylesheet"]'))
    this._linkCount(originals)
    const references = originals.map(node => this._reference(node, styles))
    this._metadata(originals)
    return originals.map((node, at) => new NativeUiStyleLink(node, references[at]))
  }

  _linkCount(originals) {
    if (!originals.length || originals.length > 128) throw new Error('Stylesheet link bound exceeded')
  }

  _metadata(originals) {
    const metadata = originals.map(node => new NativeUiStyleAttributes(node))
    if (new TextEncoder().encode(JSON.stringify(metadata)).byteLength > 65536)
      throw new Error('Stylesheet metadata bound exceeded')
  }

  _reference(node, styles) {
    this._policy(node)
    const url = this._location(node)
    return this._captured(url, styles)
  }

  _policy(node) {
    if (node.rel.trim().toLowerCase() !== 'stylesheet' || node.hasAttribute('integrity'))
      throw new Error('Unsupported stylesheet policy')
  }

  _location(node) {
    const url = new URL(node.href, document.baseURI)
    if (url.protocol !== 'tachyon-app:' || url.hostname !== 'bundle' || url.port || url.username || url.password || url.search || url.hash)
      throw new Error('Stylesheet is outside captured local assets')
    return url
  }

  _captured(url, styles) {
    const match = Object.entries(styles).find(([path, value]) => url.href === value.previous || url.pathname === '/' + path)
    if (!match) throw new Error('Stylesheet is outside captured generation')
    return match[1].candidate
  }

  _restore() { this.links.forEach(link => link.rollback()) }

  _clear() { this.ticket = null; this.phase = 'idle'; this.links = [] }
}
