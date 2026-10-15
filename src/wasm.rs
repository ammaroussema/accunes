// web port of core emulator functions!!
use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use crate::audio::AudioRingBuffer;
use crate::cartridge::Cartridge;
use crate::emulator::Emulator;

#[wasm_bindgen]
pub struct WasmEmulator {
    emu: Emulator,
    audio_queue: Arc<Mutex<AudioRingBuffer>>,
}

#[wasm_bindgen]
impl WasmEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f64) -> Self {
        let mut emu = Emulator::new();
        let audio_queue = Arc::new(Mutex::new(AudioRingBuffer::new(16384)));
        emu.set_audio_output(audio_queue.clone(), sample_rate);
        Self { emu, audio_queue }
    }

    pub fn load_rom(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let cart = Cartridge::from_bytes(name, data)?;
        self.emu.load_cartridge(cart);
        self.emu.power_cycle(crate::config::InitialRam::Zero);
        Ok(())
    }

    pub fn step_frame(&mut self) {
        self.emu.core_frame_advance();
    }

    pub fn get_framebuffer_ptr(&self) -> *const u32 {
        self.emu.screen.as_ptr()
    }

    pub fn set_controller_p1(&self, state: u8) {
        self.emu.controller_port1.store(state, Ordering::Relaxed);
    }

    pub fn set_controller_p2(&self, state: u8) {
        self.emu.controller_port2.store(state, Ordering::Relaxed);
    }

    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        if let Ok(mut buf) = self.audio_queue.lock() {
            buf.drain_all()
        } else {
            Vec::new()
        }
    }

    pub fn save_state(&mut self) -> Vec<u8> {
        self.emu.save_state_to_bytes()
    }

    pub fn load_state(&mut self, state: &[u8]) -> bool {
        self.emu.load_state_from_bytes(state).is_ok()
    }

    pub fn get_sram(&self) -> Option<Vec<u8>> {
        self.emu.get_battery_save_data()
    }

    pub fn load_sram(&mut self, sram: &[u8]) {
        self.emu.load_battery_save_data(sram);
    }

    pub fn reset(&mut self) {
        self.emu.reset();
    }

    pub fn power_cycle(&mut self) {
        self.emu.power_cycle(crate::config::InitialRam::Zero);
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.emu.set_theme(theme);
    }
}
