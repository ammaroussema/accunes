// mobile and desktop input system for the web version!!!

export const BUTTON_RIGHT  = 0x01;
export const BUTTON_LEFT   = 0x02;
export const BUTTON_DOWN   = 0x04;
export const BUTTON_UP     = 0x08;
export const BUTTON_START  = 0x10;
export const BUTTON_SELECT = 0x20;
export const BUTTON_B      = 0x40;
export const BUTTON_A      = 0x80;

export class NesInput {
    constructor(callbacks = {}) {
        this.callbacks = callbacks;
        this.p1Bits = 0;
        this.p2Bits = 0;
        this.isFastForward = false;
        this.hapticsEnabled = true;

        this.isTurboA = false;
        this.isTurboB = false;
        this.turboPulse = false;

        this.dpadPointerId = null;
        this.dpadCenter = { x: 0, y: 0 };
        this.dpadRadius = 65;
        this.activePointers = new Map();
        this.lastDpadBits = 0;

        this.hasExternalGamepad = false;

        this.keyBindings = {
            'ArrowRight': BUTTON_RIGHT,
            'KeyD':       BUTTON_RIGHT,
            'ArrowLeft':  BUTTON_LEFT,
            'KeyA':       BUTTON_LEFT,
            'ArrowDown':  BUTTON_DOWN,
            'KeyS':       BUTTON_DOWN,
            'ArrowUp':    BUTTON_UP,
            'KeyW':       BUTTON_UP,
            'KeyZ':       BUTTON_B,
            'KeyJ':       BUTTON_B,
            'KeyX':       BUTTON_A,
            'KeyK':       BUTTON_A,
            'ShiftLeft':  BUTTON_SELECT,
            'ShiftRight': BUTTON_SELECT,
            'KeyN':       BUTTON_SELECT,
            'Enter':      BUTTON_START,
            'KeyM':       BUTTON_START,
        };

        this.initKeyboard();
        this.initTouch();
    }

    setHaptics(enabled) {
        this.hapticsEnabled = enabled;
    }

    triggerHaptic(ms = 8) {
        if (!this.hapticsEnabled) return;
        try {
            if (navigator.vibrate) {
                navigator.vibrate(ms);
            }
        } catch {
        }
    }

    initKeyboard() {
        window.addEventListener('keydown', (e) => {
            if (e.code === 'Space') {
                this.isFastForward = true;
                e.preventDefault();
                return;
            }
            if (e.code === 'KeyR' && !e.ctrlKey && !e.metaKey) {
                if (this.callbacks.onReset) this.callbacks.onReset();
                e.preventDefault();
                return;
            }
            if (e.code === 'KeyP') {
                if (this.callbacks.onTogglePause) this.callbacks.onTogglePause();
                e.preventDefault();
                return;
            }
            if (e.code === 'KeyF') {
                if (this.callbacks.onToggleFullscreen) this.callbacks.onToggleFullscreen();
                e.preventDefault();
                return;
            }
            if (e.code === 'F5') {
                if (this.callbacks.onQuickSave) this.callbacks.onQuickSave();
                e.preventDefault();
                return;
            }
            if (e.code === 'F8') {
                if (this.callbacks.onQuickLoad) this.callbacks.onQuickLoad();
                e.preventDefault();
                return;
            }

            const btn = this.keyBindings[e.code];
            if (btn !== undefined) {
                this.p1Bits |= btn;
                if ((this.p1Bits & (BUTTON_LEFT | BUTTON_RIGHT)) === (BUTTON_LEFT | BUTTON_RIGHT)) {
                    this.p1Bits &= ~btn;
                }
                if ((this.p1Bits & (BUTTON_UP | BUTTON_DOWN)) === (BUTTON_UP | BUTTON_DOWN)) {
                    this.p1Bits &= ~btn;
                }
                e.preventDefault();
            }
        });

        window.addEventListener('keyup', (e) => {
            if (e.code === 'Space') {
                this.isFastForward = false;
                e.preventDefault();
                return;
            }
            const btn = this.keyBindings[e.code];
            if (btn !== undefined) {
                this.p1Bits &= ~btn;
                e.preventDefault();
            }
        });
    }

