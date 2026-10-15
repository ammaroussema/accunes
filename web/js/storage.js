// save and settings support for webport!!!

const DB_NAME = 'AccuNesDB';
const DB_VERSION = 1;
const STORE_SRAM = 'sram';
const STORE_STATES = 'savestates';

export class NesStorage {
    constructor() {
        this.db = null;
    }

    async init() {
        return new Promise((resolve) => {
            const req = indexedDB.open(DB_NAME, DB_VERSION);
            req.onupgradeneeded = (e) => {
                const db = e.target.result;
                if (!db.objectStoreNames.contains(STORE_SRAM)) {
                    db.createObjectStore(STORE_SRAM);
                }
                if (!db.objectStoreNames.contains(STORE_STATES)) {
                    db.createObjectStore(STORE_STATES);
                }
            };
            req.onsuccess = (e) => {
                this.db = e.target.result;
                resolve();
            };
            req.onerror = () => {
                console.warn('IndexedDB not available, will use fallback');
                resolve();
            };
        });
    }

    async saveSram(romName, sramBytes) {
        if (!this.db || !sramBytes || sramBytes.length === 0) return;
        return new Promise((resolve, reject) => {
            const tx = this.db.transaction(STORE_SRAM, 'readwrite');
            const store = tx.objectStore(STORE_SRAM);
            const req = store.put(sramBytes, romName);
            req.onsuccess = () => resolve();
            req.onerror = (e) => reject(e);
        });
    }

    async loadSram(romName) {
        if (!this.db) return null;
        return new Promise((resolve) => {
            const tx = this.db.transaction(STORE_SRAM, 'readonly');
            const store = tx.objectStore(STORE_SRAM);
            const req = store.get(romName);
            req.onsuccess = (e) => resolve(e.target.result || null);
            req.onerror = () => resolve(null);
        });
    }

    async saveState(romName, slot, stateBytes) {
        if (!this.db || !stateBytes) return;
        const key = `${romName}_slot_${slot}`;
        return new Promise((resolve, reject) => {
            const tx = this.db.transaction(STORE_STATES, 'readwrite');
            const store = tx.objectStore(STORE_STATES);
            const req = store.put(stateBytes, key);
            req.onsuccess = () => resolve();
            req.onerror = (e) => reject(e);
        });
    }

    async loadState(romName, slot) {
        if (!this.db) return null;
        const key = `${romName}_slot_${slot}`;
        return new Promise((resolve) => {
            const tx = this.db.transaction(STORE_STATES, 'readonly');
            const store = tx.objectStore(STORE_STATES);
            const req = store.get(key);
            req.onsuccess = (e) => resolve(e.target.result || null);
            req.onerror = () => resolve(null);
        });
    }

    // Export battery save as .sav file download for PC compatibility
    downloadSramFile(romName, sramBytes) {
        if (!sramBytes || sramBytes.length === 0) {
            alert('No save RAM data found for this game.');
            return;
        }
        const blob = new Blob([sramBytes], { type: 'application/octet-stream' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        const base = romName.replace(/\.[^/.]+$/, '');
        a.href = url;
        a.download = `${base}.sav`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
}
