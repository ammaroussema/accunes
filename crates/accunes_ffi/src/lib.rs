use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use accunes_core::audio::AudioRingBuffer;
use accunes_core::cartridge::Cartridge;
use accunes_core::emulator::Emulator;
use accunes_core::region::Region;
use accunes_core::config::InitialRam;

pub struct MobileEmulator {
    emu: Emulator,
    audio_queue: Arc<Mutex<AudioRingBuffer>>,
    audio_scratch: Vec<f32>,
    last_error: Option<CString>,
    rom_loaded: bool,
    rom_crc32: u32,
    mapper: u16,
}

#[no_mangle]
pub extern "C" fn accunes_create(sample_rate: f64) -> *mut MobileEmulator {
    let mut emu = Emulator::new();
    let audio_queue = Arc::new(Mutex::new(AudioRingBuffer::new(16384)));
    let rate = if sample_rate <= 0.0 { 44100.0 } else { sample_rate };
    emu.set_audio_output(audio_queue.clone(), rate);
    let mobile = Box::new(MobileEmulator {
        emu,
        audio_queue,
        audio_scratch: Vec::with_capacity(4096),
        last_error: None,
        rom_loaded: false,
        rom_crc32: 0,
        mapper: 0,
    });
    Box::into_raw(mobile)
}

