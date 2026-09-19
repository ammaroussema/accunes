// ram searching tool!!!
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::cheats::{CheatEntry, CheatType};
use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, MenuState, UiColors};

const RED_TEXT: u32 = 0xFF4040C0;
const UNDO_CAP: usize = 10000;
const MAX_INPUT_LEN: usize = 15;
const LIST_ROWS: usize = 10;

const FOCUS_LIST: usize = 0;
const FOCUS_VAL: usize = 1;
const FOCUS_ADDR: usize = 2;
const FOCUS_CHANGES: usize = 3;
const FOCUS_DIFFBY: usize = 4;
const FOCUS_MODBY: usize = 5;

const DEACT_NONE: i32 = 0;
const DEACT_SHRINK: i32 = 1;
const DEACT_REMOVED: i32 = 2;
const DEACT_SPLIT: i32 = 3;

#[derive(Clone)]
struct RamRegion {
    hw_addr: u32,
    size: u32,
    virt_index: u32,
    item_index: u32,
}

struct RamSearchState {
    regions: Vec<RamRegion>,
    backup: Vec<RamRegion>,
    undo_type: i32,
    prev: Vec<u8>,
    cur: Vec<u8>,
    changes: Vec<u16>,
    item_to_region: Vec<u32>,
    item_indices_valid: bool,
    max_item_index: u32,
    prev_needs_update: bool,
    result_count: u32,
    show_rom: bool,
    auto_search: bool,
    rs_c: u8,
    rs_o: u8,
    rs_t: u8,
    rs_type_size: u8,
    no_misalign: bool,
    rs_val: i64,
    rs_param: i64,
    rs_val_valid: bool,
    in_val: String,
    in_addr: String,
    in_changes: String,
    in_diffby: String,
    in_modby: String,
    note: String,
    needs_init: bool,
}

impl RamSearchState {
    const fn new() -> Self {
        Self {
            regions: Vec::new(),
            backup: Vec::new(),
            undo_type: 0,
            prev: Vec::new(),
            cur: Vec::new(),
            changes: Vec::new(),
            item_to_region: Vec::new(),
            item_indices_valid: false,
            max_item_index: 0,
            prev_needs_update: true,
            result_count: 0,
            show_rom: false,
            auto_search: false,
            rs_c: b's',
            rs_o: b'=',
            rs_t: b's',
            rs_type_size: b'b',
            no_misalign: true,
            rs_val: 0,
            rs_param: 0,
            rs_val_valid: false,
            in_val: String::new(),
            in_addr: String::new(),
            in_changes: String::new(),
            in_diffby: String::new(),
            in_modby: String::new(),
            note: String::new(),
            needs_init: true,
        }
    }

    fn step(&self) -> u32 {
        match self.rs_type_size {
            b'b' => 1,
            b'w' => {
                if self.no_misalign {
                    2
                } else {
                    1
                }
            }
            _ => {
                if self.no_misalign {
                    4
                } else {
                    1
                }
            }
        }
    }

    fn vsize(&self) -> u32 {
        match self.rs_type_size {
            b'b' => 1,
            b'w' => 2,
            _ => 4,
        }
    }

    fn signed_compare(&self) -> bool {
        if self.rs_c == b'a' || self.rs_c == b'n' {
            false
        } else {
            self.rs_t == b's'
        }
    }

    fn valid_addr(&self, a: u32) -> bool {
        if self.show_rom {
            (0x8000..0x10000).contains(&a)
        } else {
            a < 0x800 || (0x6000..0x8000).contains(&a)
        }
    }

    fn rebuild_regions(&mut self) {
        self.regions.clear();
        let mut start: Option<u32> = None;
        for a in 0..=0x10000u32 {
            let valid = a < 0x10000 && self.valid_addr(a);
            if valid {
                if start.is_none() {
                    start = Some(a);
                }
            } else if let Some(s) = start.take() {
                self.regions
                    .push(RamRegion { hw_addr: s, size: a - s, virt_index: 0, item_index: 0 });
            }
        }
        let mut next = 0u32;
        for r in self.regions.iter_mut() {
            r.virt_index = next;
            next += r.size + 4;
        }
    }

    fn ensure_pools(&mut self) {
        let needed = self
            .regions
            .last()
            .map(|r| (r.virt_index + r.size + 4) as usize)
            .unwrap_or(0);
        if self.cur.len() < needed {
            self.cur.resize(needed, 0);
            self.prev.resize(needed, 0);
            self.changes.resize(needed, 0);
        }
    }

    fn compact(&mut self) {
        let step = self.step();
        let mut item = 0u32;
        for r in self.regions.iter_mut() {
            r.item_index = item;
            let skip = (step - r.hw_addr % step) % step;
            let mut rel = skip;
            while rel < r.size {
                rel += step;
                item += 1;
            }
        }
        self.max_item_index = item;
        self.result_count = item;
        self.item_indices_valid = true;
        if self.item_to_region.len() < item as usize {
            self.item_to_region.resize(item as usize, u32::MAX);
        }
        let regions_len = self.regions.len();
        for (ri, r) in self.regions.iter_mut().enumerate() {
            let skip = (step - r.hw_addr % step) % step;
            let mut rel = skip;
            let base = r.item_index as usize;
            while rel < r.size {
                self.item_to_region[base + ((rel - skip) / step) as usize] = ri.min(regions_len - 1) as u32;
                rel += step;
            }
        }
    }

    fn item_info(&self, item: u32) -> Option<(u32, usize)> {
        if !self.item_indices_valid || item >= self.max_item_index {
            return None;
        }
        let ri = *self.item_to_region.get(item as usize)? as usize;
        let r = self.regions.get(ri)?;
        let step = self.step();
        let skip = (step - r.hw_addr % step) % step;
        let rel = skip + (item - r.item_index) * step;
        if rel >= r.size {
            return None;
        }
        Some((r.hw_addr + rel, (r.virt_index + rel) as usize))
    }

    fn read_pool(data: &[u8], vi: usize, n: usize) -> u64 {
        let mut v = 0u64;
        for k in 0..n {
            v |= (data.get(vi + k).copied().unwrap_or(0) as u64) << (8 * k);
        }
        v
    }

    fn value_at(&self, data: &[u8], vi: usize) -> i64 {
        let n = self.vsize() as usize;
        let raw = Self::read_pool(data, vi, n);
        if self.signed_compare() {
            sign_extend(raw, n)
        } else {
            raw as i64
        }
    }

    fn cmp_op(o: u8, x: i64, y: i64, p: i64, signed: bool) -> bool {
        if signed {
            match o {
                b'<' => x < y,
                b'>' => x > y,
                b'l' => x <= y,
                b'm' => x >= y,
                b'=' => x == y,
                b'!' => x != y,
                b'd' => x.wrapping_sub(y) == p || y.wrapping_sub(x) == p,
                b'%' => p != 0 && x.wrapping_rem(p) == y,
                _ => false,
            }
        } else {
            let (x, y, p) = (x as u64, y as u64, p as u64);
            match o {
                b'<' => x < y,
                b'>' => x > y,
                b'l' => x <= y,
                b'm' => x >= y,
                b'=' => x == y,
                b'!' => x != y,
                b'd' => x.wrapping_sub(y) == p || y.wrapping_sub(x) == p,
                b'%' => p != 0 && x.wrapping_rem(p) == y,
                _ => false,
            }
        }
    }

