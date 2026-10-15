// web port main loop!!!

import init, { WasmEmulator } from '../pkg/accunes.js';
import { NesRenderer } from './renderer.js';
import { NesAudio } from './audio.js';
import { NesInput } from './input.js';
import { NesStorage } from './storage.js';

class AccuNESApp {
    constructor() {
        this.wasm = null;
        this.emulator = null;
        this.renderer = null;
        this.audio = null;
        this.input = null;
        this.storage = null;

        this.currentRomName = '';
        this.isRunning = false;
        this.isPaused = false;
        this.lastFrameTime = 0;
        this.frameAccumulator = 0;
        this.fpsCount = 0;
        this.lastFpsCalc = 0;
        this.sramDirtyTimer = 0;

        this.canvas = document.getElementById('nes-canvas');
        this.fpsDisplay = document.getElementById('fps-display');
        this.statusDisplay = document.getElementById('status-display');
        this.dropZone = document.getElementById('drop-zone');

        const savedTheme = localStorage.getItem('accunes_theme') || 'dark';
        this.setTheme(savedTheme);
    }

    async init() {
        this.updateStatus('Loading WebAssembly core...');
        try {
            this.wasm = await init();
            this.storage = new NesStorage();
            await this.storage.init();

            this.renderer = new NesRenderer(this.canvas);
            this.audio = new NesAudio();

            this.input = new NesInput({
                onReset: () => this.reset(),
                onTogglePause: () => this.togglePause(),
                onToggleFullscreen: () => this.toggleFullscreen(),
                onQuickSave: () => this.quickSave(),
                onQuickLoad: () => this.quickLoad(),
            });

            this.initUI();
            this.updateStatus('Ready! Drag & drop an NES ROM to play.');
        } catch (e) {
            console.error('Initialization failed:', e);
            this.updateStatus('Error loading AccuNES Wasm: ' + e.message);
        }
    }

    initUI() {
        const fileInput = document.getElementById('rom-file-input');
        const openRomBtn = document.getElementById('btn-open-rom');

        if (openRomBtn && fileInput) {
            openRomBtn.addEventListener('click', () => fileInput.click());
            fileInput.addEventListener('change', (e) => {
                const file = e.target.files[0];
                if (file) this.loadRomFile(file);
            });
        }

        window.addEventListener('dragover', (e) => {
            e.preventDefault();
            this.dropZone.classList.add('dragover');
        });
        window.addEventListener('dragleave', (e) => {
            if (e.relatedTarget === null) {
                this.dropZone.classList.remove('dragover');
            }
        });
        window.addEventListener('drop', (e) => {
            e.preventDefault();
            this.dropZone.classList.remove('dragover');
            const file = e.dataTransfer.files[0];
            if (file) this.loadRomFile(file);
        });

        document.getElementById('btn-pause')?.addEventListener('click', () => this.togglePause());
        document.getElementById('btn-reset')?.addEventListener('click', () => this.reset());
        document.getElementById('btn-fullscreen')?.addEventListener('click', () => this.toggleFullscreen());
        document.getElementById('btn-quicksave')?.addEventListener('click', () => this.quickSave());
        document.getElementById('btn-quickload')?.addEventListener('click', () => this.quickLoad());
        document.getElementById('btn-export-sav')?.addEventListener('click', () => this.exportSram());

        const savInput = document.getElementById('sav-file-input');
        document.getElementById('btn-import-sav')?.addEventListener('click', () => savInput?.click());
        savInput?.addEventListener('change', (e) => {
            const file = e.target.files[0];
            if (file) this.importSramFile(file);
        });

        const filterSelect = document.getElementById('filter-select');
        filterSelect?.addEventListener('change', (e) => {
            this.renderer.setFilter(e.target.value);
        });

        const aspectSelect = document.getElementById('aspect-select');
        const screenContainer = document.getElementById('screen-container');
        aspectSelect?.addEventListener('change', (e) => {
            if (screenContainer) {
                screenContainer.className = `screen-container aspect-${e.target.value}`;
            }
        });

        const themeSelect = document.getElementById('theme-select');
        const savedTheme = localStorage.getItem('accunes_theme') || 'dark';
        if (themeSelect) {
            themeSelect.value = savedTheme;
            this.setTheme(savedTheme);
            themeSelect.addEventListener('change', (e) => {
                this.setTheme(e.target.value);
            });
        } else {
            this.setTheme(savedTheme);
        }

        const volumeSlider = document.getElementById('volume-slider');
        volumeSlider?.addEventListener('input', (e) => {
            this.audio.setVolume(parseFloat(e.target.value));
        });

        const muteBtn = document.getElementById('btn-mute');
        muteBtn?.addEventListener('click', () => {
            const isMuted = this.audio.toggleMute();
            muteBtn.textContent = isMuted ? 'Unmute' : 'Mute';
        });

        window.addEventListener('click', () => this.audio?.resume(), { once: true });
        window.addEventListener('keydown', () => this.audio?.resume(), { once: true });
    }

    setTheme(themeName) {
        document.body.className = document.body.className.replace(/\btheme-[a-z0-9_-]+\b/g, '').trim();
        document.body.classList.add(`theme-${themeName}`);
        localStorage.setItem('accunes_theme', themeName);
        if (this.emulator && typeof this.emulator.set_theme === 'function') {
            this.emulator.set_theme(themeName);
        }
    }

    async loadRomFile(file) {
        this.updateStatus(`Loading ${file.name}...`);
        try {
            const buffer = await file.arrayBuffer();
            const bytes = new Uint8Array(buffer);
            await this.startRom(file.name, bytes);
        } catch (e) {
            console.error('Failed to load ROM:', e);
            this.updateStatus(`Error loading ROM: ${e}`);
        }
    }

