// extra audio processing stuff with dynamic rate control!!
class NesAudioProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.buffer = new Float32Array(32768);
        this.writePtr = 0;
        this.readPtr = 0.0;
        this.sampleCount = 0;

        this.port.onmessage = (e) => {
            const samples = e.data;
            if (!samples || samples.length === 0) return;
            const len = samples.length;
            const cap = 32768;

            for (let i = 0; i < len; i++) {
                this.buffer[this.writePtr] = samples[i];
                this.writePtr = (this.writePtr + 1) % cap;
            }
            this.sampleCount = Math.min(cap, this.sampleCount + len);
        };
    }

    process(inputs, outputs, parameters) {
        const output = outputs[0];
        if (!output || output.length === 0) return true;
        const channel = output[0];
        const len = channel.length;
        const cap = 32768;

        let rate = 1.0;
        if (this.sampleCount > 6144) {
            const skip = this.sampleCount - 3072;
            this.readPtr = (this.readPtr + skip) % cap;
            this.sampleCount = 3072;
            rate = 1.015;
        } else if (this.sampleCount > 2800) {
            rate = 1.004;
        } else if (this.sampleCount > 2400) {
            rate = 1.002;
        } else if (this.sampleCount < 800) {
            rate = 0.995;
        } else if (this.sampleCount < 1400) {
            rate = 0.998;
        }

        for (let i = 0; i < len; i++) {
            if (this.sampleCount >= 2) {
                const idx0 = Math.floor(this.readPtr);
                const idx1 = (idx0 + 1) % cap;
                const frac = this.readPtr - idx0;

                channel[i] = this.buffer[idx0] * (1.0 - frac) + this.buffer[idx1] * frac;

                this.readPtr = (this.readPtr + rate) % cap;
                this.sampleCount -= rate;
            } else if (this.sampleCount > 0) {
                const idx0 = Math.floor(this.readPtr);
                channel[i] = this.buffer[idx0];
                this.readPtr = (this.readPtr + 1) % cap;
                this.sampleCount--;
            } else {
                channel[i] = 0;
            }
        }

        if (this.sampleCount < 0) this.sampleCount = 0;

        return true;
    }
}

registerProcessor('nes-audio-processor', NesAudioProcessor);
