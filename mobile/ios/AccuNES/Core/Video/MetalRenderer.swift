import MetalKit

final class MetalRenderer: NSObject, MTKViewDelegate {

    private let device: MTLDevice
    private let commandQueue: MTLCommandQueue
    private let pipelineState: MTLRenderPipelineState
    private var nearestSampler: MTLSamplerState?
    private var linearSampler: MTLSamplerState?
    private var texture: MTLTexture?

    var isLinearFilter: Bool = false

    private let rawPixelBuffer = UnsafeMutablePointer<UInt32>.allocate(capacity: 256 * 240)
    private let lock = NSLock()

    init?(mtkView: MTKView) {
        guard let dev = mtkView.device ?? MTLCreateSystemDefaultDevice(),
              let queue = dev.makeCommandQueue() else {
            return nil
        }
        self.device = dev
        self.commandQueue = queue
        mtkView.device = dev
        mtkView.colorPixelFormat = .bgra8Unorm
        mtkView.clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 1)

        guard let defaultLibrary = dev.makeDefaultLibrary(),
              let vertexFunc = defaultLibrary.makeFunction(name: "vertexShader"),
              let fragmentFunc = defaultLibrary.makeFunction(name: "fragmentShader") else {
            return nil
        }

        let pipeDesc = MTLRenderPipelineDescriptor()
        pipeDesc.vertexFunction = vertexFunc
        pipeDesc.fragmentFunction = fragmentFunc
        pipeDesc.colorAttachments[0].pixelFormat = mtkView.colorPixelFormat

        do {
            self.pipelineState = try dev.makeRenderPipelineState(descriptor: pipeDesc)
        } catch {
            return nil
        }

        super.init()

        let nearestDesc = MTLSamplerDescriptor()
        nearestDesc.minFilter = .nearest
        nearestDesc.magFilter = .nearest
        nearestSampler = dev.makeSamplerState(descriptor: nearestDesc)

        let linearDesc = MTLSamplerDescriptor()
        linearDesc.minFilter = .linear
        linearDesc.magFilter = .linear
        linearSampler = dev.makeSamplerState(descriptor: linearDesc)

        let texDesc = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .bgra8Unorm,
            width: 256,
            height: 240,
            mipmapped: false
        )
        texDesc.usage = [.shaderRead]
        self.texture = dev.makeTexture(descriptor: texDesc)
    }

    deinit {
        rawPixelBuffer.deallocate()
    }

    func updateFramebuffer(from pointer: UnsafePointer<UInt32>) {
        lock.lock()
        rawPixelBuffer.update(from: pointer, count: 256 * 240)
        lock.unlock()
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}

    func draw(in view: MTKView) {
        guard let drawable = view.currentDrawable,
              let rpd = view.currentRenderPassDescriptor,
              let texture = texture,
              let cmdBuffer = commandQueue.makeCommandBuffer(),
              let encoder = cmdBuffer.makeRenderCommandEncoder(descriptor: rpd) else {
            return
        }

        lock.lock()
        texture.replace(
            region: MTLRegionMake2D(0, 0, 256, 240),
            mipmapLevel: 0,
            withBytes: rawPixelBuffer,
            bytesPerRow: 256 * 4
        )
        lock.unlock()

        let targetRatio: CGFloat = 4.0 / 3.0
        let currentRatio = view.bounds.width / max(view.bounds.height, 1)

        var vpWidth = view.drawableSize.width
        var vpHeight = view.drawableSize.height
        var vpX: Double = 0
        var vpY: Double = 0

        if currentRatio > targetRatio {
            vpWidth = view.drawableSize.height * targetRatio
            vpX = (Double(view.drawableSize.width) - Double(vpWidth)) / 2.0
        } else {
            vpHeight = view.drawableSize.width / targetRatio
            vpY = (Double(view.drawableSize.height) - Double(vpHeight)) / 2.0
        }

        encoder.setViewport(MTLViewport(
            originX: vpX, originY: vpY,
            width: Double(vpWidth), height: Double(vpHeight),
            znear: 0.0, zfar: 1.0
        ))

        encoder.setRenderPipelineState(pipelineState)
        encoder.setFragmentTexture(texture, index: 0)

        let sampler = isLinearFilter ? linearSampler : nearestSampler
        encoder.setFragmentSamplerState(sampler, index: 0)

        encoder.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)
        encoder.endEncoding()

        cmdBuffer.present(drawable)
        cmdBuffer.commit()
    }
}
