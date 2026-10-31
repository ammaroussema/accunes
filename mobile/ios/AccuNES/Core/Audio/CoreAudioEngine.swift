import AVFoundation

final class CoreAudioEngine {

    private let engine = AVAudioEngine()
    private var sourceNode: AVAudioSourceNode?
    private let sampleRate: Double = 44100.0

    private var audioQueue = [Float]()
    private let lock = NSLock()
    private let maxBufferCapacity = 16384

    init() {
        setupAudioSession()
        setupEngine()
    }

    private func setupAudioSession() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .gameChat, options: [.mixWithOthers])
        try? session.setPreferredSampleRate(sampleRate)
        try? session.setPreferredIOBufferDuration(0.005)
        try? session.setActive(true)
    }

    private func setupEngine() {
        let format = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: sampleRate,
            channels: 2,
            interleaved: true
        )!

        sourceNode = AVAudioSourceNode { [weak self] _, _, frameCount, audioBufferList -> OSStatus in
            guard let self = self else { return noErr }
            let abl = UnsafeMutableAudioBufferListPointer(audioBufferList)
            guard let buffer = abl.first, let ptr = buffer.mData?.assumingMemoryBound(to: Float.self) else {
                return noErr
            }

            let requestedSamples = Int(frameCount) * 2
            self.lock.lock()
            let available = self.audioQueue.count
            let toCopy = min(available, requestedSamples)

            if toCopy > 0 {
                for i in 0..<toCopy {
                    ptr[i] = self.audioQueue[i]
                }
                self.audioQueue.removeFirst(toCopy)
            }

            if toCopy < requestedSamples {
                for i in toCopy..<requestedSamples {
                    ptr[i] = 0.0
                }
            }
            self.lock.unlock()

            return noErr
        }

        if let source = sourceNode {
            engine.attach(source)
            engine.connect(source, to: engine.mainMixerNode, format: format)
        }

        try? engine.start()
    }

    func write(samples: [Float], count: Int) {
        if count <= 0 { return }
        lock.lock()
        if audioQueue.count + count > maxBufferCapacity {
            let overflow = (audioQueue.count + count) - maxBufferCapacity
            audioQueue.removeFirst(min(overflow, audioQueue.count))
        }
        audioQueue.append(contentsOf: samples.prefix(count))
        lock.unlock()
    }

    func pause() {
        engine.pause()
    }

    func resume() {
        try? engine.start()
    }

    func stop() {
        engine.stop()
        lock.lock()
        audioQueue.removeAll()
        lock.unlock()
    }
}
