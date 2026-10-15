// extra audio processing stuff!!!
class NesAudioProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.buffer = new Float32Array(32768);
        this.writePtr = 0;
        this.readPtr = 0;
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

        for (let i = 0; i < len; i++) {
            if (this.sampleCount > 0) {
                channel[i] = this.buffer[this.readPtr];
                this.readPtr = (this.readPtr + 1) % cap;
                this.sampleCount--;
            } else {
                channel[i] = 0;
            }
        }

        if (this.sampleCount > 8192) {
            const skip = this.sampleCount - 4096;
            this.readPtr = (this.readPtr + skip) % cap;
            this.sampleCount = 4096;
        }

        return true;
    }
}

registerProcessor('nes-audio-processor', NesAudioProcessor);