    fn item_satisfied(&self, item: u32) -> bool {
        if !self.rs_val_valid {
            return true;
        }
        let Some((hw, vi)) = self.item_info(item) else {
            return true;
        };
        let signed = self.signed_compare();
        let (x, y) = match self.rs_c {
            b'r' => (self.value_at(&self.cur, vi), self.value_at(&self.prev, vi)),
            b's' => (self.value_at(&self.cur, vi), self.rs_val),
            b'a' => (hw as i64, self.rs_val),
            _ => (self.changes[vi] as i64, self.rs_val),
        };
        Self::cmp_op(self.rs_o, x, y, self.rs_param, signed)
    }

    fn deactivate(regions: &mut Vec<RamRegion>, idx: usize, hw: u32, size: u32) -> i32 {
        let r_end = regions[idx].hw_addr + regions[idx].size;
        let r_hw = regions[idx].hw_addr;
        let i_end = hw.wrapping_add(size);
        if i_end <= r_hw || hw >= r_end {
            return DEACT_NONE;
        }
        if hw > r_hw && i_end >= r_end {
            regions[idx].size = hw - r_hw;
            return DEACT_SHRINK;
        }
        if hw <= r_hw && i_end < r_end {
            let erase = i_end - r_hw;
            regions[idx].hw_addr += erase;
            regions[idx].size -= erase;
            regions[idx].virt_index += erase;
            return DEACT_SHRINK;
        }
        if hw <= r_hw && i_end >= r_end {
            regions.remove(idx);
            return DEACT_REMOVED;
        }
        let erase = i_end - r_hw;
        let r2 = RamRegion {
            hw_addr: r_hw + erase,
            size: regions[idx].size - erase,
            virt_index: regions[idx].virt_index + erase,
            item_index: 0,
        };
        regions[idx].size = hw - r_hw;
        regions.insert(idx + 1, r2);
        DEACT_SPLIT
    }

    fn prune(&mut self) {
        let step = self.step();
        let n = self.vsize() as usize;
        let signed = self.signed_compare();
        let c = self.rs_c;
        let o = self.rs_o;
        let val = self.rs_val;
        let param = self.rs_param;
        let mut i = 0usize;
        while i < self.regions.len() {
            let (r_hw, r_size, r_virt, r_item) = {
                let r = &self.regions[i];
                (r.hw_addr, r.size, r.virt_index, r.item_index)
            };
            let skip = (step - r_hw % step) % step;
            let mut jumped = false;
            let mut rel = skip;
            while rel < r_size {
                let hw = r_hw + rel;
                let vi = (r_virt + rel) as usize;
                let (x, y) = match c {
                    b'r' => (
                        sign_extend(Self::read_pool(&self.cur, vi, n), n),
                        sign_extend(Self::read_pool(&self.prev, vi, n), n),
                    ),
                    b's' => (
                        sign_extend(Self::read_pool(&self.cur, vi, n), n),
                        val,
                    ),
                    b'a' => (hw as i64, val),
                    _ => (self.changes[vi] as i64, val),
                };
                if !Self::cmp_op(o, x, y, param, signed) {
                    let act = Self::deactivate(&mut self.regions, i, hw, step);
                    if act == DEACT_REMOVED {
                        jumped = true;
                        break;
                    }
                    if act == DEACT_SPLIT {
                        i += 1;
                        jumped = true;
                        break;
                    }
                }
                rel += step;
            }
            let _ = r_item;
            if !jumped {
                i += 1;
            }
        }
        self.prev_needs_update = true;
        self.compact();
    }

    fn eliminate_ranges(&mut self, ranges: &[(u32, u32)]) {
        for &(addr, size) in ranges {
            let mut i = 0usize;
            let mut affected = false;
            while i < self.regions.len() {
                let act = Self::deactivate(&mut self.regions, i, addr, size);
                if act != DEACT_NONE {
                    affected = true;
                } else if affected {
                    break;
                }
                if act != DEACT_REMOVED && act != DEACT_SPLIT {
                    i += 1;
                }
            }
        }
        self.compact();
    }

    fn update_frame(&mut self, read: &mut dyn FnMut(u16) -> u8) {
        if self.prev_needs_update {
            let len = self.prev.len().min(self.cur.len());
            self.prev[..len].copy_from_slice(&self.cur[..len]);
        }
        let step = self.step();
        let n = self.vsize() as usize;
        let mut ri = 0usize;
        while ri < self.regions.len() {
            let (r_hw, r_size, r_virt) = {
                let r = &self.regions[ri];
                (r.hw_addr, r.size, r.virt_index)
            };
            let skip = (step - r_hw % step) % step;
            let mut rel = skip;
            while rel < r_size {
                let hw = r_hw + rel;
                let vi = (r_virt + rel) as usize;
                let mut bytes = [0u8; 4];
                let mut changed = false;
                for b in 0..n {
                    let hb = read((hw as u16).wrapping_add(b as u16));
                    bytes[b] = hb;
                    if self.cur[vi + b] != hb {
                        changed = true;
                    }
                }
                if changed {
                    self.cur[vi..vi + n].copy_from_slice(&bytes[..n]);
                    if self.changes[vi] < 0xFFFF {
                        self.changes[vi] += 1;
                    }
                }
                rel += step;
            }
            ri += 1;
        }
        self.prev_needs_update = false;
    }

    fn init_state(&mut self, read: &mut dyn FnMut(u16) -> u8) {
        self.rebuild_regions();
        self.ensure_pools();
        let regions_snapshot: Vec<(u32, u32, u32)> = self
            .regions
            .iter()
            .map(|r| (r.hw_addr, r.size, r.virt_index))
            .collect();
        for (hw, size, virt) in regions_snapshot {
            for k in 0..size {
                let b = read((hw + k) as u16);
                let idx = (virt + k) as usize;
                if idx < self.cur.len() {
                    self.cur[idx] = b;
                }
            }
        }
        let len = self.prev.len().min(self.cur.len());
        self.prev[..len].copy_from_slice(&self.cur[..len]);
        for c in self.changes.iter_mut() {
            *c = 0;
        }
        self.prev_needs_update = false;
        self.needs_init = false;
        self.compact();
    }

    fn soft_reset(&mut self, read: &mut dyn FnMut(u16) -> u8) {
        self.rebuild_regions();
        self.ensure_pools();
        let regions_snapshot: Vec<(u32, u32, u32)> = self
            .regions
            .iter()
            .map(|r| (r.hw_addr, r.size, r.virt_index))
            .collect();
        for (hw, size, virt) in regions_snapshot {
            for k in 0..size {
                let b = read((hw + k) as u16);
                let idx = (virt + k) as usize;
                if idx < self.cur.len() {
                    self.cur[idx] = b;
                }
            }
        }
        let len = self.prev.len().min(self.cur.len());
        self.prev[..len].copy_from_slice(&self.cur[..len]);
        self.prev_needs_update = false;
        for c in self.changes.iter_mut() {
            *c = 0;
        }
        self.compact();
    }

