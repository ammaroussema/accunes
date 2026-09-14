use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::nrom::{MapperNROM, NromConfig};
use crate::mappers::pic16c54::Pic16C54;

pub struct Mapper355 {
    nrom: MapperNROM,
    cpu_address: u16,
    pic: Option<Pic16C54>,
    irq: bool,
}

impl Mapper355 {
    pub fn new(misc_rom: Vec<u8>, nrom_config: NromConfig) -> Self {
        let pic = if misc_rom.len() == 1024 {
            Some(Pic16C54::new(misc_rom))
        } else {
            None
        };
        Self {
            nrom: MapperNROM::new(nrom_config),
            cpu_address: 0,
            pic,
            irq: false,
        }
    }

    fn sync_irq_from_pic(&mut self) {
        if let Some(pic) = &self.pic {
            self.irq = pic.irq_level();
        }
    }
}

impl Mapper for Mapper355 {
    fn reset(&mut self) {
        self.cpu_address = 0;
        self.irq = false;
        if let Some(pic) = &mut self.pic {
            pic.reset(false);
        }
        self.sync_irq_from_pic();
    }

    fn reset_power_cycle(&mut self) {
        self.cpu_address = 0;
        self.irq = false;
        if let Some(pic) = &mut self.pic {
            pic.reset(true);
        }
        self.sync_irq_from_pic();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.nrom.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.nrom.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.nrom.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        self.nrom.fetch_ppu(
            prg_rom,
            chr_rom,
            prg_ram,
            chr_ram,
            prg_vram,
            using_chr_ram,
            nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus,
            ppu_octal_latch,
            vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.nrom.store_ppu(cart, address, data, vram);
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if let Some(pic) = &mut self.pic {
            pic.run(self.cpu_address, &mut self.irq);
        }
        self.irq
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn handle_cpu_read(&mut self, address: u16) {
        self.cpu_address = address;
    }

    fn handle_cpu_write(&mut self, address: u16, _data: u8) {
        self.cpu_address = address;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(2 + 171);
        state.extend_from_slice(&self.cpu_address.to_le_bytes());
        if let Some(pic) = &self.pic {
            state.extend_from_slice(&pic.save_state());
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 > state.len() {
            return p;
        }
        self.cpu_address = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        if let Some(pic) = &mut self.pic {
            if p < state.len() {
                p += pic.load_state(&state[p..]);
            }
            self.sync_irq_from_pic();
        }
        p
    }
}
