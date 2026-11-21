import Foundation

final class EmulationLoop {

    private let bridge: AccuNESBridge
    private let renderer: MetalRenderer
    private let audioEngine: CoreAudioEngine
    private let controllerState: TouchControllerState

    private var thread: Thread?
    private var isRunning = false
    private var isPaused = false

    var isFastForward: Bool = false
    var physicalButtons: UInt8 = 0

    private var audioScratch = [Float](repeating: 0, count: 4096)

    init(
        bridge: AccuNESBridge,
        renderer: MetalRenderer,
        audioEngine: CoreAudioEngine,
        controllerState: TouchControllerState
    ) {
        self.bridge = bridge
        self.renderer = renderer
        self.audioEngine = audioEngine
        self.controllerState = controllerState
    }

    func start() {
        guard !isRunning else { return }
        isRunning = true
        isPaused = false

        thread = Thread { [weak self] in
            self?.runLoop()
        }
        thread?.qualityOfService = .userInteractive
        thread?.name = "AccuNES-Core-iOS"
        thread?.start()
    }

    func pause() {
        isPaused = true
        audioEngine.pause()
    }

    func resume() {
        isPaused = false
        audioEngine.resume()
    }

    func stop() {
        isRunning = false
        isPaused = false
        thread?.cancel()
        thread = nil
        audioEngine.stop()
    }

    private func runLoop() {
        let frameDuration: UInt64 = 1_000_000_000 / 60
        var nextFrameTime = DispatchTime.now().uptimeNanoseconds

        while isRunning && !Thread.current.isCancelled {
            if isPaused {
                usleep(16000)
                nextFrameTime = DispatchTime.now().uptimeNanoseconds
                continue
            }

            let frameStartTime = DispatchTime.now().uptimeNanoseconds

            let touchMask = controllerState.tickTurbo()
            let combinedMask = touchMask | physicalButtons
            bridge.setInputP1(AccuNESBridge.Button(rawValue: combinedMask))

            bridge.stepFrame()

            let framesDrained = bridge.drainAudioStereo(into: &audioScratch)
            if framesDrained > 0 && !isFastForward {
                audioEngine.write(samples: audioScratch, count: framesDrained * 2)
            }

            if let fbPtr = bridge.framebufferPointer {
                renderer.updateFramebuffer(from: fbPtr)
            }

            if !isFastForward {
                let elapsed = DispatchTime.now().uptimeNanoseconds - frameStartTime
                if frameDuration > elapsed {
                    let remaining = frameDuration - elapsed
                    if remaining > 2_000_000 {
                        var req = timespec(tv_sec: 0, tv_nsec: Int(remaining - 1_500_000))
                        nanosleep(&req, nil)
                    }
                    while (DispatchTime.now().uptimeNanoseconds - frameStartTime) < frameDuration {
                        sched_yield()
                    }
                }
            }
        }
    }
}