    fn save_undo(&mut self) {
        if self.regions.len() < UNDO_CAP {
            self.backup = self.regions.clone();
            self.undo_type = 1;
        } else {
            self.undo_type = 0;
        }
    }

    fn do_undo(&mut self) {
        if self.undo_type == 1 || self.undo_type == 2 {
            std::mem::swap(&mut self.regions, &mut self.backup);
            self.undo_type = 3 - self.undo_type;
            self.compact();
        }
    }

    fn set_rs_val(&mut self) -> bool {
        let ok = match self.rs_c {
            b'r' => {
                self.rs_val = 0;
                true
            }
            b's' => {
                let force_hex = self.rs_t == b'h';
                let signed = self.rs_t == b's';
                let Some(v) = parse_int(&self.in_val, force_hex, signed) else {
                    return false;
                };
                let in_range = if self.rs_type_size == b'b' {
                    if signed { (-128..=127).contains(&v) } else { (0..=255).contains(&v) }
                } else if self.rs_type_size == b'w' {
                    if signed { (-32768..=32767).contains(&v) } else { (0..=65535).contains(&v) }
                } else if signed {
                    (i32::MIN as i64..=i32::MAX as i64).contains(&v)
                } else {
                    (0..=u32::MAX as i64).contains(&v)
                };
                if in_range {
                    self.rs_val = v;
                }
                in_range
            }
            b'a' => {
                let Some(v) = parse_int(&self.in_addr, true, false) else {
                    return false;
                };
                if (0..=0x06040000).contains(&v) {
                    self.rs_val = v;
                    true
                } else {
                    false
                }
            }
            _ => {
                let Some(v) = parse_int(&self.in_changes, false, false) else {
                    return false;
                };
                if (0..=0xFFFF).contains(&v) {
                    self.rs_val = v;
                    true
                } else {
                    false
                }
            }
        };
        if !ok {
            return false;
        }
        if self.rs_o == b'd' {
            let Some(v) = parse_int(&self.in_diffby, self.rs_t == b'h', false) else {
                return false;
            };
            self.rs_param = v.abs();
        } else if self.rs_o == b'%' {
            let Some(v) = parse_int(&self.in_modby, self.rs_t == b'h', false) else {
                return false;
            };
            if v == 0 {
                return false;
            }
            self.rs_param = v;
        }
        true
    }

    fn format_value(&self, data: &[u8], vi: usize) -> String {
        let n = self.vsize() as usize;
        let mask = if n >= 8 { u64::MAX } else { (1u64 << (n * 8)) - 1 };
        let raw = Self::read_pool(data, vi, n) & mask;
        match self.rs_t {
            b'h' => match n {
                1 => format!("{:02X}", raw),
                2 => format!("{:04X}", raw),
                _ => format!("{:08X}", raw),
            },
            b'u' => format!("{}", raw),
            _ => format!("{}", sign_extend(raw, n)),
        }
    }
}

fn sign_extend(v: u64, n: usize) -> i64 {
    let bits = n * 8;
    if bits == 0 || bits >= 64 {
        return v as i64;
    }
    if (v >> (bits - 1)) & 1 == 1 {
        (v | !((1u64 << bits) - 1)) as i64
    } else {
        v as i64
    }
}

fn parse_int(s: &str, force_hex: bool, signed: bool) -> Option<i64> {
    let mapped: String = s
        .chars()
        .map(|c| if c == 'O' || c == 'o' { '0' } else { c })
        .collect();
    let mut body = mapped.trim();
    if body.is_empty() {
        return None;
    }
    let mut neg = false;
    loop {
        if let Some(rest) = body.strip_prefix('-') {
            neg = !neg;
            body = rest;
        } else if let Some(rest) = body.strip_prefix('+') {
            body = rest;
        } else {
            break;
        }
    }
    let mut hex = force_hex;
    if body.len() >= 2 && (body.starts_with("0x") || body.starts_with("0X")) {
        hex = true;
        body = &body[2..];
    } else if let Some(rest) = body.strip_prefix('$') {
        hex = true;
        body = rest;
    } else if !hex && body.chars().any(|c| matches!(c, 'a'..='f' | 'A'..='F')) {
        hex = true;
    }
    if body.is_empty() {
        return None;
    }
    let v = if hex {
        u64::from_str_radix(body, 16).ok()? as i64
    } else if signed {
        body.parse::<i64>().ok()?
    } else {
        body.parse::<u64>().ok()? as i64
    };
    Some(if neg { -v } else { v })
}

static RAM_STATE: Mutex<RamSearchState> = Mutex::new(RamSearchState::new());
static RAM_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    RAM_ACTIVE.load(Ordering::Relaxed)
}

pub fn deactivate() {
    RAM_ACTIVE.store(false, Ordering::Relaxed);
}

pub fn invalidate() {
    let mut st = RAM_STATE.lock().unwrap();
    st.needs_init = true;
    st.regions.clear();
    st.backup.clear();
    st.undo_type = 0;
    st.result_count = 0;
    st.max_item_index = 0;
    st.item_indices_valid = false;
    st.rs_val_valid = false;
    st.note.clear();
}

pub fn open_window(emu: &mut Emulator) {
    RAM_ACTIVE.store(true, Ordering::Relaxed);
    let mut st = RAM_STATE.lock().unwrap();
    let mut read = |a: u16| emu.debug_peek_mem(a);
    st.init_state(&mut read);
    st.note.clear();
}

pub fn on_frame(emu: &mut Emulator) {
    if !RAM_ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    let mut st = RAM_STATE.lock().unwrap();
    let mut read = |a: u16| emu.debug_peek_mem(a);
    if st.needs_init {
        st.init_state(&mut read);
        return;
    }
    if st.auto_search && st.result_count == 0 {
        st.note = "Autosearch: out of results, resetting.".to_string();
        st.init_state(&mut read);
        return;
    }
    st.update_frame(&mut read);
    if st.auto_search && st.result_count > 0 {
        if !st.rs_val_valid {
            st.rs_val_valid = st.set_rs_val();
        }
        if st.rs_val_valid {
            st.prune();
            if st.result_count == 0 {
                st.note = "Autosearch: out of results, resetting.".to_string();
                st.init_state(&mut read);
            }
        } else {
            st.auto_search = false;
            st.note = "Autosearch stopped: invalid compare value.".to_string();
        }
    }
}

fn do_search(st: &mut RamSearchState, read: &mut dyn FnMut(u16) -> u8) {
    if !st.set_rs_val() {
        st.note = "Invalid or out-of-bound entered value.".to_string();
        if st.auto_search {
            st.auto_search = false;
            st.note = "Invalid compare value, Autosearch disabled.".to_string();
        }
        return;
    }
    if st.result_count > 0 {
        st.save_undo();
        st.prune();
    }
    if st.result_count == 0 {
        st.note = "Resetting search. Out of results.".to_string();
        st.soft_reset(read);
    }
}

