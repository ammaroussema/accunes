// audio system web port

const WORKLET_CODE = `
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
`;

export class NesAudio {
    constructor() {
        this.ctx = null;
        this.gainNode = null;
        this.workletNode = null;
        this.scriptNode = null;
        this.buffer = new Float32Array(32768);
        this.writePtr = 0;
        this.readPtr = 0.0;
        this.sampleCount = 0;
        this.isMuted = false;
        this.volume = 0.8;
        this.isInitialized = false;
        this.initPromise = null;
    }

    async init() {
        if (this.isInitialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = (async () => {
            const AudioContext = window.AudioContext || window.webkitAudioContext;
            if (!AudioContext) {
                console.warn('Web Audio API not supported in this browser');
                return;
            }

            if (!this.ctx) {
                this.ctx = new AudioContext({ latencyHint: 'interactive' });
            }

            if (!this.gainNode) {
                this.gainNode = this.ctx.createGain();
                this.gainNode.gain.setValueAtTime(this.volume, this.ctx.currentTime);
                this.gainNode.connect(this.ctx.destination);
            }

            let workletSuccess = false;
            if (this.ctx.audioWorklet) {
                try {
                    const workletUrl = new URL('./audio-processor.js', import.meta.url);
                    await this.ctx.audioWorklet.addModule(workletUrl);
                    workletSuccess = true;
                } catch (e1) {
                    try {
                        const blob = new Blob([WORKLET_CODE], { type: 'application/javascript' });
                        const blobUrl = URL.createObjectURL(blob);
                        await this.ctx.audioWorklet.addModule(blobUrl);
                        URL.revokeObjectURL(blobUrl);
                        workletSuccess = true;
                    } catch (e2) {
                        console.warn('AudioWorklet module loading failed, falling back to ScriptProcessor:', e2);
                    }
                }
            }

            if (workletSuccess) {
                this.workletNode = new AudioWorkletNode(this.ctx, 'nes-audio-processor');
                this.workletNode.connect(this.gainNode);
            } else {
                this.scriptNode = this.ctx.createScriptProcessor(2048, 0, 1);
                this.scriptNode.onaudioprocess = (e) => this.onAudioProcess(e);
                this.scriptNode.connect(this.gainNode);
            }

            this.isInitialized = true;
        })();

        return this.initPromise;
    }

    getSampleRate() {
        if (!this.ctx) {
            const AudioContext = window.AudioContext || window.webkitAudioContext;
            if (AudioContext) {
                try {
                    this.ctx = new AudioContext({ latencyHint: 'interactive' });
                    return this.ctx.sampleRate;
                } catch (e) {
                    return 44100;
                }
            }
            return 44100;
        }
        return this.ctx.sampleRate;
    }

    resume() {
        if (this.ctx && this.ctx.state === 'suspended') {
            this.ctx.resume();
        }
    }

    suspend() {
        if (this.ctx && this.ctx.state === 'running') {
            this.ctx.suspend().catch(() => {});
        }
    }

    queueSamples(samples) {
        if (!samples || samples.length === 0) return;

        if (this.workletNode) {
            this.workletNode.port.postMessage(samples);
            return;
        }

        if (!this.scriptNode) return;

        const len = samples.length;
        const cap = this.buffer.length;

        for (let i = 0; i < len; i++) {
            this.buffer[this.writePtr] = samples[i];
            this.writePtr = (this.writePtr + 1) % cap;
        }
        this.sampleCount = Math.min(cap, this.sampleCount + len);
    }

    onAudioProcess(e) {
        const output = e.outputBuffer.getChannelData(0);
        const len = output.length;
        const cap = this.buffer.length;

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

                output[i] = this.buffer[idx0] * (1.0 - frac) + this.buffer[idx1] * frac;

                this.readPtr = (this.readPtr + rate) % cap;
                this.sampleCount -= rate;
            } else if (this.sampleCount > 0) {
                const idx0 = Math.floor(this.readPtr);
                output[i] = this.buffer[idx0];
                this.readPtr = (this.readPtr + 1) % cap;
                this.sampleCount--;
            } else {
                output[i] = 0;
            }
        }

        if (this.sampleCount < 0) this.sampleCount = 0;
    }

    setVolume(val) {
        this.volume = Math.max(0, Math.min(1, val));
        if (this.gainNode && !this.isMuted) {
            this.gainNode.gain.setValueAtTime(this.volume, this.ctx.currentTime);
        }
    }

    toggleMute() {
        this.isMuted = !this.isMuted;
        if (this.gainNode) {
            this.gainNode.gain.setValueAtTime(this.isMuted ? 0 : this.volume, this.ctx.currentTime);
        }
        return this.isMuted;
    }
}