    initTouch() {
        const dpadContainer = document.getElementById('touch-dpad');
        const nub = document.getElementById('touch-dpad-nub');
        const dpadUp = document.getElementById('touch-up');
        const dpadDown = document.getElementById('touch-down');
        const dpadLeft = document.getElementById('touch-left');
        const dpadRight = document.getElementById('touch-right');

        const preventTouchDefault = (el) => {
            if (!el) return;
            el.addEventListener('touchstart', (e) => e.preventDefault(), { passive: false });
            el.addEventListener('touchmove', (e) => e.preventDefault(), { passive: false });
        };

        if (dpadContainer) {
            preventTouchDefault(dpadContainer);

            const calculateDpadDirection = (clientX, clientY) => {
                const rect = dpadContainer.getBoundingClientRect();
                const cx = rect.left + rect.width / 2;
                const cy = rect.top + rect.height / 2;
                const dx = clientX - cx;
                const dy = clientY - cy;
                const dist = Math.hypot(dx, dy);

                if (nub) {
                    const maxRadius = rect.width * 0.38;
                    const clampedDist = Math.min(dist, maxRadius);
                    const nx = dist > 0 ? (dx / dist) * clampedDist : 0;
                    const ny = dist > 0 ? (dy / dist) * clampedDist : 0;
                    nub.style.transform = `translate(calc(-50% + ${nx}px), calc(-50% + ${ny}px))`;
                }

                const deadzone = rect.width * 0.16;
                if (dist < deadzone) {
                    return 0;
                }

                let angle = Math.atan2(dy, dx) * (180 / Math.PI);
                if (angle < 0) angle += 360;

                let bits = 0;
                if (angle >= 337.5 || angle < 22.5) {
                    bits = BUTTON_RIGHT;
                } else if (angle >= 22.5 && angle < 67.5) {
                    bits = BUTTON_DOWN | BUTTON_RIGHT;
                } else if (angle >= 67.5 && angle < 112.5) {
                    bits = BUTTON_DOWN;
                } else if (angle >= 112.5 && angle < 157.5) {
                    bits = BUTTON_DOWN | BUTTON_LEFT;
                } else if (angle >= 157.5 && angle < 202.5) {
                    bits = BUTTON_LEFT;
                } else if (angle >= 202.5 && angle < 247.5) {
                    bits = BUTTON_UP | BUTTON_LEFT;
                } else if (angle >= 247.5 && angle < 292.5) {
                    bits = BUTTON_UP;
                } else if (angle >= 292.5 && angle < 337.5) {
                    bits = BUTTON_UP | BUTTON_RIGHT;
                }
                return bits;
            };

            const updateDpadVisuals = (bits) => {
                dpadUp?.classList.toggle('active', (bits & BUTTON_UP) !== 0);
                dpadDown?.classList.toggle('active', (bits & BUTTON_DOWN) !== 0);
                dpadLeft?.classList.toggle('active', (bits & BUTTON_LEFT) !== 0);
                dpadRight?.classList.toggle('active', (bits & BUTTON_RIGHT) !== 0);
            };

            const setDpadBits = (newBits) => {
                const dirMask = BUTTON_UP | BUTTON_DOWN | BUTTON_LEFT | BUTTON_RIGHT;
                this.p1Bits = (this.p1Bits & ~dirMask) | newBits;
                if (newBits !== this.lastDpadBits) {
                    if (newBits !== 0) this.triggerHaptic(6);
                    this.lastDpadBits = newBits;
                }
                updateDpadVisuals(newBits);
            };

            const handleDpadDown = (e) => {
                this.callbacks.onTouchActive?.();
                this.dpadPointerId = e.pointerId;
                try {
                    dpadContainer.setPointerCapture(e.pointerId);
                } catch {}
                dpadContainer.classList.add('touching');
                const bits = calculateDpadDirection(e.clientX, e.clientY);
                setDpadBits(bits);
            };

            const handleDpadMove = (e) => {
                if (this.dpadPointerId !== e.pointerId) return;
                const bits = calculateDpadDirection(e.clientX, e.clientY);
                setDpadBits(bits);
            };

            const handleDpadUp = (e) => {
                if (this.dpadPointerId !== e.pointerId) return;
                this.dpadPointerId = null;
                dpadContainer.classList.remove('touching');
                if (nub) nub.style.transform = 'translate(-50%, -50%)';
                setDpadBits(0);
            };

            dpadContainer.addEventListener('pointerdown', handleDpadDown);
            dpadContainer.addEventListener('pointermove', handleDpadMove);
            dpadContainer.addEventListener('pointerup', handleDpadUp);
            dpadContainer.addEventListener('pointercancel', handleDpadUp);
        }

        const bindPointerButton = (id, bitmask, onToggle = null) => {
            const el = document.getElementById(id);
            if (!el) return;
            preventTouchDefault(el);

            const onDown = (e) => {
                this.callbacks.onTouchActive?.();
                try {
                    el.setPointerCapture(e.pointerId);
                } catch {}
                el.classList.add('active');
                this.triggerHaptic(8);

                if (onToggle) {
                    onToggle(true);
                } else if (bitmask) {
                    this.p1Bits |= bitmask;
                }
            };

            const onUp = (e) => {
                el.classList.remove('active');
                if (onToggle) {
                    onToggle(false);
                } else if (bitmask) {
                    this.p1Bits &= ~bitmask;
                }
            };

            el.addEventListener('pointerdown', onDown);
            el.addEventListener('pointerup', onUp);
            el.addEventListener('pointercancel', onUp);
        };
        bindPointerButton('touch-b', BUTTON_B);
        bindPointerButton('touch-a', BUTTON_A);

        bindPointerButton('touch-ba', BUTTON_B | BUTTON_A);

        bindPointerButton('touch-tb', 0, (active) => {
            this.isTurboB = active;
        });
        bindPointerButton('touch-ta', 0, (active) => {
            this.isTurboA = active;
        });

        bindPointerButton('touch-select', BUTTON_SELECT);
        bindPointerButton('touch-start', BUTTON_START);

        const ffBtn = document.getElementById('touch-ff');
        if (ffBtn) {
            preventTouchDefault(ffBtn);
            ffBtn.addEventListener('pointerdown', (e) => {
                this.callbacks.onTouchActive?.();
                this.isFastForward = !this.isFastForward;
                ffBtn.classList.toggle('active', this.isFastForward);
                this.triggerHaptic(10);
            });
        }
    }

