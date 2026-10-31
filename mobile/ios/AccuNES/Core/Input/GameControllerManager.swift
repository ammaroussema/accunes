import GameController

final class GameControllerManager {

    private let onButtonsChanged: (UInt8) -> Void
    private var physicalMask: UInt8 = 0

    init(onButtonsChanged: @escaping (UInt8) -> Void) {
        self.onButtonsChanged = onButtonsChanged
        setupNotificationObservers()
        checkConnectedControllers()
    }

    private func setupNotificationObservers() {
        NotificationCenter.default.addObserver(
            forName: .GCControllerDidConnect,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.checkConnectedControllers()
        }

        NotificationCenter.default.addObserver(
            forName: .GCControllerDidDisconnect,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.checkConnectedControllers()
        }
    }

    private func checkConnectedControllers() {
        let controllers = GCController.controllers()
        for controller in controllers {
            setupController(controller)
        }
    }

    private func setupController(_ controller: GCController) {
        if let gamepad = controller.extendedGamepad {
            gamepad.valueChangedHandler = { [weak self] _, _ in
                self?.pollExtendedGamepad(gamepad)
            }
        } else if let gamepad = controller.microGamepad {
            gamepad.valueChangedHandler = { [weak self] _, _ in
                self?.pollMicroGamepad(gamepad)
            }
        }
    }

    private func pollExtendedGamepad(_ gamepad: GCExtendedGamepad) {
        var mask: UInt8 = 0

        if gamepad.buttonA.isPressed { mask |= AccuNESBridge.Button.a.rawValue }
        if gamepad.buttonB.isPressed { mask |= AccuNESBridge.Button.b.rawValue }
        if gamepad.buttonX.isPressed { mask |= AccuNESBridge.Button.b.rawValue }
        if gamepad.buttonY.isPressed { mask |= AccuNESBridge.Button.a.rawValue }

        if gamepad.buttonMenu.isPressed { mask |= AccuNESBridge.Button.start.rawValue }
        if gamepad.buttonOptions?.isPressed == true { mask |= AccuNESBridge.Button.select.rawValue }

        if gamepad.dpad.up.isPressed || gamepad.leftThumbstick.up.value > 0.4 { mask |= AccuNESBridge.Button.up.rawValue }
        if gamepad.dpad.down.isPressed || gamepad.leftThumbstick.down.value > 0.4 { mask |= AccuNESBridge.Button.down.rawValue }
        if gamepad.dpad.left.isPressed || gamepad.leftThumbstick.left.value > 0.4 { mask |= AccuNESBridge.Button.left.rawValue }
        if gamepad.dpad.right.isPressed || gamepad.leftThumbstick.right.value > 0.4 { mask |= AccuNESBridge.Button.right.rawValue }

        physicalMask = mask
        onButtonsChanged(physicalMask)
    }

    private func pollMicroGamepad(_ gamepad: GCMicroGamepad) {
        var mask: UInt8 = 0
        if gamepad.buttonA.isPressed { mask |= AccuNESBridge.Button.a.rawValue }
        if gamepad.buttonX.isPressed { mask |= AccuNESBridge.Button.b.rawValue }
        if gamepad.dpad.up.isPressed { mask |= AccuNESBridge.Button.up.rawValue }
        if gamepad.dpad.down.isPressed { mask |= AccuNESBridge.Button.down.rawValue }
        if gamepad.dpad.left.isPressed { mask |= AccuNESBridge.Button.left.rawValue }
        if gamepad.dpad.right.isPressed { mask |= AccuNESBridge.Button.right.rawValue }

        physicalMask = mask
        onButtonsChanged(physicalMask)
    }
}
