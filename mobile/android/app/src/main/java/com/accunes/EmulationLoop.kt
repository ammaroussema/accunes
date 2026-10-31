package com.accunes

import android.os.Process
import com.accunes.audio.AudioEngine
import com.accunes.input.TouchControllerState
import com.accunes.video.NesSurfaceView
import java.util.concurrent.atomic.AtomicBoolean

class EmulationLoop(
    private val emulator: Emulator,
    private val surfaceView: NesSurfaceView,
    private val audioEngine: AudioEngine,
    private val controllerState: TouchControllerState
) : Runnable {

    private var thread: Thread? = null
    private val isRunning = AtomicBoolean(false)
    private val isPaused = AtomicBoolean(false)

    var isFastForward = false
    var physicalButtons = 0

    private val audioScratch = FloatArray(4096)

    fun start() {
        if (isRunning.compareAndSet(false, true)) {
            thread = Thread(this, "AccuNES-Core").apply {
                priority = Thread.MAX_PRIORITY
                start()
            }
        }
    }

    fun pause() {
        isPaused.set(true)
        audioEngine.pause()
    }

    fun resume() {
        isPaused.set(false)
        audioEngine.resume()
    }

    fun stop() {
        isRunning.set(false)
        isPaused.set(false)
        thread?.join(500)
        thread = null
        audioEngine.release()
    }

    override fun run() {
        Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)

        val frameDurationNs = 1_000_000_000L / 60L
        var nextFrameTime = System.nanoTime()

        while (isRunning.get()) {
            if (isPaused.get()) {
                try {
                    Thread.sleep(16)
                } catch (_: InterruptedException) {
                    break
                }
                nextFrameTime = System.nanoTime()
                continue
            }

            val touchButtons = controllerState.tickTurbo()
            val combinedButtons = touchButtons or physicalButtons
            emulator.setInputP1(combinedButtons)

            emulator.stepFrame()

            val samplesRead = emulator.drainAudioStereo(audioScratch)
            if (samplesRead > 0 && !isFastForward) {
                audioEngine.write(audioScratch, samplesRead * 2)
            }

            synchronized(surfaceView.renderer.pixelBuffer) {
                surfaceView.renderer.pixelBuffer.position(0)
                emulator.copyFramebuffer(surfaceView.renderer.pixelBuffer, Emulator.PIXFMT_RGBA8888)
            }
            surfaceView.requestRender()

            if (!isFastForward) {
                nextFrameTime += frameDurationNs
                val sleepNs = nextFrameTime - System.nanoTime()
                if (sleepNs > 0) {
                    val sleepMs = sleepNs / 1_000_000L
                    val sleepRemNs = (sleepNs % 1_000_000L).toInt()
                    try {
                        Thread.sleep(sleepMs, sleepRemNs)
                    } catch (_: InterruptedException) {
                        break
                    }
                } else if (sleepNs < -frameDurationNs * 3) {
                    nextFrameTime = System.nanoTime()
                }
            }
        }
    }
}
