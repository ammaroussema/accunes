// audio system web port

const WORKLET_CODE = `
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
`;

export class NesAudio {
    constructor() {
        this.ctx = null;
        this.gainNode = null;
        this.workletNode = null;
        this.scriptNode = null;
        this.buffer = new Float32Array(32768);
        this.writePtr = 0;
        this.readPtr = 0;
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
                // Fallback to ScriptProcessorNode only if AudioWorklet is unavailable
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

        if (this.sampleCount < len) {
            for (let i = 0; i < len; i++) {
                if (this.sampleCount > 0) {
                    output[i] = this.buffer[this.readPtr];
                    this.readPtr = (this.readPtr + 1) % cap;
                    this.sampleCount--;
                } else {
                    output[i] = 0;
                }
            }
        } else {
            for (let i = 0; i < len; i++) {
                output[i] = this.buffer[this.readPtr];
                this.readPtr = (this.readPtr + 1) % cap;
            }
            this.sampleCount -= len;
        }

        if (this.sampleCount > 8192) {
            const skip = this.sampleCount - 4096;
            this.readPtr = (this.readPtr + skip) % cap;
            this.sampleCount = 4096;
        }
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