fn ram_fit_factor(scale: f32, width: usize, height: usize) -> f32 {
    let cap_w = width.saturating_sub(16) as f32;
    let cap_h = height.saturating_sub(36) as f32;
    let measure = |fit: f32| -> (f32, f32) {
        let s = scale * fit;
        let r = |v: f32| -> f32 { (v * s * 2.0).round() / 2.0 };
        let w = r(8.0) + r(100.0) + r(6.0) + r(216.0) + r(6.0) + r(88.0) + r(6.0) + r(128.0) + r(8.0);
        let h = r(22.0) + r(4.0) + r(14.0) + r(13.0) * 10.0 + r(8.0) + r(100.0) + r(6.0) + r(12.0) + r(6.0) + r(12.0) + r(8.0);
        (w, h)
    };
    let est = ((cap_w / (566.0 * scale)).min(cap_h / (336.0 * scale))).min(1.0).max(0.2);
    let (w0, h0) = measure(est);
    let f1 = est * (cap_w / w0.max(1.0)).min(cap_h / h0.max(1.0));
    let f1 = f1.min(1.0).max(0.2);
    let (w1, h1) = measure(f1);
    let f2 = f1 * (cap_w / w1.max(1.0)).min(cap_h / h1.max(1.0));
    f2.min(f1 * 1.02).min(1.0).max(0.2) * 0.96
}

pub(crate) fn compute_ram_search_layout(width: usize, height: usize, scale: f32) -> RamSearchLayout {
    let fit = ram_fit_factor(scale, width, height);
    let sc = scale * fit;
    let u = |v: f32| -> usize { (v * sc).round() as usize };

    let pad = u(8.0);
    let title_h = u(22.0);
    let list_x = pad;
    let list_y = title_h + u(4.0);
    let header_h = u(14.0);
    let row_h = u(13.0);
    let rows_visible = LIST_ROWS;
    let list_w = u(306.0);
    let list_h = header_h + row_h * rows_visible;
    let col_off = [0usize, u(44.0), u(140.0), u(236.0)];
    let col_w = [u(44.0), u(96.0), u(96.0), u(64.0)];

    let btn_x = list_x + list_w + u(6.0);
    let btn_w = u(112.0);
    let btn_h = u(15.0);
    let btn_gap = u(5.0);
    let mut btn = [(0usize, 0usize, 0usize, 0usize); 6];
    for i in 0..6 {
        btn[i] = (btn_x, list_y + i * (btn_h + btn_gap), btn_w, btn_h);
    }

    let gy = list_y + list_h + u(8.0);
    let group_title_h = u(12.0);
    let group_row_h = u(14.0);
    let group_h = u(100.0);

    let op_x = pad;
    let op_w = u(100.0);
    let op_col_w = op_w.saturating_sub(u(6.0)) / 2;
    let op_ids = [b'<', b'l', b'>', b'm', b'=', b'!'];
    let mut ops = [(0usize, 0usize, 0usize, 0usize); 6];
    for i in 0..6 {
        let cx = op_x + (i % 2) * (op_col_w + u(6.0));
        let cy = gy + group_title_h + (i / 2) * group_row_h;
        ops[i] = (cx, cy, op_col_w, u(12.0));
    }

    let cmp_x = op_x + op_w + u(6.0);
    let cmp_w = u(216.0);
    let cmp_ids = [b'r', b's', b'a', b'n', b'd', b'%'];
    let cmp_labels_w = u(144.0);
    let mut cmp = [(0usize, 0usize, 0usize, 0usize); 6];
    let mut inputs = [(0usize, 0usize, 0usize, 0usize); 5];
    for i in 0..6 {
        let cy = gy + group_title_h + i * group_row_h;
        cmp[i] = (cmp_x + u(4.0), cy, cmp_labels_w, u(12.0));
    }
    for i in 0..5 {
        let cy = gy + group_title_h + (i + 1) * group_row_h;
        inputs[i] = (cmp_x + u(148.0), cy, u(64.0), u(13.0));
    }

    let size_x = cmp_x + cmp_w + u(6.0);
    let size_w = u(88.0);
    let mut size_opts = [(0usize, 0usize, 0usize, 0usize); 3];
    for i in 0..3 {
        let cy = gy + group_title_h + i * group_row_h;
        size_opts[i] = (size_x + u(4.0), cy, size_w.saturating_sub(u(8.0)), u(12.0));
    }
    let misalign = (size_x + u(4.0), gy + group_title_h + 3 * group_row_h, size_w.saturating_sub(u(8.0)), u(12.0));

    let disp_x = size_x + size_w + u(6.0);
    let disp_w = u(128.0);
    let mut disp_opts = [(0usize, 0usize, 0usize, 0usize); 3];
    for i in 0..3 {
        let cy = gy + group_title_h + i * group_row_h;
        disp_opts[i] = (disp_x + u(4.0), cy, disp_w.saturating_sub(u(8.0)), u(12.0));
    }
    let auto_chk = (disp_x + u(4.0), gy + group_title_h + 3 * group_row_h, disp_w.saturating_sub(u(8.0)), u(12.0));
    let rom_chk = (disp_x + u(4.0), gy + group_title_h + 4 * group_row_h, disp_w.saturating_sub(u(8.0)), u(12.0));

    let poss_y = gy + group_h + u(6.0);
    let note_y = poss_y + u(12.0) + u(6.0);
    let win_w = disp_x + disp_w + pad;
    let win_h = note_y + u(12.0) + pad;
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;
    let close_w = u(20.0);
    let close_h = u(20.0);
    let close_x = win_x + win_w.saturating_sub(close_w + u(6.0));
    let close_y = win_y + u(2.0);

    let map = |r: (usize, usize, usize, usize)| -> (usize, usize, usize, usize) {
        (win_x + r.0, win_y + r.1, r.2, r.3)
    };
    let mut m_btn = [(0usize, 0usize, 0usize, 0usize); 6];
    for i in 0..6 {
        m_btn[i] = map(btn[i]);
    }
    let mut m_ops = [(0usize, 0usize, 0usize, 0usize); 6];
    for i in 0..6 {
        m_ops[i] = map(ops[i]);
    }
    let mut m_cmp = [(0usize, 0usize, 0usize, 0usize); 6];
    for i in 0..6 {
        m_cmp[i] = map(cmp[i]);
    }
    let mut m_inputs = [(0usize, 0usize, 0usize, 0usize); 5];
    for i in 0..5 {
        m_inputs[i] = map(inputs[i]);
    }
    let mut m_size = [(0usize, 0usize, 0usize, 0usize); 3];
    for i in 0..3 {
        m_size[i] = map(size_opts[i]);
    }
    let mut m_disp = [(0usize, 0usize, 0usize, 0usize); 3];
    for i in 0..3 {
        m_disp[i] = map(disp_opts[i]);
    }
    let mut m_col_x = [0usize; 4];
    for i in 0..4 {
        m_col_x[i] = win_x + list_x + col_off[i];
    }

    RamSearchLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        title_h,
        close: (close_x, close_y, close_w, close_h),
        list: (win_x + list_x, win_y + list_y, list_w, list_h),
        header_h,
        row_h,
        rows_visible,
        col_x: m_col_x,
        col_w,
        btn: m_btn,
        ops: m_ops,
        op_ids,
        cmp: m_cmp,
        cmp_ids,
        inputs: m_inputs,
        size_opts: m_size,
        misalign: map(misalign),
        disp_opts: m_disp,
        auto_chk: map(auto_chk),
        rom_chk: map(rom_chk),
        poss_y: win_y + poss_y,
        note_y: win_y + note_y,
    }
}

