class TacCodeScanner {
  static key = Symbol.for('tachyon.tac.codeScanner')
  static decoderFile = 'tac-qr-decoder.js'
  static reasonMeta = 'meta[name="tachyon-camera-reason"]'
  static busy = 'A code scan is already open.'
  static label = 'Scan a QR code'
  static cancelLabel = 'Cancel'
  static readEvery = 150
  static widest = 640
  static constraints = Object.freeze({ audio: false, video: { facingMode: { ideal: 'environment' }, width: { ideal: 1280 } } })
  static reasons = Object.freeze({
    NotAllowedError: 'denied',
    SecurityError: 'denied',
    NotFoundError: 'unsupported',
    OverconstrainedError: 'unsupported',
  })
  static styles = [
    ':host{all:initial}',
    'dialog{position:fixed;inset:0;width:100%;height:100%;max-width:none;max-height:none;margin:0;padding:0;border:0;background:#000;color:#fff;overflow:hidden;font:16px/1.5 system-ui,-apple-system,sans-serif}',
    'dialog::backdrop{background:#000}',
    'video{position:absolute;inset:0;width:100%;height:100%;object-fit:cover}',
    '.square{position:absolute;top:50%;left:50%;width:min(64vmin,320px);aspect-ratio:1;transform:translate(-50%,-50%);border:3px solid #fff;border-radius:16px;box-shadow:0 0 0 100vmax rgba(0,0,0,.45)}',
    'p{position:absolute;top:calc(env(safe-area-inset-top,0px) + 24px);left:24px;right:24px;margin:0;text-align:center;font-weight:600;text-shadow:0 1px 3px #000}',
    'button{position:absolute;bottom:calc(env(safe-area-inset-bottom,0px) + 24px);left:50%;transform:translateX(-50%);min-width:120px;min-height:44px;padding:10px 24px;border:0;border-radius:22px;background:#fff;color:#000;font:inherit;font-weight:600;cursor:pointer}',
    'button:hover{background:#e6e6e6}',
    'button:focus-visible{outline:3px solid #4da3ff;outline-offset:3px}',
    '@media (prefers-reduced-motion:no-preference){button{transition:background-color .15s}}',
  ].join('')
  static cameras = null
  static scanning = null
  static installed = TacCodeScanner.install()

  static install() {
    Object.defineProperty(globalThis, TacCodeScanner.key, {
      configurable: true,
      value: Object.freeze({
        supported: () => TacCodeScanner.supported(),
        scan: () => TacCodeScanner.scan(),
        cancel: () => TacCodeScanner.cancel(),
      }),
    })
    TacCodeScanner.watchCameras()
    return true
  }

  static devices() {
    return globalThis.isSecureContext === true ? globalThis.navigator?.mediaDevices : undefined
  }

  static supported() {
    return typeof TacCodeScanner.devices()?.getUserMedia === 'function' && TacCodeScanner.cameras !== false
  }

  static watchCameras() {
    const devices = TacCodeScanner.devices()
    if (typeof devices?.enumerateDevices !== 'function') return
    devices.addEventListener?.('devicechange', TacCodeScanner.countCameras)
    TacCodeScanner.countCameras()
  }

  static countCameras() {
    TacCodeScanner.devices().enumerateDevices().then((list) => {
      TacCodeScanner.cameras = list.some((device) => device.kind === 'videoinput')
    }, () => {})
  }

  static scan() {
    if (TacCodeScanner.scanning) throw new Error(TacCodeScanner.busy)
    if (!TacCodeScanner.supported()) return TacCodeScanner.failure('unsupported')
    return new Promise((resolve) => TacCodeScanner.begin(resolve))
  }

  static cancel() {
    TacCodeScanner.end(TacCodeScanner.scanning, TacCodeScanner.failure('cancelled'))
    return null
  }

  static begin(resolve) {
    const scanning = { resolve, stream: null, timer: 0, reading: false, read: null, view: TacCodeScanner.view() }
    TacCodeScanner.scanning = scanning
    document.addEventListener('visibilitychange', TacCodeScanner.hidden)
    TacCodeScanner.devices().getUserMedia(TacCodeScanner.constraints).then(
      (stream) => TacCodeScanner.play(scanning, stream),
      (error) => TacCodeScanner.end(scanning, TacCodeScanner.failure(TacCodeScanner.reason(error))),
    )
  }

  static async play(scanning, stream) {
    if (TacCodeScanner.scanning !== scanning) {
      TacCodeScanner.stop(stream)
      return
    }
    scanning.stream = stream
    scanning.view.video.srcObject = stream
    scanning.view.video.play().catch(() => {})
    try {
      scanning.read = await TacCodeScanner.reader()
    } catch {
      TacCodeScanner.end(scanning, TacCodeScanner.failure('failed'))
      return
    }
    scanning.timer = setInterval(() => TacCodeScanner.tick(scanning), TacCodeScanner.readEvery)
  }

