class TacMediaView extends (globalThis.HTMLElement ?? Object) {
  static tagName = 'media-view'
  static tracksKey = Symbol.for('tachyon.tac.media.tracks')
  static fits = ['cover', 'contain']
  static views = new Set()
  static hooked = false
  static style = ':host{display:block;position:relative;overflow:hidden}video{display:block;width:100%;height:100%;background:transparent}'
  static installed = TacMediaView.install()

  static get observedAttributes() {
    return ['track', 'fit', 'mirror', 'label']
  }

  static install() {
    if (!globalThis.customElements || customElements.get(TacMediaView.tagName)) return false
    customElements.define(TacMediaView.tagName, TacMediaView)
    return true
  }

  static hook() {
    const tracks = TacMediaView.tracks()
    if (TacMediaView.hooked || !tracks) return
    TacMediaView.hooked = true
    tracks.changed(() => TacMediaView.refreshAll())
    tracks.ended(() => TacMediaView.refreshAll())
  }

  static tracks() {
    return globalThis[TacMediaView.tracksKey] ?? null
  }

  static refreshAll() {
    for (const view of TacMediaView.views) view.refresh()
  }

  constructor() {
    super()
    const root = this.attachShadow({ mode: 'closed' })
    const style = document.createElement('style')
    style.textContent = TacMediaView.style
    this.video = document.createElement('video')
    this.video.muted = true
    this.video.autoplay = true
    this.video.playsInline = true
    this.video.setAttribute('aria-hidden', 'true')
    root.append(style, this.video)
  }

  connectedCallback() {
    TacMediaView.hook()
    TacMediaView.views.add(this)
    this.refresh()
  }

  disconnectedCallback() {
    TacMediaView.views.delete(this)
    this.video.srcObject = null
  }

  attributeChangedCallback() {
    this.refresh()
  }

  refresh() {
    this.showTrack(TacMediaView.tracks()?.track(this.getAttribute('track') ?? '') ?? null)
    this.video.style.objectFit = TacMediaView.fits.includes(this.getAttribute('fit')) ? this.getAttribute('fit') : 'cover'
    this.video.style.transform = this.getAttribute('mirror') === 'true' ? 'scaleX(-1)' : ''
    this.labelWith(this.getAttribute('label') ?? '')
  }

  showTrack(track) {
    const video = track?.kind === 'video' ? track : null
    if (this.video.srcObject?.getVideoTracks()[0] === video) return
    this.video.srcObject = video ? new MediaStream([video]) : null
  }

  labelWith(label) {
    if (label === '') return this.removeAttribute('aria-label')
    this.setAttribute('role', 'img')
    this.setAttribute('aria-label', label)
  }
}
