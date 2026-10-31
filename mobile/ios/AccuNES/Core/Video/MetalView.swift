import SwiftUI
import MetalKit

struct MetalView: UIViewRepresentable {

    let renderer: MetalRenderer

    func makeUIView(context: Context) -> MTKView {
        let mtkView = MTKView()
        mtkView.delegate = renderer
        mtkView.isPaused = false
        mtkView.enableSetNeedsDisplay = false
        mtkView.preferredFramesPerSecond = 60
        return mtkView
    }

    func updateUIView(_ uiView: MTKView, context: Context) {}
}