pub(crate) struct RamSearchLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub title_h: usize,
    pub close: (usize, usize, usize, usize),
    pub list: (usize, usize, usize, usize),
    pub header_h: usize,
    pub row_h: usize,
    pub rows_visible: usize,
    pub col_x: [usize; 4],
    pub col_w: [usize; 4],
    pub btn: [(usize, usize, usize, usize); 6],
    pub ops: [(usize, usize, usize, usize); 6],
    pub op_ids: [u8; 6],
    pub cmp: [(usize, usize, usize, usize); 6],
    pub cmp_ids: [u8; 6],
    pub inputs: [(usize, usize, usize, usize); 5],
    pub size_opts: [(usize, usize, usize, usize); 3],
    pub misalign: (usize, usize, usize, usize),
    pub disp_opts: [(usize, usize, usize, usize); 3],
    pub auto_chk: (usize, usize, usize, usize),
    pub rom_chk: (usize, usize, usize, usize),
    pub poss_y: usize,
    pub note_y: usize,
}

fn fit_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let keep = max_chars.saturating_sub(1);
        let mut s: String = text.chars().take(keep).collect();
        s.push('…');
        s
    }
}

fn input_string(st: &RamSearchState, focus: usize) -> &str {
    match focus {
        FOCUS_VAL => &st.in_val,
        FOCUS_ADDR => &st.in_addr,
        FOCUS_CHANGES => &st.in_changes,
        FOCUS_DIFFBY => &st.in_diffby,
        FOCUS_MODBY => &st.in_modby,
        _ => "",
    }
}

fn input_enabled(st: &RamSearchState, focus: usize) -> bool {
    match focus {
        FOCUS_VAL => st.rs_c == b's',
        FOCUS_ADDR => st.rs_c == b'a',
        FOCUS_CHANGES => st.rs_c == b'n',
        FOCUS_DIFFBY => st.rs_o == b'd',
        FOCUS_MODBY => st.rs_o == b'%',
        _ => false,
    }
}

