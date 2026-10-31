package com.accunes

import java.io.Closeable
import java.nio.ByteBuffer

class Emulator(sampleRate: Double = 44100.0) : Closeable {

    companion object {
        init {
            System.loadLibrary("accunes_ffi")
        }

        const val BTN_A = 1 shl 0
        const val BTN_B = 1 shl 1
        const val BTN_SELECT = 1 shl 2
        const val BTN_START = 1 shl 3
        const val BTN_UP = 1 shl 4
        const val BTN_DOWN = 1 shl 5
        const val BTN_LEFT = 1 shl 6
        const val BTN_RIGHT = 1 shl 7

        const val PIXFMT_RGBA8888 = 0
        const val PIXFMT_BGRA8888 = 1
        const val PIXFMT_ARGB8888 = 2
        const val PIXFMT_RGB565 = 3

        const val REGION_AUTO = 0
        const val REGION_NTSC = 1
        const val REGION_PAL = 2
        const val REGION_DENDY = 3

        @JvmStatic private external fun nativeCreate(sampleRate: Double): Long
        @JvmStatic private external fun nativeDestroy(handle: Long)
        @JvmStatic private external fun nativeLoadRom(handle: Long, data: ByteArray): Int
        @JvmStatic private external fun nativeLoadRomFile(handle: Long, path: String): Int
        @JvmStatic private external fun nativeStepFrame(handle: Long)
        @JvmStatic private external fun nativeDrainAudioStereo(handle: Long, buffer: FloatArray): Int
        @JvmStatic private external fun nativeSetInputP1(handle: Long, buttons: Int)
        @JvmStatic private external fun nativeSetInputP2(handle: Long, buttons: Int)
        @JvmStatic private external fun nativeSetZapper(handle: Long, x: Int, y: Int, trigger: Boolean)
        @JvmStatic private external fun nativeSaveState(handle: Long): ByteArray?
        @JvmStatic private external fun nativeLoadState(handle: Long, state: ByteArray): Boolean
        @JvmStatic private external fun nativeGetSram(handle: Long): ByteArray?
        @JvmStatic private external fun nativeSetSram(handle: Long, sram: ByteArray): Boolean
        @JvmStatic private external fun nativeReset(handle: Long)
        @JvmStatic private external fun nativePowerCycle(handle: Long)
        @JvmStatic private external fun nativeGetFramebufferPtr(handle: Long): Long
        @JvmStatic private external fun nativeCopyFramebuffer(handle: Long, dest: ByteBuffer, format: Int): Boolean
        @JvmStatic private external fun nativeGetRomCrc(handle: Long): Int
        @JvmStatic private external fun nativeGetRomMapper(handle: Long): Int
        @JvmStatic private external fun nativePeekMemory(handle: Long, addr: Int): Int
        @JvmStatic private external fun nativePokeMemory(handle: Long, addr: Int, value: Int)
    }

    private var handle: Long = nativeCreate(sampleRate)

    fun loadRom(data: ByteArray): Boolean {
        checkOpen()
        return nativeLoadRom(handle, data) == 0
    }

    fun loadRomFile(path: String): Boolean {
        checkOpen()
        return nativeLoadRomFile(handle, path) == 0
    }

    fun stepFrame() {
        checkOpen()
        nativeStepFrame(handle)
    }

    fun drainAudioStereo(buffer: FloatArray): Int {
        checkOpen()
        return nativeDrainAudioStereo(handle, buffer)
    }

    fun setInputP1(buttons: Int) {
        checkOpen()
        nativeSetInputP1(handle, buttons)
    }

    fun setInputP2(buttons: Int) {
        checkOpen()
        nativeSetInputP2(handle, buttons)
    }

    fun setZapper(x: Int, y: Int, trigger: Boolean) {
        checkOpen()
        nativeSetZapper(handle, x, y, trigger)
    }

    fun saveState(): ByteArray? {
        checkOpen()
        return nativeSaveState(handle)
    }

    fun loadState(state: ByteArray): Boolean {
        checkOpen()
        return nativeLoadState(handle, state)
    }

    fun getSram(): ByteArray? {
        checkOpen()
        return nativeGetSram(handle)
    }

    fun setSram(sram: ByteArray): Boolean {
        checkOpen()
        return nativeSetSram(handle, sram)
    }

    fun reset() {
        checkOpen()
        nativeReset(handle)
    }

    fun powerCycle() {
        checkOpen()
        nativePowerCycle(handle)
    }

    fun getFramebufferPointer(): Long {
        checkOpen()
        return nativeGetFramebufferPtr(handle)
    }

    fun copyFramebuffer(dest: ByteBuffer, format: Int = PIXFMT_RGBA8888): Boolean {
        checkOpen()
        return nativeCopyFramebuffer(handle, dest, format)
    }

    fun getRomCrc(): Int {
        checkOpen()
        return nativeGetRomCrc(handle)
    }

    fun getRomMapper(): Int {
        checkOpen()
        return nativeGetRomMapper(handle)
    }

    fun peekMemory(addr: Int): Int {
        checkOpen()
        return nativePeekMemory(handle, addr)
    }

    fun pokeMemory(addr: Int, value: Int) {
        checkOpen()
        nativePokeMemory(handle, addr, value)
    }

    override fun close() {
        if (handle != 0L) {
            nativeDestroy(handle)
            handle = 0L
        }
    }

    private fun checkOpen() {
        check(handle != 0L) { "Emulator has been closed" }
    }
}
