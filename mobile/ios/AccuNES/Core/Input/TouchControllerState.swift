import SwiftUI
import UIKit

final class TouchControllerState: ObservableObject {

    @Published var buttonsMask: UInt8 = 0
    @Published var turboA: Bool = false
    @Published var turboB: Bool = false
    @Published var hapticsEnabled: Bool = true
    @Published var zapperMode: Bool = false
    @Published var opacity: Double = 0.75
    @Published var isPhysicalConnected: Bool = false

    private var turboCounter = 0
    private let impactFeedback = UIImpactFeedbackGenerator(style: .medium)

    init() {
        impactFeedback.prepare()
    }

    func pressButton(_ mask: UInt8) {
        if (buttonsMask & mask) == 0 {
            triggerHaptic()
        }
        buttonsMask |= mask
    }

    func releaseButton(_ mask: UInt8) {
        buttonsMask &= ~mask
    }

    func setDpadDirection(_ mask: UInt8) {
        let dpadClear: UInt8 = ~(AccuNESBridge.Button.up.rawValue |
                                AccuNESBridge.Button.down.rawValue |
                                AccuNESBridge.Button.left.rawValue |
                                AccuNESBridge.Button.right.rawValue)
        let oldDpad = buttonsMask & (AccuNESBridge.Button.up.rawValue |
                                     AccuNESBridge.Button.down.rawValue |
                                     AccuNESBridge.Button.left.rawValue |
                                     AccuNESBridge.Button.right.rawValue)
        if oldDpad != mask && mask != 0 {
            triggerHaptic()
        }
        buttonsMask = (buttonsMask & dpadClear) | mask
    }

    func releaseDpad() {
        let dpadClear: UInt8 = ~(AccuNESBridge.Button.up.rawValue |
                                AccuNESBridge.Button.down.rawValue |
                                AccuNESBridge.Button.left.rawValue |
                                AccuNESBridge.Button.right.rawValue)
        buttonsMask &= dpadClear
    }

    func tickTurbo() -> UInt8 {
        turboCounter = (turboCounter + 1) % 2
        var active = buttonsMask
        if turboA {
            if turboCounter == 0 {
                active |= AccuNESBridge.Button.a.rawValue
            } else {
                active &= ~AccuNESBridge.Button.a.rawValue
            }
        }
        if turboB {
            if turboCounter == 0 {
                active |= AccuNESBridge.Button.b.rawValue
            } else {
                active &= ~AccuNESBridge.Button.b.rawValue
            }
        }
        return active
    }

    func triggerHaptic() {
        guard hapticsEnabled else { return }
        impactFeedback.impactOccurred()
    }
}
