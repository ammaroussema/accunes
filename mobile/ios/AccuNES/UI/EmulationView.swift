import SwiftUI

struct EmulationView: View {

    let bridge: AccuNESBridge
    let emulationLoop: EmulationLoop
    let renderer: MetalRenderer
    @ObservedObject var controllerState: TouchControllerState
    let saveManager: SaveManager
    let romHash: String
    let onExitGame: () -> Void

    @State private var showQuickMenu = false

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            MetalView(renderer: renderer)
                .ignoresSafeArea()
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { value in
                            guard controllerState.zapperMode else { return }
                            let size = UIScreen.main.bounds.size
                            let targetRatio: CGFloat = 4.0 / 3.0
                            let currentRatio = size.width / max(size.height, 1)

                            var vpWidth = size.width
                            var vpHeight = size.height
                            var vpX: CGFloat = 0
                            var vpY: CGFloat = 0

                            if currentRatio > targetRatio {
                                vpWidth = size.height * targetRatio
                                vpX = (size.width - vpWidth) / 2.0
                            } else {
                                vpHeight = size.width / targetRatio
                                vpY = (size.height - vpHeight) / 2.0
                            }

                            let relX = Int32(((value.location.x - vpX) / vpWidth * 256.0).clamped(to: 0...255))
                            let relY = Int32(((value.location.y - vpY) / vpHeight * 240.0).clamped(to: 0...239))

                            controllerState.triggerHaptic()
                            bridge.setZapper(x: relX, y: relY, trigger: true)
                        }
                        .onEnded { _ in
                            guard controllerState.zapperMode else { return }
                            bridge.setZapper(x: 0, y: 0, trigger: false)
                        }
                )

            VirtualGamepad(state: controllerState)
                .ignoresSafeArea()

            VStack {
                HStack {
                    Spacer()
                    Button(action: {
                        emulationLoop.pause()
                        showQuickMenu = true
                    }) {
                        Image(systemName: "line.3.horizontal")
                            .font(.title2)
                            .foregroundColor(.white.opacity(0.7))
                            .padding(12)
                            .background(Color.black.opacity(0.4))
                            .clipShape(Circle())
                    }
                    .padding(.top, 8)
                    .padding(.trailing, 16)
                }
                Spacer()
            }
        }
        .sheet(isPresented: $showQuickMenu) {
            QuickMenuSheet(
                bridge: bridge,
                emulationLoop: emulationLoop,
                renderer: renderer,
                controllerState: controllerState,
                saveManager: saveManager,
                romHash: romHash,
                onDismiss: {
                    showQuickMenu = false
                    emulationLoop.resume()
                },
                onExitGame: onExitGame
            )
        }
    }
}
