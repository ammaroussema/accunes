import SwiftUI

struct VirtualGamepad: View {

    @ObservedObject var state: TouchControllerState

    var body: some View {
        let alpha = state.isPhysicalConnected ? 0.0 : state.opacity

        GeometryReader { _ in
            ZStack {
                VStack {
                    Spacer()
                    HStack(alignment: .bottom) {
                        if !state.zapperMode {
                            DpadView(state: state)
                                .frame(width: 170, height: 170)
                        } else {
                            RoundButton(
                                title: "AWAY",
                                color: Color.gray.opacity(0.6),
                                size: 70
                            ) { pressed in
                                if pressed {
                                    state.pressButton(AccuNESBridge.Button.b.rawValue)
                                } else {
                                    state.releaseButton(AccuNESBridge.Button.b.rawValue)
                                }
                            }
                        }

                        Spacer()

                        HStack(spacing: 16) {
                            PillButtonView(title: "SELECT") { pressed in
                                if pressed {
                                    state.pressButton(AccuNESBridge.Button.select.rawValue)
                                } else {
                                    state.releaseButton(AccuNESBridge.Button.select.rawValue)
                                }
                            }

                            PillButtonView(title: "START") { pressed in
                                if pressed {
                                    state.pressButton(AccuNESBridge.Button.start.rawValue)
                                } else {
                                    state.releaseButton(AccuNESBridge.Button.start.rawValue)
                                }
                            }
                        }
                        .padding(.bottom, 8)

                        Spacer()

                        ActionButtonsView(state: state)
                            .frame(width: 180, height: 180)
                    }
                    .padding(.horizontal, 16)
                    .padding(.bottom, 16)
                }
            }
            .opacity(alpha)
            .animation(.easeInOut(duration: 0.2), value: alpha)
        }
    }
}

struct DpadView: View {

    @ObservedObject var state: TouchControllerState
    @State private var knobOffset: CGSize = .zero

    var body: some View {
        GeometryReader { geo in
            let radius = geo.size.width / 2.0

            ZStack {
                Circle()
                    .fill(Color.white.opacity(0.15))

                Circle()
                    .fill(Color.white.opacity(0.25))
                    .frame(width: radius * 0.9, height: radius * 0.9)

                Circle()
                    .fill(Color(red: 229/255, green: 37/255, blue: 33/255))
                    .frame(width: radius * 0.55, height: radius * 0.55)
                    .offset(knobOffset)
            }
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        let deltaX = value.location.x - radius
                        let deltaY = value.location.y - radius
                        updateDirection(dx: deltaX, dy: deltaY, maxRadius: radius)
                    }
                    .onEnded { _ in
                        knobOffset = .zero
                        state.releaseDpad()
                    }
            )
        }
    }

    private func updateDirection(dx: CGFloat, dy: CGFloat, maxRadius: CGFloat) {
        let dist = sqrt(dx * dx + dy * dy)
        let deadzone = maxRadius * 0.15
        if dist < deadzone {
            state.setDpadDirection(0)
            knobOffset = .zero
            return
        }

        var angle = atan2(dy, dx) * 180.0 / .pi
        if angle < 0 { angle += 360.0 }

        var mask: UInt8 = 0
        switch angle {
        case 22.5..<67.5:
            mask = AccuNESBridge.Button.down.rawValue | AccuNESBridge.Button.right.rawValue
        case 67.5..<112.5:
            mask = AccuNESBridge.Button.down.rawValue
        case 112.5..<157.5:
            mask = AccuNESBridge.Button.down.rawValue | AccuNESBridge.Button.left.rawValue
        case 157.5..<202.5:
            mask = AccuNESBridge.Button.left.rawValue
        case 202.5..<247.5:
            mask = AccuNESBridge.Button.up.rawValue | AccuNESBridge.Button.left.rawValue
        case 247.5..<292.5:
            mask = AccuNESBridge.Button.up.rawValue
        case 292.5..<337.5:
            mask = AccuNESBridge.Button.up.rawValue | AccuNESBridge.Button.right.rawValue
        default:
            mask = AccuNESBridge.Button.right.rawValue
        }

        state.setDpadDirection(mask)

        let clampedDist = min(dist, maxRadius * 0.6)
        let rad = angle * .pi / 180.0
        knobOffset = CGSize(width: clampedDist * cos(rad), height: clampedDist * sin(rad))
    }
}

struct ActionButtonsView: View {

    @ObservedObject var state: TouchControllerState

    var body: some View {
        ZStack {
            RoundButton(
                title: "TB",
                color: Color.orange.opacity(0.4),
                size: 48
            ) { state.turboB = $0 }
            .offset(x: -45, y: -45)

            RoundButton(
                title: "TA",
                color: Color.orange.opacity(0.4),
                size: 48
            ) { state.turboA = $0 }
            .offset(x: 45, y: -45)

            RoundButton(
                title: "B",
                color: Color(red: 229/255, green: 37/255, blue: 33/255),
                size: 62
            ) { pressed in
                if pressed {
                    state.pressButton(AccuNESBridge.Button.b.rawValue)
                } else {
                    state.releaseButton(AccuNESBridge.Button.b.rawValue)
                }
            }
            .offset(x: -45, y: 35)

            RoundButton(
                title: "A",
                color: Color(red: 229/255, green: 37/255, blue: 33/255),
                size: 62
            ) { pressed in
                if pressed {
                    state.pressButton(AccuNESBridge.Button.a.rawValue)
                } else {
                    state.releaseButton(AccuNESBridge.Button.a.rawValue)
                }
            }
            .offset(x: 45, y: 35)

            RoundButton(
                title: "B+A",
                color: Color.gray.opacity(0.5),
                size: 40
            ) { pressed in
                let mask = AccuNESBridge.Button.a.rawValue | AccuNESBridge.Button.b.rawValue
                if pressed {
                    state.pressButton(mask)
                } else {
                    state.releaseButton(mask)
                }
            }
        }
    }
}

struct RoundButton: View {

    let title: String
    let color: Color
    let size: CGFloat
    let onStateChange: (Bool) -> Void

    @State private var isPressed = false

    var body: some View {
        ZStack {
            Circle()
                .fill(isPressed ? color.opacity(0.9) : color)
                .frame(width: size, height: size)

            Text(title)
                .font(.system(size: size * 0.32, weight: .bold))
                .foregroundColor(.white)
        }
        .gesture(
            DragGesture(minimumDistance: 0)
                .onChanged { _ in
                    if !isPressed {
                        isPressed = true
                        onStateChange(true)
                    }
                }
                .onEnded { _ in
                    isPressed = false
                    onStateChange(false)
                }
        )
    }
}

struct PillButtonView: View {

    let title: String
    let onStateChange: (Bool) -> Void

    @State private var isPressed = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 13)
                .fill(isPressed ? Color.gray : Color.black.opacity(0.4))
                .frame(width: 64, height: 26)

            Text(title)
                .font(.system(size: 10, weight: .bold))
                .foregroundColor(.white)
        }
        .gesture(
            DragGesture(minimumDistance: 0)
                .onChanged { _ in
                    if !isPressed {
                        isPressed = true
                        onStateChange(true)
                    }
                }
                .onEnded { _ in
                    isPressed = false
                    onStateChange(false)
                }
        )
    }
}
