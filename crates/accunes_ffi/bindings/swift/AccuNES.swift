import Foundation

public final class AccuNES {
    public struct Button: OptionSet {
        public let rawValue: UInt8
        public init(rawValue: UInt8) { self.rawValue = rawValue }

        public static let a      = Button(rawValue: 1 << 0)
        public static let b      = Button(rawValue: 1 << 1)
        public static let select = Button(rawValue: 1 << 2)
        public static let start  = Button(rawValue: 1 << 3)
        public static let up     = Button(rawValue: 1 << 4)
        public static let down   = Button(rawValue: 1 << 5)
        public static let left   = Button(rawValue: 1 << 6)
        public static let right  = Button(rawValue: 1 << 7)
    }

    public enum PixelFormat: UInt32 {
        case rgba8888 = 0
        case bgra8888 = 1
        case argb8888 = 2
        case rgb565   = 3
    }

    public enum Region: UInt32 {
        case auto  = 0
        case ntsc  = 1
        case pal   = 2
        case dendy = 3
    }

    private var emu: OpaquePointer?

    public init(sampleRate: Double = 44100.0) {
        self.emu = accunes_create(sampleRate)
    }

    deinit {
        if let emu = emu {
            accunes_destroy(emu)
        }
    }

    public func loadROM(data: Data) -> Bool {
        guard let emu = emu else { return false }
        return data.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return false }
            return accunes_load_rom(emu, ptr, data.count) == 0
        }
    }

    public func loadROM(atPath path: String) -> Bool {
        guard let emu = emu else { return false }
        return path.withCString { cstr in
            accunes_load_rom_file(emu, cstr) == 0
        }
    }

    public var isROMLoaded: Bool {
        guard let emu = emu else { return false }
        return accunes_is_rom_loaded(emu)
    }

    public var romCRC: UInt32 {
        guard let emu = emu else { return 0 }
        return accunes_get_rom_crc(emu)
    }

    public var romMapper: UInt16 {
        guard let emu = emu else { return 0 }
        return accunes_get_rom_mapper(emu)
    }

    public func stepFrame() {
        guard let emu = emu else { return }
        accunes_step_frame(emu)
    }

    public func reset() {
        guard let emu = emu else { return }
        accunes_reset(emu)
    }

    public func powerCycle() {
        guard let emu = emu else { return }
        accunes_power_cycle(emu)
    }

    public var framebufferPointer: UnsafePointer<UInt32>? {
        guard let emu = emu else { return nil }
        return accunes_get_framebuffer_ptr(emu)
    }

    public func copyFramebuffer(to dest: UnsafeMutablePointer<UInt8>, format: PixelFormat) -> Bool {
        guard let emu = emu else { return false }
        return accunes_copy_framebuffer(emu, dest, format.rawValue)
    }

    public func drainAudio(into buffer: inout [Float]) -> Int {
        guard let emu = emu else { return 0 }
        return buffer.withUnsafeMutableBufferPointer { ptr in
            guard let base = ptr.baseAddress else { return 0 }
            return accunes_drain_audio(emu, base, ptr.count)
        }
    }

    public func drainAudioStereo(into buffer: inout [Float]) -> Int {
        guard let emu = emu else { return 0 }
        return buffer.withUnsafeMutableBufferPointer { ptr in
            guard let base = ptr.baseAddress else { return 0 }
            return accunes_drain_audio_stereo(emu, base, ptr.count / 2)
        }
    }

    public func setInputP1(_ buttons: Button) {
        guard let emu = emu else { return }
        accunes_set_input_p1(emu, buttons.rawValue)
    }

    public func setInputP2(_ buttons: Button) {
        guard let emu = emu else { return }
        accunes_set_input_p2(emu, buttons.rawValue)
    }

    public func setZapper(x: Int32, y: Int32, trigger: bool) {
        guard let emu = emu else { return }
        accunes_set_zapper(emu, x, y, trigger)
    }

    public func saveState() -> Data? {
        guard let emu = emu else { return nil }
        var len: Int = 0
        guard let ptr = accunes_save_state(emu, &len), len > 0 else { return nil }
        let data = Data(bytes: ptr, count: len)
        accunes_free_bytes(ptr, len)
        return data
    }

    public func loadState(_ state: Data) -> Bool {
        guard let emu = emu else { return false }
        return state.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return false }
            return accunes_load_state(emu, ptr, state.count)
        }
    }

    public func getSRAM() -> Data? {
        guard let emu = emu else { return nil }
        var len: Int = 0
        guard let ptr = accunes_get_sram(emu, &len), len > 0 else { return nil }
        let data = Data(bytes: ptr, count: len)
        accunes_free_bytes(ptr, len)
        return data
    }

    public func setSRAM(_ sram: Data) -> Bool {
        guard let emu = emu else { return false }
        return sram.withUnsafeBytes { raw in
            guard let ptr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return false }
            return accunes_set_sram(emu, ptr, sram.count)
        }
    }

    public func setRegion(_ region: Region) {
        guard let emu = emu else { return }
        accunes_set_region(emu, region.rawValue)
    }

    public func peekMemory(addr: UInt16) -> UInt8 {
        guard let emu = emu else { return 0 }
        return accunes_peek_memory(emu, addr)
    }

    public func pokeMemory(addr: UInt16, value: UInt8) {
        guard let emu = emu else { return }
        accunes_poke_memory(emu, addr, value)
    }
}