#[no_mangle]
pub unsafe extern "C" fn accunes_destroy(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        drop(Box::from_raw(emu));
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_load_rom(emu: *mut MobileEmulator, rom_data: *const u8, len: usize) -> i32 {
    if emu.is_null() || rom_data.is_null() || len == 0 {
        return -1;
    }
    let emu = &mut *emu;
    let slice = std::slice::from_raw_parts(rom_data, len);
    match Cartridge::from_bytes("mobile_rom.nes", slice) {
        Ok(cart) => {
            emu.rom_crc32 = cart.prg_rom_crc32;
            emu.mapper = cart.memory_mapper;
            emu.emu.load_cartridge(cart);
            emu.emu.power_cycle(InitialRam::Zero);
            emu.rom_loaded = true;
            emu.last_error = None;
            0
        }
        Err(e) => {
            emu.last_error = CString::new(e).ok();
            -2
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_load_rom_file(emu: *mut MobileEmulator, path: *const c_char) -> i32 {
    if emu.is_null() || path.is_null() {
        return -1;
    }
    let emu = &mut *emu;
    let cstr = CStr::from_ptr(path);
    let path_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    match std::fs::read(path_str) {
        Ok(data) => {
            let filename = std::path::Path::new(path_str)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("game.nes");
            match Cartridge::from_bytes(filename, &data) {
                Ok(cart) => {
                    emu.rom_crc32 = cart.prg_rom_crc32;
                    emu.mapper = cart.memory_mapper;
                    emu.emu.load_cartridge(cart);
                    emu.emu.power_cycle(InitialRam::Zero);
                    emu.rom_loaded = true;
                    emu.last_error = None;
                    0
                }
                Err(e) => {
                    emu.last_error = CString::new(e).ok();
                    -2
                }
            }
        }
        Err(e) => {
            emu.last_error = CString::new(e.to_string()).ok();
            -3
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_is_rom_loaded(emu: *const MobileEmulator) -> bool {
    if emu.is_null() { false } else { (*emu).rom_loaded }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_rom_crc(emu: *const MobileEmulator) -> u32 {
    if emu.is_null() { 0 } else { (*emu).rom_crc32 }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_rom_mapper(emu: *const MobileEmulator) -> u16 {
    if emu.is_null() { 0 } else { (*emu).mapper }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_last_error(emu: *const MobileEmulator) -> *const c_char {
    if emu.is_null() {
        std::ptr::null()
    } else {
        match &(*emu).last_error {
            Some(err) => err.as_ptr(),
            None => std::ptr::null(),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_step_frame(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        (*emu).emu.core_frame_advance();
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_reset(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        (*emu).emu.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_power_cycle(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        (*emu).emu.power_cycle(InitialRam::Zero);
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_framebuffer_ptr(emu: *const MobileEmulator) -> *const u32 {
    if emu.is_null() {
        std::ptr::null()
    } else {
        (*emu).emu.screen.as_ptr()
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_copy_framebuffer(
    emu: *const MobileEmulator,
    dest: *mut u8,
    pixel_format: u32,
) -> bool {
    if emu.is_null() || dest.is_null() {
        return false;
    }
    let screen = &(*emu).emu.screen;
    match pixel_format {
        0 => {
            let out = std::slice::from_raw_parts_mut(dest, 256 * 240 * 4);
            for (i, &p) in screen.iter().enumerate() {
                let r = ((p >> 16) & 0xFF) as u8;
                let g = ((p >> 8) & 0xFF) as u8;
                let b = (p & 0xFF) as u8;
                let idx = i * 4;
                out[idx] = r;
                out[idx + 1] = g;
                out[idx + 2] = b;
                out[idx + 3] = 0xFF;
            }
            true
        }
        1 => {
            let out = std::slice::from_raw_parts_mut(dest, 256 * 240 * 4);
            for (i, &p) in screen.iter().enumerate() {
                let r = ((p >> 16) & 0xFF) as u8;
                let g = ((p >> 8) & 0xFF) as u8;
                let b = (p & 0xFF) as u8;
                let idx = i * 4;
                out[idx] = b;
                out[idx + 1] = g;
                out[idx + 2] = r;
                out[idx + 3] = 0xFF;
            }
            true
        }
        2 => {
            let out = std::slice::from_raw_parts_mut(dest as *mut u32, 256 * 240);
            out.copy_from_slice(screen);
            true
        }
        3 => {
            let out = std::slice::from_raw_parts_mut(dest as *mut u16, 256 * 240);
            for (i, &p) in screen.iter().enumerate() {
                let r = ((p >> 19) & 0x1F) as u16;
                let g = ((p >> 10) & 0x3F) as u16;
                let b = ((p >> 3) & 0x1F) as u16;
                out[i] = (r << 11) | (g << 5) | b;
            }
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_drain_audio(
    emu: *mut MobileEmulator,
    dest: *mut f32,
    max_samples: usize,
) -> usize {
    if emu.is_null() || dest.is_null() || max_samples == 0 {
        return 0;
    }
    let emu = &mut *emu;
    emu.audio_scratch.clear();
    if let Ok(mut buf) = emu.audio_queue.lock() {
        let count = buf.len().min(max_samples);
        if count > 0 {
            buf.drain_into(&mut emu.audio_scratch);
            let copy_count = emu.audio_scratch.len().min(max_samples);
            std::ptr::copy_nonoverlapping(emu.audio_scratch.as_ptr(), dest, copy_count);
            copy_count
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_drain_audio_stereo(
    emu: *mut MobileEmulator,
    dest: *mut f32,
    max_frames: usize,
) -> usize {
    if emu.is_null() || dest.is_null() || max_frames == 0 {
        return 0;
    }
    let emu = &mut *emu;
    emu.audio_scratch.clear();
    if let Ok(mut buf) = emu.audio_queue.lock() {
        let count = buf.len().min(max_frames);
        if count > 0 {
            buf.drain_into(&mut emu.audio_scratch);
            let frames = emu.audio_scratch.len().min(max_frames);
            let out_slice = std::slice::from_raw_parts_mut(dest, frames * 2);
            for (i, &s) in emu.audio_scratch.iter().take(frames).enumerate() {
                out_slice[i * 2] = s;
                out_slice[i * 2 + 1] = s;
            }
            frames
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_audio_samples_ready(emu: *const MobileEmulator) -> usize {
    if emu.is_null() {
        0
    } else if let Ok(buf) = (*emu).audio_queue.lock() {
        buf.len()
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_set_input_p1(emu: *mut MobileEmulator, buttons: u8) {
    if !emu.is_null() {
        (*emu).emu.controller_port1.store(buttons, Ordering::Relaxed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_set_input_p2(emu: *mut MobileEmulator, buttons: u8) {
    if !emu.is_null() {
        (*emu).emu.controller_port2.store(buttons, Ordering::Relaxed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_input_p1(emu: *const MobileEmulator) -> u8 {
    if emu.is_null() { 0 } else { (*emu).emu.controller_port1.load(Ordering::Relaxed) }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_input_p2(emu: *const MobileEmulator) -> u8 {
    if emu.is_null() { 0 } else { (*emu).emu.controller_port2.load(Ordering::Relaxed) }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_set_zapper(emu: *mut MobileEmulator, x: i32, y: i32, trigger: bool) {
    if !emu.is_null() {
        if let Ok(mut zx) = (*emu).emu.zapper_x.lock() {
            *zx = (x.clamp(0, 255) as f32) / 255.0;
        }
        if let Ok(mut zy) = (*emu).emu.zapper_y.lock() {
            *zy = (y.clamp(0, 239) as f32) / 239.0;
        }
        (*emu).emu.zapper_trigger.store(trigger, Ordering::Relaxed);
        if trigger {
            (*emu).emu.zapper_bogo.store(3, Ordering::Relaxed);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_peek_memory(emu: *mut MobileEmulator, addr: u16) -> u8 {
    if emu.is_null() { 0 } else { (*emu).emu.debug_peek_mem(addr) }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_poke_memory(emu: *mut MobileEmulator, addr: u16, val: u8) {
    if !emu.is_null() {
        match addr {
            0x0000..=0x1FFF => {
                let mask = (*emu).emu.cpu_ram_mask as usize;
                (*emu).emu.ram[(addr as usize) & mask] = val;
            }
            _ => {}
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_save_state(emu: *mut MobileEmulator, out_len: *mut usize) -> *mut u8 {
    if emu.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let data = (*emu).emu.save_state_to_bytes();
    *out_len = data.len();
    let mut boxed = data.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn accunes_load_state(emu: *mut MobileEmulator, data: *const u8, len: usize) -> bool {
    if emu.is_null() || data.is_null() || len == 0 {
        return false;
    }
    let slice = std::slice::from_raw_parts(data, len);
    (*emu).emu.load_state_from_bytes(slice).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_sram(emu: *mut MobileEmulator, out_len: *mut usize) -> *mut u8 {
    if emu.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    if let Some(sram) = (*emu).emu.get_battery_save_data() {
        *out_len = sram.len();
        let mut boxed = sram.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        ptr
    } else {
        *out_len = 0;
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_set_sram(emu: *mut MobileEmulator, data: *const u8, len: usize) -> bool {
    if emu.is_null() || data.is_null() || len == 0 {
        return false;
    }
    let slice = std::slice::from_raw_parts(data, len);
    (*emu).emu.load_battery_save_data(slice);
    true
}

#[no_mangle]
pub unsafe extern "C" fn accunes_free_bytes(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_set_region(emu: *mut MobileEmulator, region: u32) {
    if !emu.is_null() {
        let r = match region {
            1 => Region::Ntsc,
            2 => Region::Pal,
            3 => Region::Dendy,
            _ => Region::Ntsc,
        };
        (*emu).emu.set_region_preference(r);
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_get_region(emu: *const MobileEmulator) -> u32 {
    if emu.is_null() {
        0
    } else if (*emu).emu.is_pal() {
        2
    } else if (*emu).emu.is_dendy() {
        3
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_capture_rewind(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        (*emu).emu.capture_rewind_state();
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_rewind_step(emu: *mut MobileEmulator) -> bool {
    if emu.is_null() { false } else { (*emu).emu.rewind_one_frame() }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_change_disk(emu: *mut MobileEmulator) {
    if !emu.is_null() {
        (*emu).emu.change_disk();
    }
}

#[no_mangle]
pub unsafe extern "C" fn accunes_insert_coin(emu: *mut MobileEmulator, coin: u8) {
    if !emu.is_null() {
        (*emu).emu.insert_coin(coin);
    }
}

#[no_mangle]
pub extern "C" fn accunes_version() -> *const c_char {
    static VERSION: &[u8] = b"1.8.0\0";
    VERSION.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeCreate(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    sample_rate: f64,
) -> i64 {
    accunes_create(sample_rate) as i64
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeDestroy(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) {
    accunes_destroy(handle as *mut MobileEmulator);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeLoadRom(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    data: jni::objects::JByteArray,
) -> i32 {
    let bytes = match env.convert_byte_array(data) {
        Ok(b) => b,
        Err(_) => return -1,
    };
    accunes_load_rom(handle as *mut MobileEmulator, bytes.as_ptr(), bytes.len())
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeLoadRomFile(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    path: jni::objects::JString,
) -> i32 {
    let path_str: String = match env.get_string(&path) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };
    let cstr = match CString::new(path_str) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    accunes_load_rom_file(handle as *mut MobileEmulator, cstr.as_ptr())
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeStepFrame(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) {
    accunes_step_frame(handle as *mut MobileEmulator);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeDrainAudioStereo(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    buffer: jni::objects::JFloatArray,
) -> i32 {
    let len = match env.get_array_length(&buffer) {
        Ok(l) => l as usize,
        Err(_) => return 0,
    };
    let emu = handle as *mut MobileEmulator;
    if emu.is_null() || len < 2 {
        return 0;
    }
    let frames = len / 2;
    let mut scratch = vec![0.0f32; len];
    let drained = accunes_drain_audio_stereo(emu, scratch.as_mut_ptr(), frames);
    if drained > 0 {
        let _ = env.set_float_array_region(&buffer, 0, &scratch[..drained * 2]);
    }
    drained as i32
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeSetInputP1(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    buttons: i32,
) {
    accunes_set_input_p1(handle as *mut MobileEmulator, buttons as u8);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeSetInputP2(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    buttons: i32,
) {
    accunes_set_input_p2(handle as *mut MobileEmulator, buttons as u8);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeSetZapper(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    x: i32,
    y: i32,
    trigger: bool,
) {
    accunes_set_zapper(handle as *mut MobileEmulator, x, y, trigger);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeSaveState(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) -> jni::sys::jbyteArray {
    let emu = handle as *mut MobileEmulator;
    if emu.is_null() {
        return std::ptr::null_mut();
    }
    let mut len: usize = 0;
    let ptr = accunes_save_state(emu, &mut len);
    if ptr.is_null() || len == 0 {
        return std::ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    let result = match env.byte_array_from_slice(slice) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    };
    accunes_free_bytes(ptr, len);
    result
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeLoadState(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    data: jni::objects::JByteArray,
) -> bool {
    let bytes = match env.convert_byte_array(data) {
        Ok(b) => b,
        Err(_) => return false,
    };
    accunes_load_state(handle as *mut MobileEmulator, bytes.as_ptr(), bytes.len())
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeGetSram(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) -> jni::sys::jbyteArray {
    let emu = handle as *mut MobileEmulator;
    if emu.is_null() {
        return std::ptr::null_mut();
    }
    let mut len: usize = 0;
    let ptr = accunes_get_sram(emu, &mut len);
    if ptr.is_null() || len == 0 {
        return std::ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    let result = match env.byte_array_from_slice(slice) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    };
    accunes_free_bytes(ptr, len);
    result
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeSetSram(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    data: jni::objects::JByteArray,
) -> bool {
    let bytes = match env.convert_byte_array(data) {
        Ok(b) => b,
        Err(_) => return false,
    };
    accunes_set_sram(handle as *mut MobileEmulator, bytes.as_ptr(), bytes.len())
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeReset(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) {
    accunes_reset(handle as *mut MobileEmulator);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativePowerCycle(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) {
    accunes_power_cycle(handle as *mut MobileEmulator);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeGetFramebufferPtr(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) -> i64 {
    accunes_get_framebuffer_ptr(handle as *const MobileEmulator) as i64
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeCopyFramebuffer(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    dest: jni::objects::JByteBuffer,
    format: i32,
) -> bool {
    let emu = handle as *const MobileEmulator;
    if emu.is_null() {
        return false;
    }
    let ptr = match env.get_direct_buffer_address(&dest) {
        Ok(p) => p,
        Err(_) => return false,
    };
    accunes_copy_framebuffer(emu, ptr, format as u32)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeGetRomCrc(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) -> i32 {
    accunes_get_rom_crc(handle as *const MobileEmulator) as i32
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativeGetRomMapper(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
) -> i32 {
    accunes_get_rom_mapper(handle as *const MobileEmulator) as i32
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativePeekMemory(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    addr: i32,
) -> i32 {
    accunes_peek_memory(handle as *mut MobileEmulator, addr as u16) as i32
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_accunes_Emulator_nativePokeMemory(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
    handle: i64,
    addr: i32,
    val: i32,
) {
    accunes_poke_memory(handle as *mut MobileEmulator, addr as u16, val as u8);
}
