import SwiftUI

struct QuickMenuSheet: View {

    let bridge: AccuNESBridge
    let emulationLoop: EmulationLoop
    let renderer: MetalRenderer
    @ObservedObject var controllerState: TouchControllerState
    let saveManager: SaveManager
    let romHash: String
    let onDismiss: () -> Void
    let onExitGame: () -> Void

    @State private var selectedSlot: Int = 0
    @State private var statusMessage: String?
    @State private var isFastForward: Bool = false
    @State private var isLinear: Bool = false

    var body: some View {
        NavigationView {
            Form {
                if let msg = statusMessage {
                    Section {
                        Text(msg)
                            .foregroundColor(.red)
                            .font(.subheadline)
                    }
                }

                Section(header: Text("Save States")) {
                    Picker("Slot", selection: $selectedSlot) {
                        ForEach(0..<5) { slot in
                            let hasSave = saveManager.hasStateSlot(romHash: romHash, slot: slot)
                            Text("Slot \(slot)" + (hasSave ? " *" : "")).tag(slot)
                        }
                    }
                    .pickerStyle(.segmented)

                    HStack(spacing: 16) {
                        Button("Save State") {
                            let ok = saveManager.saveStateSlot(bridge: bridge, romHash: romHash, slot: selectedSlot)
                            statusMessage = ok ? "Saved to Slot \(selectedSlot)" : "Save failed"
                        }
                        .buttonStyle(.borderedProminent)

                        Button("Load State") {
                            let ok = saveManager.loadStateSlot(bridge: bridge, romHash: romHash, slot: selectedSlot)
                            statusMessage = ok ? "Loaded Slot \(selectedSlot)" : "Slot is empty"
                        }
                        .buttonStyle(.bordered)
                    }
                }

                Section(header: Text("Emulation Controls")) {
                    HStack {
                        Button("Soft Reset") {
                            bridge.reset()
                            statusMessage = "Reset performed"
                        }
                        Spacer()
                        Button("Hard Reset") {
                            bridge.powerCycle()
                            statusMessage = "Power cycle performed"
                        }
                    }

                    Toggle("Fast Forward (2x)", isOn: $isFastForward)
                        .onChange(of: isFastForward) { val in
                            emulationLoop.isFastForward = val
                        }
                }

                Section(header: Text("Display & Controls")) {
                    Toggle("Smooth Video Filtering", isOn: $isLinear)
                        .onChange(of: isLinear) { val in
                            renderer.isLinearFilter = val
                        }

                    Toggle("Touch Zapper Gun Mode", isOn: $controllerState.zapperMode)
                    Toggle("Haptic Feedback", isOn: $controllerState.hapticsEnabled)

                    VStack(alignment: .leading) {
                        Text("Touch Opacity: \(Int(controllerState.opacity * 100))%")
                        Slider(value: $controllerState.opacity, in: 0.1...1.0)
                    }
                }

                Section {
                    Button("Exit to Game Library", role: .destructive) {
                        onDismiss()
                        onExitGame()
                    }
                }
            }
            .navigationTitle("AccuNES Menu")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Resume") {
                        onDismiss()
                    }
                }
            }
            .onAppear {
                isFastForward = emulationLoop.isFastForward
                isLinear = renderer.isLinearFilter
            }
        }
    }
}
