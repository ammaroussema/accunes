// input system web port!!!

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
        }

        return gpBits;
    }

    getControllerP1() {
        return this.p1Bits | this.pollGamepad();
    }

    getControllerP2() {
        return this.p2Bits;
    }

    initTouch() {
        const bindTouchBtn = (id, mask) => {
            const el = document.getElementById(id);
            if (!el) return;
            const onDown = (e) => {
                e.preventDefault();
                this.p1Bits |= mask;
                el.classList.add('active');
                if (navigator.vibrate) navigator.vibrate(8);
            };
            const onUp = (e) => {
                e.preventDefault();
                this.p1Bits &= ~mask;
                el.classList.remove('active');
            };
            el.addEventListener('touchstart', onDown, { passive: false });
            el.addEventListener('touchend', onUp, { passive: false });
            el.addEventListener('mousedown', onDown);
            el.addEventListener('mouseup', onUp);
            el.addEventListener('mouseleave', onUp);
        };

        bindTouchBtn('touch-up', BUTTON_UP);
        bindTouchBtn('touch-down', BUTTON_DOWN);
        bindTouchBtn('touch-left', BUTTON_LEFT);
        bindTouchBtn('touch-right', BUTTON_RIGHT);
        bindTouchBtn('touch-a', BUTTON_A);
        bindTouchBtn('touch-b', BUTTON_B);
        bindTouchBtn('touch-select', BUTTON_SELECT);
        bindTouchBtn('touch-start', BUTTON_START);
    }
}