  static tick(scanning) {
    const video = scanning.view.video
    if (scanning.reading || video.readyState < 2 || video.videoWidth === 0) return
    scanning.reading = true
    TacCodeScanner.frame(scanning).then((text) => {
      scanning.reading = false
      if (text) TacCodeScanner.end(scanning, ['CodeScanned', { text }])
    }, () => {
      scanning.reading = false
    })
  }

  static frame(scanning) {
    const { video, canvas } = scanning.view
    const scale = Math.min(1, TacCodeScanner.widest / video.videoWidth)
    canvas.width = Math.round(video.videoWidth * scale)
    canvas.height = Math.round(video.videoHeight * scale)
    const context = canvas.getContext('2d', { willReadFrequently: true })
    context.drawImage(video, 0, 0, canvas.width, canvas.height)
    return Promise.resolve(scanning.read(canvas, context))
  }

  static async reader() {
    const detector = await TacCodeScanner.detector()
    if (detector) return (canvas) => detector.detect(canvas).then(TacCodeScanner.firstText)
    const decode = await TacCodeScanner.decoder()
    return (canvas, context) => TacCodeScanner.decoded(decode, context.getImageData(0, 0, canvas.width, canvas.height))
  }

  static async detector() {
    if (typeof globalThis.BarcodeDetector !== 'function') return null
    const formats = await Promise.resolve(globalThis.BarcodeDetector.getSupportedFormats?.()).catch(() => []) ?? []
    return formats.includes('qr_code') ? new globalThis.BarcodeDetector({ formats: ['qr_code'] }) : null
  }

  static async decoder() {
    await import(new URL(TacCodeScanner.decoderFile, import.meta.url).href)
    if (typeof globalThis.jsQR !== 'function') throw new Error('The QR decoder did not load.')
    return globalThis.jsQR
  }

  static firstText(codes) {
    return codes.find((code) => typeof code.rawValue === 'string' && code.rawValue !== '')?.rawValue ?? null
  }

  static decoded(decode, image) {
    return decode(image.data, image.width, image.height, { inversionAttempts: 'attemptBoth' })?.data || null
  }

  static hidden() {
    if (document.visibilityState === 'hidden') TacCodeScanner.cancel()
  }

  static reason(error) {
    return TacCodeScanner.reasons[error?.name] ?? 'failed'
  }

  static failure(reason) {
    return ['ScanFailed', { reason }]
  }

  static end(scanning, event) {
    if (!scanning || TacCodeScanner.scanning !== scanning) return
    TacCodeScanner.scanning = null
    clearInterval(scanning.timer)
    TacCodeScanner.stop(scanning.stream)
    document.removeEventListener('visibilitychange', TacCodeScanner.hidden)
    scanning.view.dialog.close()
    scanning.view.host.remove()
    scanning.resolve(event)
  }

  static stop(stream) {
    for (const track of stream?.getTracks() ?? []) track.stop()
  }

  static view() {
    const host = document.createElement('div')
    host.setAttribute('data-tachyon-code-scanner', '')
    const root = host.attachShadow({ mode: 'closed' })
    const style = document.createElement('style')
    style.textContent = TacCodeScanner.styles
    const dialog = TacCodeScanner.dialog()
    root.append(style, dialog)
    document.body.append(host)
    dialog.showModal()
    dialog.querySelector('button').focus()
    return { host, dialog, video: dialog.querySelector('video'), canvas: document.createElement('canvas') }
  }

  static dialog() {
    const reason = document.querySelector(TacCodeScanner.reasonMeta)?.content ?? ''
    const dialog = document.createElement('dialog')
    dialog.setAttribute('aria-label', reason || TacCodeScanner.label)
    dialog.addEventListener('cancel', (event) => {
      event.preventDefault()
      TacCodeScanner.cancel()
    })
    dialog.append(TacCodeScanner.video(), TacCodeScanner.square(), ...TacCodeScanner.text(reason), TacCodeScanner.button())
    return dialog
  }

  static video() {
    const video = document.createElement('video')
    video.muted = true
    video.playsInline = true
    video.setAttribute('playsinline', '')
    video.setAttribute('aria-hidden', 'true')
    return video
  }

  static square() {
    const square = document.createElement('div')
    square.className = 'square'
    square.setAttribute('aria-hidden', 'true')
    return square
  }

  static text(reason) {
    if (!reason) return []
    const text = document.createElement('p')
    text.textContent = reason
    return [text]
  }

  static button() {
    const button = document.createElement('button')
    button.type = 'button'
    button.textContent = TacCodeScanner.cancelLabel
    button.addEventListener('click', () => TacCodeScanner.cancel())
    return button
  }
}
