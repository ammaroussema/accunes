#ifndef ACCUNES_H
#define ACCUNES_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MobileEmulator MobileEmulator;

typedef enum {
    ACCUNES_BTN_A      = 1 << 0,
    ACCUNES_BTN_B      = 1 << 1,
    ACCUNES_BTN_SELECT = 1 << 2,
    ACCUNES_BTN_START  = 1 << 3,
    ACCUNES_BTN_UP     = 1 << 4,
    ACCUNES_BTN_DOWN   = 1 << 5,
    ACCUNES_BTN_LEFT   = 1 << 6,
    ACCUNES_BTN_RIGHT  = 1 << 7,
} AccuNESButton;

typedef enum {
    ACCUNES_PIXFMT_RGBA8888 = 0,
    ACCUNES_PIXFMT_BGRA8888 = 1,
    ACCUNES_PIXFMT_ARGB8888 = 2,
    ACCUNES_PIXFMT_RGB565   = 3,
} AccuNESPixelFormat;

typedef enum {
    ACCUNES_REGION_AUTO  = 0,
    ACCUNES_REGION_NTSC  = 1,
    ACCUNES_REGION_PAL   = 2,
    ACCUNES_REGION_DENDY = 3,
} AccuNESRegion;

MobileEmulator* accunes_create(double sample_rate);
void accunes_destroy(MobileEmulator* emu);

int32_t accunes_load_rom(MobileEmulator* emu, const uint8_t* rom_data, size_t len);
int32_t accunes_load_rom_file(MobileEmulator* emu, const char* path);
bool accunes_is_rom_loaded(const MobileEmulator* emu);
uint32_t accunes_get_rom_crc(const MobileEmulator* emu);
uint16_t accunes_get_rom_mapper(const MobileEmulator* emu);
const char* accunes_get_last_error(const MobileEmulator* emu);

void accunes_step_frame(MobileEmulator* emu);
void accunes_reset(MobileEmulator* emu);
void accunes_power_cycle(MobileEmulator* emu);

const uint32_t* accunes_get_framebuffer_ptr(const MobileEmulator* emu);
bool accunes_copy_framebuffer(const MobileEmulator* emu, uint8_t* dest, uint32_t pixel_format);

size_t accunes_drain_audio(MobileEmulator* emu, float* dest, size_t max_samples);
size_t accunes_drain_audio_stereo(MobileEmulator* emu, float* dest, size_t max_frames);
size_t accunes_audio_samples_ready(const MobileEmulator* emu);

void accunes_set_input_p1(MobileEmulator* emu, uint8_t buttons);
void accunes_set_input_p2(MobileEmulator* emu, uint8_t buttons);
uint8_t accunes_get_input_p1(const MobileEmulator* emu);
uint8_t accunes_get_input_p2(const MobileEmulator* emu);
void accunes_set_zapper(MobileEmulator* emu, int32_t x, int32_t y, bool trigger);

uint8_t accunes_peek_memory(MobileEmulator* emu, uint16_t addr);
void accunes_poke_memory(MobileEmulator* emu, uint16_t addr, uint8_t val);

uint8_t* accunes_save_state(MobileEmulator* emu, size_t* out_len);
bool accunes_load_state(MobileEmulator* emu, const uint8_t* data, size_t len);
uint8_t* accunes_get_sram(MobileEmulator* emu, size_t* out_len);
bool accunes_set_sram(MobileEmulator* emu, const uint8_t* data, size_t len);
void accunes_free_bytes(uint8_t* ptr, size_t len);

void accunes_set_region(MobileEmulator* emu, uint32_t region);
uint32_t accunes_get_region(const MobileEmulator* emu);
void accunes_capture_rewind(MobileEmulator* emu);
bool accunes_rewind_step(MobileEmulator* emu);
void accunes_change_disk(MobileEmulator* emu);
void accunes_insert_coin(MobileEmulator* emu, uint8_t coin);
const char* accunes_version(void);

#ifdef __cplusplus
}
#endif

#endif
