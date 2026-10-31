import Foundation

final class SaveManager {

    private let savesDirectory: URL

    init() {
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        self.savesDirectory = docs.appendingPathComponent("Saves", isDirectory: true)
        try? FileManager.default.createDirectory(at: savesDirectory, withIntermediateDirectories: true)
    }

    func saveSRAM(bridge: AccuNESBridge, romHash: String) {
        guard let sram = bridge.getSRAM() else { return }
        let file = savesDirectory.appendingPathComponent("\(romHash).sav")
        try? sram.write(to: file)
    }

    func loadSRAM(bridge: AccuNESBridge, romHash: String) {
        let file = savesDirectory.appendingPathComponent("\(romHash).sav")
        guard let data = try? Data(contentsOf: file), !data.isEmpty else { return }
        _ = bridge.setSRAM(data)
    }

    func saveStateSlot(bridge: AccuNESBridge, romHash: String, slot: Int) -> Bool {
        guard let state = bridge.saveState() else { return false }
        let file = savesDirectory.appendingPathComponent("\(romHash)_slot\(slot).state")
        do {
            try state.write(to: file)
            return true
        } catch {
            return false
        }
    }

    func loadStateSlot(bridge: AccuNESBridge, romHash: String, slot: Int) -> Bool {
        let file = savesDirectory.appendingPathComponent("\(romHash)_slot\(slot).state")
        guard let data = try? Data(contentsOf: file), !data.isEmpty else { return false }
        return bridge.loadState(data)
    }

    func hasStateSlot(romHash: String, slot: Int) -> Bool {
        let file = savesDirectory.appendingPathComponent("\(romHash)_slot\(slot).state")
        return FileManager.default.fileExists(atPath: file.path)
    }
}