pub(crate) fn render_ram_search_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    scale: f32,
) {
    let l = compute_ram_search_layout(width, height, scale);
    let sc = scale * ram_fit_factor(scale, width, height);
    let (mx, my) = ms.mouse_pos;
    let char_px = (8.0 * sc).round() as usize;
    let char_px = char_px.max(1);

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h.saturating_sub(1), l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y, 1, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x + l.win_w.saturating_sub(1), l.win_y, 1, l.win_h, width, colors.window_border);

    draw_rect(buffer, l.win_x + 1, l.win_y + 1, l.win_w.saturating_sub(2), l.title_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.win_x, l.win_y + l.title_h + 1, l.win_w, 1, width, colors.window_border);
    let title_ty = l.win_y + ((l.title_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "RAM Search", colors.menu_text, sc);

    let (cx, cy, cw, ch) = l.close;
    let close_hover = point_in_rect(mx, my, cx, cy, cw, ch);
    draw_rect(buffer, cx, cy, cw, ch, width, if close_hover { colors.menu_highlight } else { colors.close_bg });
    let close_ty = cy + ((ch as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, cx + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, sc);

    let st = RAM_STATE.lock().unwrap();
    let mut count = st.result_count as usize;
    if st.needs_init || !st.item_indices_valid {
        count = 0;
    }
    let scroll = ms.ram_search_scroll.min(count.saturating_sub(l.rows_visible));

    let (lx, ly, lw, lh) = l.list;
    draw_rect(buffer, lx, ly, lw, lh, width, colors.box_bg_default);
    draw_rect(buffer, lx, ly, lw, 1, width, colors.btn_border);
    draw_rect(buffer, lx, ly + lh.saturating_sub(1), lw, 1, width, colors.btn_border);
    draw_rect(buffer, lx, ly, 1, lh, width, colors.btn_border);
    draw_rect(buffer, lx + lw.saturating_sub(1), ly, 1, lh, width, colors.btn_border);

    draw_rect(buffer, lx + 1, ly + 1, lw.saturating_sub(2), l.header_h.saturating_sub(1), width, colors.dropdown_bg);
    let headers = ["Addr", "Value", "Prev", "Changes"];
    for i in 0..4 {
        let label = fit_text(headers[i], l.col_w[i] / char_px);
        let ty = ly + ((l.header_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, l.col_x[i] + (4.0 * sc).round() as usize, ty, width, &label, colors.btn_sub_label, sc);
    }

    for r in 0..l.rows_visible {
        let item = scroll + r;
        if item >= count {
            break;
        }
        let ry = ly + l.header_h + r * l.row_h;
        let selected = ms.ram_search_selected == Some(item);
        if selected {
            draw_rect(buffer, lx + 1, ry, lw.saturating_sub(2), l.row_h, width, colors.menu_highlight);
        }
        let Some((hw, vi)) = st.item_info(item as u32) else {
            continue;
        };
        let satisfied = st.item_satisfied(item as u32);
        let text_color = if satisfied { colors.menu_text } else { RED_TEXT };
        let addr_str = format!("{:04X}", hw);
        let val_str = st.format_value(&st.cur, vi);
        let prev_str = st.format_value(&st.prev, vi);
        let chg_str = format!("{}", st.changes[vi]);
        let texts = [addr_str, val_str, prev_str, chg_str];
        let row_ty = ry + ((l.row_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        for i in 0..4 {
            let shown = fit_text(&texts[i], l.col_w[i].saturating_sub((6.0 * sc).round() as usize) / char_px);
            draw_text(buffer, l.col_x[i] + (4.0 * sc).round() as usize, row_ty, width, &shown, text_color, sc);
        }
    }

    let btn_labels: [&str; 6] = [
        "Search",
        "Reset",
        "Eliminate",
        if st.undo_type == 2 { "Redo" } else { "Undo" },
        "Clear Counts",
        "Add Cheat",
    ];
    let btn_enabled = [
        true,
        true,
        ms.ram_search_selected.is_some() && count > 0,
        st.undo_type > 0,
        true,
        ms.ram_search_selected.is_some() && count > 0,
    ];
    for i in 0..6 {
        let (bx, by, bw, bh) = l.btn[i];
        let enabled = btn_enabled[i];
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        draw_rect(buffer, bx, by, bw, bh, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buffer, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), width, if hover { colors.box_bg_hover } else if enabled { colors.box_bg_default } else { colors.disabled_btn_bg });
        let label = fit_text(btn_labels[i], bw.saturating_sub((6.0 * sc).round() as usize) / char_px);
        let tw = label.chars().count() * char_px;
        let tx = bx + bw.saturating_sub(tw) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, tx, ty, width, &label, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
    }

    let radio = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, selected: bool, enabled: bool| {
        let (bx, by, bw, bh) = r;
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        if hover {
            draw_rect(buf, bx, by, bw, bh, width, colors.menu_highlight);
        }
        let border = if selected { colors.menu_text } else { colors.btn_border };
        draw_rect(buf, bx, by, bw, 1, width, border);
        draw_rect(buf, bx, by + bh.saturating_sub(1), bw, 1, width, border);
        draw_rect(buf, bx, by, 1, bh, width, border);
        draw_rect(buf, bx + bw.saturating_sub(1), by, 1, bh, width, border);
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        let shown = fit_text(text, bw.saturating_sub((6.0 * sc).round() as usize) / char_px);
        draw_text(buf, bx + (4.0 * sc).round() as usize, ty, width, &shown, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
        if selected {
            draw_rect(buf, bx + 2, by + bh.saturating_sub(3), bw.saturating_sub(4), 1, width, colors.menu_text);
        }
    };

    let check = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, checked: bool, enabled: bool| {
        let (bx, by, bw, bh) = r;
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        if hover {
            draw_rect(buf, bx, by, bw, bh, width, colors.menu_highlight);
        }
        let box_sz = u16px(sc);
        draw_rect(buf, bx, by + bh.saturating_sub(box_sz) / 2, box_sz, box_sz, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buf, bx + 1, by + bh.saturating_sub(box_sz) / 2 + 1, box_sz.saturating_sub(2), box_sz.saturating_sub(2), width, colors.box_bg_default);
        if checked {
            draw_rect(buf, bx + 3, by + bh.saturating_sub(box_sz) / 2 + 3, box_sz.saturating_sub(6), box_sz.saturating_sub(6), width, colors.menu_text);
        }
        let tx = bx + box_sz + (5.0 * sc).round() as usize;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        let shown = fit_text(text, bw.saturating_sub(tx - bx) / char_px);
        draw_text(buf, tx, ty, width, &shown, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
    };

    let group_box = |buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, title: &str| {
        draw_rect(buf, x, y, w, h, width, colors.menu_bg);
        draw_rect(buf, x, y, w, 1, width, colors.btn_border);
        draw_rect(buf, x, y + h.saturating_sub(1), w, 1, width, colors.btn_border);
        draw_rect(buf, x, y, 1, h, width, colors.btn_border);
        draw_rect(buf, x + w.saturating_sub(1), y, 1, h, width, colors.btn_border);
        let ty = y + (2.0 * sc).round() as usize;
        let shown = fit_text(title, w.saturating_sub((8.0 * sc).round() as usize) / char_px);
        draw_text(buf, x + (6.0 * sc).round() as usize, ty, width, &shown, colors.btn_sub_label, sc);
    };

    let gy_abs = l.list.1 + l.list.3 + (8.0 * sc).round() as usize;
    let group_h_abs = (100.0 * sc).round() as usize;
    group_box(buffer, l.ops[0].0.saturating_sub((4.0 * sc).round() as usize), gy_abs, l.cmp[0].0.saturating_sub(l.ops[0].0).saturating_sub((2.0 * sc).round() as usize), group_h_abs, "Operator");
    group_box(buffer, l.cmp[0].0.saturating_sub((4.0 * sc).round() as usize), gy_abs, l.size_opts[0].0.saturating_sub(l.cmp[0].0).saturating_sub((2.0 * sc).round() as usize), group_h_abs, "Compare To / By");
    group_box(buffer, l.size_opts[0].0.saturating_sub((4.0 * sc).round() as usize), gy_abs, l.disp_opts[0].0.saturating_sub(l.size_opts[0].0).saturating_sub((2.0 * sc).round() as usize), group_h_abs, "Data Size");
    group_box(buffer, l.disp_opts[0].0.saturating_sub((4.0 * sc).round() as usize), gy_abs, (l.win_x + l.win_w).saturating_sub(l.disp_opts[0].0).saturating_sub((8.0 * sc).round() as usize), group_h_abs, "Display");

    let op_labels = ["<", "<=", ">", ">=", "=", "!="];
    for i in 0..6 {
        radio(buffer, l.ops[i], op_labels[i], st.rs_o == l.op_ids[i], true);
    }

    let cmp_labels = ["Prev Value", "Specific Value", "Specific Address", "Number of Changes", "Different By", "Modulo Is"];
    for i in 0..6 {
        let id = l.cmp_ids[i];
        let selected = if id == b'd' || id == b'%' {
            st.rs_o == id
        } else {
            st.rs_c == id
        };
        radio(buffer, l.cmp[i], cmp_labels[i], selected, true);
    }

    let input_focus_ids = [FOCUS_VAL, FOCUS_ADDR, FOCUS_CHANGES, FOCUS_DIFFBY, FOCUS_MODBY];
    for i in 0..5 {
        let (bx, by, bw, bh) = l.inputs[i];
        let focus_id = input_focus_ids[i];
        let enabled = input_enabled(&st, focus_id);
        let focused = ms.ram_search_focus == focus_id;
        draw_rect(buffer, bx, by, bw, bh, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buffer, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), width, if enabled { colors.box_bg_default } else { colors.disabled_btn_bg });
        let value = input_string(&st, focus_id).to_string();
        let max_chars = bw.saturating_sub((8.0 * sc).round() as usize) / char_px;
        let shown: String = value.chars().skip(value.chars().count().saturating_sub(max_chars)).collect();
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        let text_color = if enabled { colors.menu_text } else { colors.disabled_text };
        draw_text(buffer, bx + (5.0 * sc).round() as usize, ty, width, &shown, text_color, sc);
        if focused && enabled {
            let caret_x = bx + (5.0 * sc).round() as usize + shown.chars().count() * char_px;
            draw_rect(buffer, caret_x.min(bx + bw.saturating_sub(3)), by + (3.0 * sc).round() as usize, (2.0 * sc).round() as usize, bh.saturating_sub((6.0 * sc).round() as usize), width, text_color);
        }
    }

    let size_labels = ["1 byte", "2 bytes", "4 bytes"];
    let size_ids = [b'b', b'w', b'd'];
    for i in 0..3 {
        radio(buffer, l.size_opts[i], size_labels[i], st.rs_type_size == size_ids[i], true);
    }
    check(buffer, l.misalign, "Misalign", !st.no_misalign, true);

    let disp_labels = ["Signed", "Unsigned", "Hexadecimal"];
    let disp_ids = [b's', b'u', b'h'];
    for i in 0..3 {
        radio(buffer, l.disp_opts[i], disp_labels[i], st.rs_t == disp_ids[i], true);
    }
    check(buffer, l.auto_chk, "Autosearch", st.auto_search, true);
    check(buffer, l.rom_chk, "Search ROM", st.show_rom, true);

    let poss_text = format!("{} possibilities / {} regions", count, if st.needs_init { 0 } else { st.regions.len() });
    let max_poss = l.win_w.saturating_sub((16.0 * sc).round() as usize) / char_px;
    draw_text(buffer, l.win_x + (8.0 * sc).round() as usize, l.poss_y, width, &fit_text(&poss_text, max_poss), colors.btn_sub_label, sc);

    let note_color = if st.note.is_empty() { colors.disabled_text } else { RED_TEXT };
    let note_text = if st.note.is_empty() { "Ready" } else { &st.note };
    draw_text(buffer, l.win_x + (8.0 * sc).round() as usize, l.note_y, width, &fit_text(note_text, max_poss), note_color, sc);
}

fn u16px(sc: f32) -> usize {
    ((12.0 * sc).round() as usize).max(4)
}

pub(crate) fn handle_ram_search_click(
    ms: &mut MenuState,
    emu: &mut Emulator,
    button: winit::event::MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) {
    if button != winit::event::MouseButton::Left {
        return;
    }
    let l = compute_ram_search_layout(width, height, scale);

    let (cx, cy, cw, ch) = l.close;
    if point_in_rect(mx, my, cx, cy, cw, ch) {
        ms.show_ram_search_window = false;
        deactivate();
        return;
    }

    let (lx, ly, lw, lh) = l.list;
    if point_in_rect(mx, my, lx, ly, lw, lh) && my >= ly + l.header_h {
        let st = RAM_STATE.lock().unwrap();
        let count = if st.needs_init || !st.item_indices_valid { 0 } else { st.result_count as usize };
        drop(st);
        let scroll = ms.ram_search_scroll.min(count.saturating_sub(l.rows_visible));
        let row = (my - ly - l.header_h) / l.row_h;
        let item = scroll + row;
        if item < count {
            ms.ram_search_selected = Some(item);
        } else {
            ms.ram_search_selected = None;
        }
        return;
    }

    let btn_ids = [BTN_SEARCH, BTN_RESET, BTN_ELIMINATE, BTN_UNDO, BTN_CLEAR, BTN_ADDCHEAT];
    for i in 0..6 {
        let (bx, by, bw, bh) = l.btn[i];
        if point_in_rect(mx, my, bx, by, bw, bh) {
            match btn_ids[i] {
                BTN_SEARCH => {
                    let mut st = RAM_STATE.lock().unwrap();
                    let mut read = |a: u16| emu.debug_peek_mem(a);
                    do_search(&mut st, &mut read);
                }
                BTN_RESET => {
                    let mut st = RAM_STATE.lock().unwrap();
                    st.save_undo();
                    let mut read = |a: u16| emu.debug_peek_mem(a);
                    st.soft_reset(&mut read);
                    st.note.clear();
                    ms.ram_search_selected = None;
                }
                BTN_ELIMINATE => {
                    let Some(item) = ms.ram_search_selected else { return };
                    let mut st = RAM_STATE.lock().unwrap();
                    let Some((hw, _)) = st.item_info(item as u32) else { return };
                    let step = st.step();
                    st.save_undo();
                    st.eliminate_ranges(&[(hw, step)]);
                    st.note.clear();
                    drop(st);
                    ms.ram_search_selected = None;
                }
                BTN_UNDO => {
                    let mut st = RAM_STATE.lock().unwrap();
                    st.do_undo();
                    st.note.clear();
                    ms.ram_search_selected = None;
                }
                BTN_CLEAR => {
                    let mut st = RAM_STATE.lock().unwrap();
                    for c in st.changes.iter_mut() {
                        *c = 0;
                    }
                }
                BTN_ADDCHEAT => {
                    let Some(item) = ms.ram_search_selected else { return };
                    let req = {
                        let st = RAM_STATE.lock().unwrap();
                        st.item_info(item as u32).map(|(hw, _)| (hw, st.vsize()))
                    };
                    let Some((hw, n)) = req else { return };
                    for b in 0..n {
                        let addr = (hw + b) as u16;
                        let v = emu.debug_peek_mem(addr);
                        emu.cheats.add_cheat(CheatEntry::new(
                            format!("{:04X}", addr),
                            CheatType::Custom,
                            format!("{:04X}:{:02X}", addr, v),
                        ));
                    }
                    let mut st = RAM_STATE.lock().unwrap();
                    st.note = format!("Added cheat at {:04X}", hw);
                }
                _ => {}
            }
            return;
        }
    }

    {
        let mut st = RAM_STATE.lock().unwrap();
        for i in 0..6 {
            let (bx, by, bw, bh) = l.ops[i];
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.rs_o = l.op_ids[i];
                st.rs_val_valid = false;
                ms.ram_search_focus = 0;
                return;
            }
        }
        for i in 0..6 {
            let (bx, by, bw, bh) = l.cmp[i];
            if point_in_rect(mx, my, bx, by, bw, bh) {
                let id = l.cmp_ids[i];
                if id == b'd' || id == b'%' {
                    st.rs_o = id;
                } else {
                    st.rs_c = id;
                }
                st.rs_val_valid = false;
                ms.ram_search_focus = 0;
                return;
            }
        }
        for i in 0..3 {
            let (bx, by, bw, bh) = l.size_opts[i];
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.rs_type_size = [b'b', b'w', b'd'][i];
                st.rs_val_valid = false;
                st.compact();
                ms.ram_search_selected = ms.ram_search_selected.and_then(|s| {
                    if (s as u32) < st.max_item_index {
                        Some(s)
                    } else {
                        None
                    }
                });
                return;
            }
        }
        {
            let (bx, by, bw, bh) = l.misalign;
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.no_misalign = !st.no_misalign;
                st.rs_val_valid = false;
                st.compact();
                return;
            }
        }
        for i in 0..3 {
            let (bx, by, bw, bh) = l.disp_opts[i];
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.rs_t = [b's', b'u', b'h'][i];
                st.rs_val_valid = false;
                return;
            }
        }
        {
            let (bx, by, bw, bh) = l.auto_chk;
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.auto_search = !st.auto_search;
                st.note.clear();
                return;
            }
        }
        {
            let (bx, by, bw, bh) = l.rom_chk;
            if point_in_rect(mx, my, bx, by, bw, bh) {
                st.show_rom = !st.show_rom;
                st.save_undo();
                let mut read = |a: u16| emu.debug_peek_mem(a);
                st.soft_reset(&mut read);
                st.note.clear();
                drop(st);
                ms.ram_search_selected = None;
                return;
            }
        }
        let input_focus_ids = [FOCUS_VAL, FOCUS_ADDR, FOCUS_CHANGES, FOCUS_DIFFBY, FOCUS_MODBY];
        for i in 0..5 {
            let (bx, by, bw, bh) = l.inputs[i];
            if point_in_rect(mx, my, bx, by, bw, bh) {
                let focus_id = input_focus_ids[i];
                if input_enabled(&st, focus_id) {
                    ms.ram_search_focus = focus_id;
                }
                return;
            }
        }
    }

    ms.ram_search_focus = FOCUS_LIST;
}

