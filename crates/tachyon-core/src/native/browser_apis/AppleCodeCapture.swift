
final class AppleCodeCapture: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate, @unchecked Sendable {
    private let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "dev.tachyon.code-scanner")
    private let found: @Sendable (String) -> Void
    private var done = false

    init?(_ device: AVCaptureDevice, found: @escaping @Sendable (String) -> Void) {
        self.found = found
        super.init()
        guard let input = try? AVCaptureDeviceInput(device: device), session.canAddInput(input) else { return nil }
        let output = AVCaptureVideoDataOutput()
        output.alwaysDiscardsLateVideoFrames = true
        output.setSampleBufferDelegate(self, queue: queue)
        guard session.canAddOutput(output) else { return nil }
        session.addInput(input)
        session.addOutput(output)
    }

    func preview() -> AVCaptureVideoPreviewLayer {
        let layer = AVCaptureVideoPreviewLayer(session: session)
        layer.videoGravity = .resizeAspectFill
        return layer
    }

    func start() {
        queue.async { self.session.startRunning() }
    }

    func stop() {
        queue.async {
            self.done = true
            self.session.stopRunning()
        }
    }

    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer, from connection: AVCaptureConnection) {
        guard !done, let text = Self.text(sampleBuffer) else { return }
        done = true
        found(text)
    }

    private static func text(_ sampleBuffer: CMSampleBuffer) -> String? {
        let request = VNDetectBarcodesRequest()
        request.symbologies = [.qr]
        let handler = VNImageRequestHandler(cmSampleBuffer: sampleBuffer, options: [:])
        try? handler.perform([request])
        return request.results?.lazy.compactMap { $0.payloadStringValue }.first
    }
}
