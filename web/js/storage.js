// save, settings, and ROM storage support for the web version!!!

const DB_NAME = 'AccuNesDB';
const DB_VERSION = 2;
const STORE_SRAM = 'sram';
const STORE_STATES = 'savestates';
const STORE_ROMS = 'roms';

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
                if (!db.objectStoreNames.contains(STORE_ROMS)) {
                    const romStore = db.createObjectStore(STORE_ROMS, { keyPath: 'name' });
                    romStore.createIndex('timestamp', 'timestamp', { unique: false });
                }
            };
            req.onsuccess = (e) => {
                this.db = e.target.result;
                resolve();
            };
            req.onerror = () => {
                console.warn('IndexedDB not available, will use memory fallback');
                resolve();
            };
        });
    }

    async saveSram(romName, sramBytes) {
        if (!this.db || !sramBytes || sramBytes.length === 0) return;
        return new Promise((resolve, reject) => {
            try {
                const tx = this.db.transaction(STORE_SRAM, 'readwrite');
                const store = tx.objectStore(STORE_SRAM);
                const req = store.put(sramBytes, romName);
                req.onsuccess = () => resolve();
                req.onerror = (e) => reject(e);
            } catch (err) {
                reject(err);
            }
        });
    }

    async loadSram(romName) {
        if (!this.db) return null;
        return new Promise((resolve) => {
            try {
                const tx = this.db.transaction(STORE_SRAM, 'readonly');
                const store = tx.objectStore(STORE_SRAM);
                const req = store.get(romName);
                req.onsuccess = (e) => resolve(e.target.result || null);
                req.onerror = () => resolve(null);
            } catch {
                resolve(null);
            }
        });
    }

    async saveState(romName, slot, stateBytes) {
        if (!this.db || !stateBytes) return;
        const key = `${romName}_slot_${slot}`;
        return new Promise((resolve, reject) => {
            try {
                const tx = this.db.transaction(STORE_STATES, 'readwrite');
                const store = tx.objectStore(STORE_STATES);
                const req = store.put(stateBytes, key);
                req.onsuccess = () => resolve();
                req.onerror = (e) => reject(e);
            } catch (err) {
                reject(err);
            }
        });
    }

    async loadState(romName, slot) {
        if (!this.db) return null;
        const key = `${romName}_slot_${slot}`;
        return new Promise((resolve) => {
            try {
                const tx = this.db.transaction(STORE_STATES, 'readonly');
                const store = tx.objectStore(STORE_STATES);
                const req = store.get(key);
                req.onsuccess = (e) => resolve(e.target.result || null);
                req.onerror = () => resolve(null);
            } catch {
                resolve(null);
            }
        });
    }

    async saveRom(name, dataBytes) {
        if (!this.db || !dataBytes) return;
        return new Promise((resolve, reject) => {
            try {
                const tx = this.db.transaction(STORE_ROMS, 'readwrite');
                const store = tx.objectStore(STORE_ROMS);
                const record = {
                    name,
                    data: dataBytes,
                    timestamp: Date.now(),
                    size: dataBytes.length
                };
                const req = store.put(record);
                req.onsuccess = () => resolve();
                req.onerror = (e) => reject(e);
            } catch (err) {
                reject(err);
            }
        });
    }

    async loadRom(name) {
        if (!this.db) return null;
        return new Promise((resolve) => {
            try {
                const tx = this.db.transaction(STORE_ROMS, 'readonly');
                const store = tx.objectStore(STORE_ROMS);
                const req = store.get(name);
                req.onsuccess = (e) => {
                    const record = e.target.result;
                    if (record) {
                        this.touchRom(name);
                        resolve(record.data);
                    } else {
                        resolve(null);
                    }
                };
                req.onerror = () => resolve(null);
            } catch {
                resolve(null);
            }
        });
    }

    async touchRom(name) {
        if (!this.db) return;
        try {
            const tx = this.db.transaction(STORE_ROMS, 'readwrite');
            const store = tx.objectStore(STORE_ROMS);
            const getReq = store.get(name);
            getReq.onsuccess = () => {
                if (getReq.result) {
                    getReq.result.timestamp = Date.now();
                    store.put(getReq.result);
                }
            };
        } catch {
        }
    }

    async listRoms() {
        if (!this.db) return [];
        return new Promise((resolve) => {
            try {
                const tx = this.db.transaction(STORE_ROMS, 'readonly');
                const store = tx.objectStore(STORE_ROMS);
                const req = store.getAll();
                req.onsuccess = (e) => {
                    const records = e.target.result || [];
                    const list = records.map((r) => ({
                        name: r.name,
                        size: r.size,
                        timestamp: r.timestamp
                    })).sort((a, b) => b.timestamp - a.timestamp);
                    resolve(list);
                };
                req.onerror = () => resolve([]);
            } catch {
                resolve([]);
            }
        });
    }

    async deleteRom(name) {
        if (!this.db) return;
        return new Promise((resolve) => {
            try {
                const tx = this.db.transaction(STORE_ROMS, 'readwrite');
                const store = tx.objectStore(STORE_ROMS);
                const req = store.delete(name);
                req.onsuccess = () => resolve();
                req.onerror = () => resolve();
            } catch {
                resolve();
            }
        });
    }

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
