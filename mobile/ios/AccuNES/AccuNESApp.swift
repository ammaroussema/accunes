import SwiftUI
import MetalKit

@main
struct AccuNESApp: App {

    @Environment(\.scenePhase) private var scenePhase

    @StateObject private var controllerState = TouchControllerState()
    private let saveManager = SaveManager()

    @State private var bridge: AccuNESBridge?
    @State private var renderer: MetalRenderer?
    @State private var audioEngine: CoreAudioEngine?
    @State private var emulationLoop: EmulationLoop?
    @State private var gameControllerManager: GameControllerManager?

    @State private var activeGameTitle: String?
    @State private var currentRomHash: String = ""
    @State private var recentGames: [RecentGame] = []

    var body: some Scene {
        WindowGroup {
            if activeGameTitle != nil,
               let bridge = bridge,
               let renderer = renderer,
               let loop = emulationLoop {
                EmulationView(
                    bridge: bridge,
                    emulationLoop: loop,
                    renderer: renderer,
                    controllerState: controllerState,
                    saveManager: saveManager,
                    romHash: currentRomHash,
                    onExitGame: {
                        stopEmulation()
                        activeGameTitle = nil
                    }
                )
            } else {
                ROMPickerView(
                    recentGames: $recentGames,
                    onSelectROM: { url, title in
                        loadROM(url: url, title: title)
                        if !recentGames.contains(where: { $0.url == url }) {
                            recentGames.insert(RecentGame(title: title, url: url), at: 0)
                        }
                        activeGameTitle = title
                    }
                )
            }
        }
        .onChange(of: scenePhase) { phase in
            switch phase {
            case .background, .inactive:
                emulationLoop?.pause()
                if let b = bridge, !currentRomHash.isEmpty {
                    saveManager.saveSRAM(bridge: b, romHash: currentRomHash)
                }
            case .active:
                emulationLoop?.resume()
            @unknown default:
                break
            }
        }
    }

    private func loadROM(url: URL, title: String) {
        stopEmulation()

        let secured = url.startAccessingSecurityScopedResource()
        defer { if secured { url.stopAccessingSecurityScopedResource() } }

        guard let data = try? Data(contentsOf: url) else { return }

        let br = AccuNESBridge(sampleRate: 44100.0)
        guard br.loadROM(data: data) else { return }

        let crc = br.romCRC
        currentRomHash = String(format: "%08X", crc)

        saveManager.loadSRAM(bridge: br, romHash: currentRomHash)

        let dummyView = MTKView()
        guard let rend = MetalRenderer(mtkView: dummyView) else { return }

        let audio = CoreAudioEngine()
        let loop = EmulationLoop(
            bridge: br,
            renderer: rend,
            audioEngine: audio,
            controllerState: controllerState
        )

        gameControllerManager = GameControllerManager { buttons in
            loop.physicalButtons = buttons
            controllerState.isPhysicalConnected = (buttons != 0)
        }

        self.bridge = br
        self.renderer = rend
        self.audioEngine = audio
        self.emulationLoop = loop

        loop.start()
    }

    private func stopEmulation() {
        if let b = bridge, !currentRomHash.isEmpty {
            saveManager.saveSRAM(bridge: b, romHash: currentRomHash)
        }
        emulationLoop?.stop()
        bridge = nil
        emulationLoop = nil
        renderer = nil
        audioEngine = nil
    }
}