const BTN_SEARCH: usize = 0;
const BTN_RESET: usize = 1;
const BTN_ELIMINATE: usize = 2;
const BTN_UNDO: usize = 3;
const BTN_CLEAR: usize = 4;
const BTN_ADDCHEAT: usize = 5;

pub(crate) fn handle_ram_search_scroll(ms: &mut MenuState, amount: i32) {
    let count = {
        let st = RAM_STATE.lock().unwrap();
        if st.needs_init || !st.item_indices_valid {
            0
        } else {
            st.result_count as usize
        }
    };
    let cur = ms.ram_search_scroll as i32 - amount;
    ms.ram_search_scroll = cur.max(0).min(count as i32).max(0) as usize;
}

pub(crate) fn handle_ram_search_key(ms: &mut MenuState, emu: &mut Emulator, keycode: winit::event::VirtualKeyCode) {
    match keycode {
        winit::event::VirtualKeyCode::Escape => {
            ms.show_ram_search_window = false;
            deactivate();
        }
        winit::event::VirtualKeyCode::Return => {
            let mut st = RAM_STATE.lock().unwrap();
            let mut read = |a: u16| emu.debug_peek_mem(a);
            do_search(&mut st, &mut read);
        }
        winit::event::VirtualKeyCode::Tab => {
            ms.ram_search_focus = (ms.ram_search_focus + 1) % 6;
        }
        winit::event::VirtualKeyCode::Up | winit::event::VirtualKeyCode::Down => {
            let count = {
                let st = RAM_STATE.lock().unwrap();
                if st.needs_init || !st.item_indices_valid {
                    0
                } else {
                    st.result_count as usize
                }
            };
            if count == 0 {
                return;
            }
            let sel = match ms.ram_search_selected {
                Some(s) => s,
                None => {
                    ms.ram_search_selected = Some(0);
                    0
                }
            };
            let new_sel = if keycode == winit::event::VirtualKeyCode::Up {
                sel.saturating_sub(1)
            } else {
                (sel + 1).min(count - 1)
            };
            ms.ram_search_selected = Some(new_sel);
            let rows = 10;
            if new_sel < ms.ram_search_scroll {
                ms.ram_search_scroll = new_sel;
            } else if new_sel >= ms.ram_search_scroll + rows {
                ms.ram_search_scroll = new_sel + 1 - rows;
            }
        }
        winit::event::VirtualKeyCode::PageUp => {
            handle_ram_search_scroll(ms, 10);
        }
        winit::event::VirtualKeyCode::PageDown => {
            handle_ram_search_scroll(ms, -10);
        }
        winit::event::VirtualKeyCode::Back => {
            let focus = ms.ram_search_focus;
            let caret = current_caret(ms, focus);
            {
                let mut st = RAM_STATE.lock().unwrap();
                if !input_enabled(&st, focus) {
                    return;
                }
                edit_input(&mut st, focus, EditKind::Backspace, ' ', caret);
                st.rs_val_valid = false;
            }
            if focus == FOCUS_VAL {
                ms.ram_search_val_caret = ms.ram_search_val_caret.saturating_sub(1);
            } else if focus == FOCUS_ADDR {
                ms.ram_search_addr_caret = ms.ram_search_addr_caret.saturating_sub(1);
            } else if focus == FOCUS_CHANGES {
                ms.ram_search_changes_caret = ms.ram_search_changes_caret.saturating_sub(1);
            } else if focus == FOCUS_DIFFBY {
                ms.ram_search_diffby_caret = ms.ram_search_diffby_caret.saturating_sub(1);
            } else if focus == FOCUS_MODBY {
                ms.ram_search_modby_caret = ms.ram_search_modby_caret.saturating_sub(1);
            }
        }
        winit::event::VirtualKeyCode::Delete => {
            let focus = ms.ram_search_focus;
            let caret = current_caret(ms, focus);
            let mut st = RAM_STATE.lock().unwrap();
            if !input_enabled(&st, focus) {
                return;
            }
            edit_input(&mut st, focus, EditKind::Delete, ' ', caret);
            st.rs_val_valid = false;
        }
        winit::event::VirtualKeyCode::Left => {
            match ms.ram_search_focus {
                FOCUS_VAL => ms.ram_search_val_caret = ms.ram_search_val_caret.saturating_sub(1),
                FOCUS_ADDR => ms.ram_search_addr_caret = ms.ram_search_addr_caret.saturating_sub(1),
                FOCUS_CHANGES => ms.ram_search_changes_caret = ms.ram_search_changes_caret.saturating_sub(1),
                FOCUS_DIFFBY => ms.ram_search_diffby_caret = ms.ram_search_diffby_caret.saturating_sub(1),
                FOCUS_MODBY => ms.ram_search_modby_caret = ms.ram_search_modby_caret.saturating_sub(1),
                _ => {}
            }
        }
        winit::event::VirtualKeyCode::Right => {
            let focus = ms.ram_search_focus;
            let len = {
                let st = RAM_STATE.lock().unwrap();
                input_string(&st, focus).chars().count()
            };
            match focus {
                FOCUS_VAL => ms.ram_search_val_caret = (ms.ram_search_val_caret + 1).min(len),
                FOCUS_ADDR => ms.ram_search_addr_caret = (ms.ram_search_addr_caret + 1).min(len),
                FOCUS_CHANGES => ms.ram_search_changes_caret = (ms.ram_search_changes_caret + 1).min(len),
                FOCUS_DIFFBY => ms.ram_search_diffby_caret = (ms.ram_search_diffby_caret + 1).min(len),
                FOCUS_MODBY => ms.ram_search_modby_caret = (ms.ram_search_modby_caret + 1).min(len),
                _ => {}
            }
        }
        _ => {}
    }
}