    async startRom(name, bytes) {
        await this.syncSram();

        await this.audio.init();
        const sampleRate = this.audio.getSampleRate();

        if (!this.emulator) {
            this.emulator = new WasmEmulator(sampleRate);
            const savedTheme = localStorage.getItem('accunes_theme') || 'dark';
            if (typeof this.emulator.set_theme === 'function') {
                this.emulator.set_theme(savedTheme);
            }
        }

        try {
            this.emulator.load_rom(name, bytes);
            this.currentRomName = name;

            const savedSram = await this.storage.loadSram(name);
            if (savedSram) {
                this.emulator.load_sram(savedSram);
                console.log(`Loaded saved SRAM for ${name}`);
            }

            this.isRunning = true;
            this.isPaused = false;
            this.lastFrameTime = performance.now();
            this.frameAccumulator = 0;
            this.fpsCount = 0;
            this.lastFpsCalc = performance.now();
            this.dropZone.style.display = 'none';
            this.updateStatus(`Playing: ${name}`);

            if (!this.loopStarted) {
                this.loopStarted = true;
                requestAnimationFrame((t) => this.loop(t));
            }
        } catch (e) {
            console.error('Core error loading ROM:', e);
            this.updateStatus(`Core error: ${e}`);
        }
    }

    loop(timestamp) {
        requestAnimationFrame((t) => this.loop(t));

        if (!this.isRunning || this.isPaused) {
            this.lastFrameTime = timestamp;
            return;
        }

        if (!this.lastFrameTime) this.lastFrameTime = timestamp;
        const elapsed = Math.min(timestamp - this.lastFrameTime, 100);
        this.lastFrameTime = timestamp;
        this.frameAccumulator += elapsed;

        const NES_FRAME_DURATION = 1000 / 60.0988;
        let framesToAdvance = 0;

        if (this.input.isFastForward) {
            framesToAdvance = 3;
            this.frameAccumulator = 0;
        } else {
            while (this.frameAccumulator >= NES_FRAME_DURATION && framesToAdvance < 3) {
                framesToAdvance++;
                this.frameAccumulator -= NES_FRAME_DURATION;
            }
        }

        for (let i = 0; i < framesToAdvance; i++) {
            this.emulator.set_controller_p1(this.input.getControllerP1());
            this.emulator.set_controller_p2(this.input.getControllerP2());
            this.emulator.step_frame();

            const samples = this.emulator.get_audio_samples();
            this.audio.queueSamples(samples);
        }

        if (framesToAdvance > 0) {
            const ptr = this.emulator.get_framebuffer_ptr();
            this.renderer.render(this.wasm.memory.buffer, ptr);
        }

        this.fpsCount += framesToAdvance;
        if (timestamp - this.lastFpsCalc >= 1000) {
            if (this.fpsDisplay) {
                this.fpsDisplay.textContent = `${this.fpsCount} FPS`;
            }
            this.fpsCount = 0;
            this.lastFpsCalc = timestamp;

            this.sramDirtyTimer++;
            if (this.sramDirtyTimer >= 5) {
                this.sramDirtyTimer = 0;
                this.syncSram();
            }
        }
    }

    async syncSram() {
        if (!this.emulator || !this.currentRomName) return;
        const sram = this.emulator.get_sram();
        if (sram && sram.length > 0) {
            await this.storage.saveSram(this.currentRomName, sram);
        }
    }

    togglePause() {
        if (!this.isRunning) return;
        this.isPaused = !this.isPaused;
        const btn = document.getElementById('btn-pause');
        if (btn) btn.textContent = this.isPaused ? 'Resume' : 'Pause';
        this.updateStatus(this.isPaused ? 'Paused' : `Playing: ${this.currentRomName}`);
    }

    reset() {
        if (this.emulator) {
            this.emulator.reset();
            this.updateStatus('Reset');
        }
    }

    toggleFullscreen() {
        const container = document.getElementById('screen-container');
        if (!document.fullscreenElement) {
            container?.requestFullscreen?.();
        } else {
            document.exitFullscreen?.();
        }
    }

    async quickSave() {
        if (!this.emulator || !this.currentRomName) return;
        const state = this.emulator.save_state();
        await this.storage.saveState(this.currentRomName, 0, state);
        this.flashStatus('State Saved (Slot 0)');
    }

    async quickLoad() {
        if (!this.emulator || !this.currentRomName) return;
        const state = await this.storage.loadState(this.currentRomName, 0);
        if (state) {
            this.emulator.load_state(state);
            this.flashStatus('State Loaded (Slot 0)');
        } else {
            this.flashStatus('No save state found!');
        }
    }

    exportSram() {
        if (!this.emulator || !this.currentRomName) return;
        const sram = this.emulator.get_sram();
        if (sram) {
            this.storage.downloadSramFile(this.currentRomName, sram);
        } else {
            alert('This game does not use battery-backed save RAM.');
        }
    }

    async importSramFile(file) {
        if (!this.emulator) return;
        const buf = await file.arrayBuffer();
        const bytes = new Uint8Array(buf);
        this.emulator.load_sram(bytes);
        await this.storage.saveSram(this.currentRomName, bytes);
        this.flashStatus('Imported .sav successfully!');
    }

    updateStatus(msg) {
        if (this.statusDisplay) this.statusDisplay.textContent = msg;
    }

    flashStatus(msg) {
        this.updateStatus(msg);
        setTimeout(() => {
            if (this.isRunning && !this.isPaused) {
                this.updateStatus(`Playing: ${this.currentRomName}`);
            }
        }, 2000);
    }
}

function startApp() {
    const app = new AccuNESApp();
    app.init();
}

if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', startApp);
} else {
    startApp();
}
