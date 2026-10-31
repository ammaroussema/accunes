package com.accunes.audio

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack

class AudioEngine(private val sampleRate: Int = 44100) {

    private var audioTrack: AudioTrack? = null
    private val bufferSize: Int

    init {
        val minBuf = AudioTrack.getMinBufferSize(
            sampleRate,
            AudioFormat.CHANNEL_OUT_STEREO,
            AudioFormat.ENCODING_PCM_FLOAT
        )
        bufferSize = (minBuf * 2).coerceAtLeast(2048)
        initTrack()
    }

    private fun initTrack() {
        val attributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_GAME)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()

        val format = AudioFormat.Builder()
            .setSampleRate(sampleRate)
            .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
            .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
            .build()

        audioTrack = AudioTrack.Builder()
            .setAudioAttributes(attributes)
            .setAudioFormat(format)
            .setBufferSizeInBytes(bufferSize * 4)
            .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()

        audioTrack?.play()
    }

    fun write(samples: FloatArray, count: Int) {
        if (count <= 0) return
        audioTrack?.write(samples, 0, count, AudioTrack.WRITE_NON_BLOCKING)
    }

    fun pause() {
        audioTrack?.pause()
        audioTrack?.flush()
    }

    fun resume() {
        audioTrack?.play()
    }

    fun release() {
        audioTrack?.stop()
        audioTrack?.release()
        audioTrack = null
    }
}