    updateTurbo(pulseActive) {
        this.turboPulse = pulseActive;
    }

    pollGamepad() {
        if (!navigator.getGamepads) return 0;
        const gamepads = navigator.getGamepads();
        if (!gamepads) return 0;

        let gpBits = 0;
        const gp = gamepads[0];
        if (gp && gp.connected) {
            if (gp.buttons[14]?.pressed || gp.axes[0] < -0.4) gpBits |= BUTTON_LEFT;
            if (gp.buttons[15]?.pressed || gp.axes[0] > 0.4)  gpBits |= BUTTON_RIGHT;
            if (gp.buttons[12]?.pressed || gp.axes[1] < -0.4) gpBits |= BUTTON_UP;
            if (gp.buttons[13]?.pressed || gp.axes[1] > 0.4)  gpBits |= BUTTON_DOWN;

            if (gp.buttons[0]?.pressed || gp.buttons[2]?.pressed) gpBits |= BUTTON_B;
            if (gp.buttons[1]?.pressed || gp.buttons[3]?.pressed) gpBits |= BUTTON_A;

            if (gp.buttons[8]?.pressed) gpBits |= BUTTON_SELECT; 
            if (gp.buttons[9]?.pressed) gpBits |= BUTTON_START;

            this.isFastForward = gp.buttons[7]?.pressed || this.isFastForward;

            if (gpBits !== 0 && !this.hasExternalGamepad) {
                this.hasExternalGamepad = true;
                this.callbacks.onGamepadActive?.();
            }
        } else if (this.hasExternalGamepad) {
            this.hasExternalGamepad = false;
        }

        return gpBits;
    }

    getControllerP1() {
        let bits = this.p1Bits | this.pollGamepad();

        if (this.isTurboA) {
            if (this.turboPulse) {
                bits |= BUTTON_A;
            } else {
                bits &= ~BUTTON_A;
            }
        }
        if (this.isTurboB) {
            if (this.turboPulse) {
                bits |= BUTTON_B;
            } else {
                bits &= ~BUTTON_B;
            }
        }

        return bits;
    }

    getControllerP2() {
        return this.p2Bits;
    }
}