fn current_caret(ms: &MenuState, focus: usize) -> usize {
    match focus {
        FOCUS_VAL => ms.ram_search_val_caret,
        FOCUS_ADDR => ms.ram_search_addr_caret,
        FOCUS_CHANGES => ms.ram_search_changes_caret,
        FOCUS_DIFFBY => ms.ram_search_diffby_caret,
        FOCUS_MODBY => ms.ram_search_modby_caret,
        _ => 0,
    }
}

enum EditKind {
    Insert,
    Backspace,
    Delete,
}

fn edit_input(st: &mut RamSearchState, focus: usize, kind: EditKind, c: char, caret: usize) {
    let s = match focus {
        FOCUS_VAL => &mut st.in_val,
        FOCUS_ADDR => &mut st.in_addr,
        FOCUS_CHANGES => &mut st.in_changes,
        FOCUS_DIFFBY => &mut st.in_diffby,
        FOCUS_MODBY => &mut st.in_modby,
        _ => return,
    };
    let len = s.chars().count();
    let caret = caret.min(len);
    match kind {
        EditKind::Insert => {
            if len >= MAX_INPUT_LEN {
                return;
            }
            s.insert(caret, c);
        }
        EditKind::Backspace => {
            if caret > 0 {
                s.remove(caret - 1);
            }
        }
        EditKind::Delete => {
            if caret < len {
                s.remove(caret);
            }
        }
    }
}

pub(crate) fn handle_ram_search_char(ms: &mut MenuState, c: char) {
    let focus = ms.ram_search_focus;
    if focus == FOCUS_LIST {
        return;
    }
    let allowed = c.is_ascii_alphanumeric() || c == '$' || c == '-' || c == '+';
    if !allowed {
        return;
    }
    let caret = current_caret(ms, focus);
    {
        let mut st = RAM_STATE.lock().unwrap();
        if !input_enabled(&st, focus) {
            return;
        }
        edit_input(&mut st, focus, EditKind::Insert, c, caret);
        st.rs_val_valid = false;
    }
    match focus {
        FOCUS_VAL => ms.ram_search_val_caret = caret + 1,
        FOCUS_ADDR => ms.ram_search_addr_caret = caret + 1,
        FOCUS_CHANGES => ms.ram_search_changes_caret = caret + 1,
        FOCUS_DIFFBY => ms.ram_search_diffby_caret = caret + 1,
        FOCUS_MODBY => ms.ram_search_modby_caret = caret + 1,
        _ => {}
    }
}
