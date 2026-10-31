// AccuNES webport main loop!!!

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
        this.turboCycle = 0;

        this.wakeLock = null;

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
            this.initServiceWorker();

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
                onGamepadActive: () => {
                    document.getElementById('touch-controls')?.classList.add('faded');
                },
                onTouchActive: () => {
                    document.getElementById('touch-controls')?.classList.remove('faded');
                }
            });

            this.initUI();
            this.initLifecycle();
            await this.refreshRecentRoms();

            this.updateStatus('Ready! Drag & drop an NES ROM or pick from recent games.');
        } catch (e) {
            console.error('Initialization failed:', e);
            this.updateStatus('Error loading AccuNES Wasm: ' + e.message);
        }
    }

    initServiceWorker() {
        if ('serviceWorker' in navigator) {
            window.addEventListener('load', () => {
                navigator.serviceWorker.register('./sw.js').then((reg) => {
                    console.log('PWA ServiceWorker registered with scope:', reg.scope);
                }).catch((err) => {
                    console.warn('PWA ServiceWorker registration failed:', err);
                });
            });
        }
    }

    initLifecycle() {
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                this.releaseWakeLock();
                if (this.audio) this.audio.suspend();
                this.syncSram();
            } else {
                if (this.isRunning && !this.isPaused) {
                    this.requestWakeLock();
                    if (this.audio) this.audio.resume();
                }
                this.lastFrameTime = performance.now();
                this.frameAccumulator = 0;
            }
        });

        window.addEventListener('focus', () => {
            this.lastFrameTime = performance.now();
            this.frameAccumulator = 0;
            if (this.isRunning && !this.isPaused) {
                this.requestWakeLock();
                if (this.audio) this.audio.resume();
            }
        });

        window.addEventListener('blur', () => {
            if (this.isRunning) {
                this.syncSram();
            }
        });
    }

    async requestWakeLock() {
        if ('wakeLock' in navigator && !this.wakeLock) {
            try {
                this.wakeLock = await navigator.wakeLock.request('screen');
                this.wakeLock.addEventListener('release', () => {
                    this.wakeLock = null;
                });
            } catch (err) {
            }
        }
    }

    releaseWakeLock() {
        if (this.wakeLock) {
            this.wakeLock.release().catch(() => {});
            this.wakeLock = null;
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
        const modalFilterSelect = document.getElementById('modal-filter-select');
        const onFilterChange = (val) => {
            this.renderer.setFilter(val);
            if (filterSelect) filterSelect.value = val;
            if (modalFilterSelect) modalFilterSelect.value = val;
            localStorage.setItem('accunes_filter', val);
        };
        filterSelect?.addEventListener('change', (e) => onFilterChange(e.target.value));
        modalFilterSelect?.addEventListener('change', (e) => onFilterChange(e.target.value));

        const savedFilter = localStorage.getItem('accunes_filter') || 'pixelated';
        onFilterChange(savedFilter);

        const aspectSelect = document.getElementById('aspect-select');
        const modalAspectSelect = document.getElementById('modal-aspect-select');
        const screenContainer = document.getElementById('screen-container');
        const onAspectChange = (val) => {
            if (screenContainer) {
                screenContainer.className = `screen-container aspect-${val}`;
            }
            if (aspectSelect) aspectSelect.value = val;
            if (modalAspectSelect) modalAspectSelect.value = val;
            localStorage.setItem('accunes_aspect', val);
        };
        aspectSelect?.addEventListener('change', (e) => onAspectChange(e.target.value));
        modalAspectSelect?.addEventListener('change', (e) => onAspectChange(e.target.value));

        const savedAspect = localStorage.getItem('accunes_aspect') || '4-3';
        onAspectChange(savedAspect);

        const themeSelect = document.getElementById('theme-select');
        const modalThemeSelect = document.getElementById('modal-theme-select');
        const onThemeChange = (val) => {
            this.setTheme(val);
            if (themeSelect) themeSelect.value = val;
            if (modalThemeSelect) modalThemeSelect.value = val;
        };
        themeSelect?.addEventListener('change', (e) => onThemeChange(e.target.value));
        modalThemeSelect?.addEventListener('change', (e) => onThemeChange(e.target.value));

        const savedTheme = localStorage.getItem('accunes_theme') || 'dark';
        if (themeSelect) themeSelect.value = savedTheme;
        if (modalThemeSelect) modalThemeSelect.value = savedTheme;

        const volumeSlider = document.getElementById('volume-slider');
        volumeSlider?.addEventListener('input', (e) => {
            this.audio.setVolume(parseFloat(e.target.value));
        });

        const muteBtn = document.getElementById('btn-mute');
        const modalMuteBtn = document.getElementById('modal-btn-mute');
        const onMuteToggle = () => {
            const isMuted = this.audio.toggleMute();
            const label = isMuted ? 'Unmute' : 'Mute';
            if (muteBtn) muteBtn.textContent = label;
            if (modalMuteBtn) modalMuteBtn.textContent = label;
        };
        muteBtn?.addEventListener('click', onMuteToggle);
        modalMuteBtn?.addEventListener('click', onMuteToggle);

        const resumeAudioHandler = () => this.audio?.resume();
        window.addEventListener('click', resumeAudioHandler, { once: true });
        window.addEventListener('keydown', resumeAudioHandler, { once: true });
        window.addEventListener('touchstart', resumeAudioHandler, { once: true });

        const mobileModal = document.getElementById('mobile-menu-modal');
        const openModal = () => {
            if (mobileModal) {
                mobileModal.style.display = 'flex';
                this.refreshRecentRoms();
            }
        };
        const closeModal = () => {
            if (mobileModal) mobileModal.style.display = 'none';
        };

        document.getElementById('btn-mobile-menu')?.addEventListener('click', openModal);
        document.getElementById('touch-menu-pill')?.addEventListener('click', openModal);
        document.getElementById('btn-close-modal')?.addEventListener('click', closeModal);
        document.getElementById('mobile-menu-backdrop')?.addEventListener('click', closeModal);

        document.getElementById('modal-btn-pause')?.addEventListener('click', () => {
            this.togglePause();
            closeModal();
        });
        document.getElementById('modal-btn-reset')?.addEventListener('click', () => {
            this.reset();
            closeModal();
        });
        document.getElementById('modal-btn-save')?.addEventListener('click', () => {
            this.quickSave();
            closeModal();
        });
        document.getElementById('modal-btn-load')?.addEventListener('click', () => {
            this.quickLoad();
            closeModal();
        });
        document.getElementById('modal-btn-fullscreen')?.addEventListener('click', () => {
            this.toggleFullscreen();
            closeModal();
        });

        const hapticsCheck = document.getElementById('modal-haptics-check');
        if (hapticsCheck) {
            const savedHaptics = localStorage.getItem('accunes_haptics') !== 'false';
            hapticsCheck.checked = savedHaptics;
            this.input.setHaptics(savedHaptics);
            hapticsCheck.addEventListener('change', (e) => {
                this.input.setHaptics(e.target.checked);
                localStorage.setItem('accunes_haptics', e.target.checked ? 'true' : 'false');
            });
        }

        const opacitySlider = document.getElementById('modal-opacity-slider');
        if (opacitySlider) {
            const savedOpacity = localStorage.getItem('accunes_controls_opacity') || '0.9';
            opacitySlider.value = savedOpacity;
            document.documentElement.style.setProperty('--controls-opacity', savedOpacity);
            opacitySlider.addEventListener('input', (e) => {
                document.documentElement.style.setProperty('--controls-opacity', e.target.value);
                localStorage.setItem('accunes_controls_opacity', e.target.value);
            });
        }
    }

    setTheme(themeName) {
        document.body.className = document.body.className.replace(/\btheme-[a-z0-9_-]+\b/g, '').trim();
        document.body.classList.add(`theme-${themeName}`);
        localStorage.setItem('accunes_theme', themeName);
        if (this.emulator && typeof this.emulator.set_theme === 'function') {
            this.emulator.set_theme(themeName);
        }
    }

    async refreshRecentRoms() {
        if (!this.storage) return;
        const roms = await this.storage.listRoms();

        const shelfContainer = document.getElementById('recent-roms-container');
        const shelfList = document.getElementById('recent-roms-list');
        if (shelfContainer && shelfList) {
            if (roms.length > 0) {
                shelfContainer.style.display = 'block';
                shelfList.innerHTML = '';
                roms.slice(0, 5).forEach((r) => {
                    const btn = document.createElement('button');
                    btn.className = 'recent-rom-chip';
                    btn.textContent = r.name.replace(/\.[^/.]+$/, '');
                    btn.title = `Play ${r.name}`;
                    btn.addEventListener('click', (e) => {
                        e.stopPropagation();
                        this.launchStoredRom(r.name);
                    });
                    shelfList.appendChild(btn);
                });
            } else {
                shelfContainer.style.display = 'none';
            }
        }

        const modalList = document.getElementById('modal-roms-list');
        if (modalList) {
            modalList.innerHTML = '';
            if (roms.length === 0) {
                modalList.innerHTML = '<div style="color:var(--text-dim);font-size:0.75rem;">No games saved yet.</div>';
            } else {
                roms.forEach((r) => {
                    const row = document.createElement('div');
                    row.className = 'modal-rom-item';

                    const nameSpan = document.createElement('span');
                    nameSpan.className = 'modal-rom-name';
                    nameSpan.textContent = r.name;
                    nameSpan.addEventListener('click', () => {
                        document.getElementById('mobile-menu-modal').style.display = 'none';
                        this.launchStoredRom(r.name);
                    });

                    const delBtn = document.createElement('button');
                    delBtn.className = 'modal-rom-del';
                    delBtn.textContent = '✕';
                    delBtn.title = 'Remove from storage';
                    delBtn.addEventListener('click', async (e) => {
                        e.stopPropagation();
                        await this.storage.deleteRom(r.name);
                        this.refreshRecentRoms();
                    });

                    row.appendChild(nameSpan);
                    row.appendChild(delBtn);
                    modalList.appendChild(row);
                });
            }
        }
    }

    async launchStoredRom(name) {
        this.updateStatus(`Loading ${name} from storage...`);
        const bytes = await this.storage.loadRom(name);
        if (bytes) {
            await this.startRom(name, bytes);
        } else {
            this.flashStatus(`Could not load ${name}`);
        }
    }

    async loadRomFile(file) {
        this.updateStatus(`Loading ${file.name}...`);
        try {
            const buffer = await file.arrayBuffer();
            const bytes = new Uint8Array(buffer);
            await this.storage.saveRom(file.name, bytes);
            await this.refreshRecentRoms();
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

            this.requestWakeLock();

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
            this.frameAccumulator = 0;
            return;
        }

        if (!this.lastFrameTime) this.lastFrameTime = timestamp;
        const elapsed = timestamp - this.lastFrameTime;
        this.lastFrameTime = timestamp;

        this.turboCycle = (this.turboCycle + 1) % 4;
        this.input.updateTurbo(this.turboCycle < 2);

        const NES_FRAME_DURATION = 1000 / 60.0988;
        let framesToAdvance = 0;

        if (this.input.isFastForward) {
            framesToAdvance = 2;
            this.frameAccumulator = 0;
        } else if (elapsed > 60) {
            framesToAdvance = 1;
            this.frameAccumulator = 0;
        } else {
            this.frameAccumulator += elapsed;
            if (this.frameAccumulator >= NES_FRAME_DURATION - 2.5) {
                framesToAdvance = 1;
                this.frameAccumulator = Math.min(this.frameAccumulator - NES_FRAME_DURATION, NES_FRAME_DURATION * 0.4);
                if (this.frameAccumulator < 0) this.frameAccumulator = 0;
            }
        }

        for (let i = 0; i < framesToAdvance; i++) {
            this.emulator.set_controller_p1(this.input.getControllerP1());
            this.emulator.set_controller_p2(this.input.getControllerP2());
            this.emulator.step_frame();

            if (typeof this.emulator.audio_samples_ready === 'function') {
                const count = this.emulator.audio_samples_ready();
                if (count > 0) {
                    const ptr = this.emulator.get_audio_buffer_ptr();
                    const samples = new Float32Array(this.wasm.memory.buffer, ptr, count);
                    this.audio.queueSamples(samples);
                }
            } else {
                const samples = this.emulator.get_audio_samples();
                this.audio.queueSamples(samples);
            }
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
        const label = this.isPaused ? 'Resume' : 'Pause';

        const btn = document.getElementById('btn-pause');
        if (btn) btn.textContent = label;
        const modalBtn = document.getElementById('modal-btn-pause');
        if (modalBtn) modalBtn.textContent = label;

        if (this.isPaused) {
            this.releaseWakeLock();
            this.audio?.suspend();
        } else {
            this.requestWakeLock();
            this.audio?.resume();
            this.lastFrameTime = performance.now();
            this.frameAccumulator = 0;
        }

        this.updateStatus(this.isPaused ? 'Paused' : `Playing: ${this.currentRomName}`);
    }

    reset() {
        if (this.emulator) {
            this.emulator.reset();
            this.updateStatus('Reset');
        }
    }

    toggleFullscreen() {
        const app = document.getElementById('app-container') || document.documentElement;
        if (!document.fullscreenElement) {
            app.requestFullscreen?.().catch(() => {
                document.getElementById('screen-container')?.requestFullscreen?.();
            });
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
