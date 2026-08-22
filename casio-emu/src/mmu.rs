// MMU + chipset + minimal peripherals for the ClassWiz (nX-U16) target,
// ported 1:1 from CasioEmuMsvc (GPL-3.0).
// Sources: src/Chipset/MMU.cpp, MMURegion.hpp, Chipset.cpp (interrupt SFR,
// AcceptInterrupt), src/Peripheral/Miscellaneous.cpp, Timer.cpp,
// Keyboard.cpp, Screen.cpp, PowerSupply.cpp.

use crate::cpu::Cpu;

pub const INT_RESET: usize = 1;
pub const INT_BREAK: usize = 2;
pub const INT_EMULATOR: usize = 3;
pub const INT_NONMASKABLE: usize = 4;
pub const INT_MASKABLE: usize = 5;
pub const INT_SOFTWARE: usize = 64;
pub const INT_COUNT: usize = 128;

pub const EFFECTIVE_MI_COUNT: usize = 17; // HW_CLASSWIZ
// mask = (1 << (count + 1)) - 2  (WDT disabled)
pub const INT_SFR_MASK: u64 = ((1u64 << (EFFECTIVE_MI_COUNT + 1)) - 2) & 0xFFFF_FFFF;

pub const RAM_BASE: u32 = 0xD000;
pub const RAM_SIZE: usize = 0x2000;
pub const SIM_RAM_BASE: u32 = 0x49800;
pub const SIM_RAM_SIZE: usize = 0x2800;
pub const SCREEN_BASE: u32 = 0xF800;
pub const SCREEN_SIZE: usize = 0x800;

pub const BUTTON_KIKO_POWER: u8 = 0xFF;
pub const BUTTON_KIKO_RESET: u8 = 0xFE;

// ---------------------------------------------------------------- Ints

pub struct Ints {
    pub active: [bool; INT_COUNT],
    pub count: usize,
    pub mask_sfr: u64,
    pub pending_sfr: u64,
    pub is_mi_blocked: bool,
}

impl Ints {
    pub fn new() -> Ints {
        Ints {
            active: [false; INT_COUNT],
            count: 0,
            mask_sfr: 0,
            pending_sfr: 0,
            is_mi_blocked: false,
        }
    }

    #[inline]
    pub fn get_pending_bit(&self, index: usize) -> bool {
        (self.pending_sfr >> (index - 4)) & 1 != 0
    }

    #[inline]
    pub fn set_pending_bit(&mut self, index: usize, val: bool) {
        if val {
            if index > INT_NONMASKABLE {
                self.pending_sfr |= 1u64 << (index - 4);
            }
        } else if index >= INT_NONMASKABLE {
            self.pending_sfr &= !(1u64 << (index - 4));
        }
    }

    pub fn raise_software(&mut self, index: usize) {
        let index = index + 0x40;
        if self.active[index] {
            return;
        }
        self.active[index] = true;
        self.count += 1;
    }

    pub fn raise_break(&mut self) {
        if self.active[INT_BREAK] {
            return;
        }
        self.active[INT_BREAK] = true;
        self.count += 1;
    }

    pub fn raise_emulator(&mut self) {
        if self.active[INT_EMULATOR] {
            return;
        }
        self.active[INT_EMULATOR] = true;
        self.count += 1;
    }

    pub fn raise_reset(&mut self) {
        if self.active[INT_RESET] {
            return;
        }
        self.active[INT_RESET] = true;
        self.count += 1;
    }

    pub fn raise_nonmaskable(&mut self) {
        if self.active[INT_NONMASKABLE] {
            return;
        }
        self.active[INT_NONMASKABLE] = true;
        self.count += 1;
    }

    pub fn reset_nonmaskable(&mut self) {
        if !self.active[INT_NONMASKABLE] {
            return;
        }
        self.active[INT_NONMASKABLE] = false;
        self.count -= 1;
    }

    pub fn raise_maskable(&mut self, index: usize) {
        if index < INT_MASKABLE || index >= INT_SOFTWARE {
            return;
        }
        if self.active[index] {
            return;
        }
        self.active[index] = true;
        self.count += 1;
    }

    pub fn reset_maskable(&mut self, index: usize) {
        if index < INT_MASKABLE || index >= INT_SOFTWARE {
            return;
        }
        if !self.active[index] {
            return;
        }
        self.active[index] = false;
        self.count -= 1;
    }

    /// k is the index into the MaskableInterrupts array (0..EffectiveMICount),
    /// i.e. vector index INT_MASKABLE + k, SFR pending bit k + 1.
    pub fn try_raise_maskable(&mut self, k: usize) {
        let index = INT_MASKABLE + k;
        self.set_pending_bit(index, true);
        if self.mask_sfr & (1u64 << (k + 1)) != 0 {
            self.raise_maskable(index);
        }
    }

    pub fn reset_interrupt_sfr(&mut self) {
        self.mask_sfr = 0;
        self.pending_sfr = 0;
        for i in 0..EFFECTIVE_MI_COUNT {
            self.reset_maskable(INT_MASKABLE + i);
        }
        self.reset_nonmaskable();
    }
}

// ---------------------------------------------------------------- Timer

pub struct Timer {
    pub interval: u16,
    pub counter: u16,
    pub f024: u8,
    pub control: u8,
    pub ext_to_int_counter: u64,
    pub timer_freq_div: u32,
}

pub const TM0INT: usize = 4; // index into MaskableInterrupts array

impl Timer {
    pub fn new() -> Timer {
        Timer {
            interval: 0,
            counter: 0,
            f024: 0,
            control: 0,
            ext_to_int_counter: 0,
            timer_freq_div: 1,
        }
    }

    pub fn reset(&mut self) {
        self.interval = 0;
        self.counter = 0;
        self.f024 = 0;
        self.control = 0;
        self.ext_to_int_counter = 0;
    }

    pub fn tick(&mut self, ints: &mut Ints) {
        let v = if self.interval == 0 { 1 } else { self.interval };
        // EMUCLK path: no data_control gating (port of Timer::Tick)
        let threshold = (v as f64 * self.timer_freq_div as f64) / 32678.0 / 0.025 * 2.0;
        self.ext_to_int_counter += 1;
        if self.ext_to_int_counter as f64 >= threshold {
            self.ext_to_int_counter = 0;
            ints.try_raise_maskable(TM0INT);
        }
    }
}

// ---------------------------------------------------------------- Keyboard

pub struct Keyboard {
    pub ko_mask: u16, // 0xF044, 2 bytes, masked 0x03FF
    pub ko: u16,      // 0xF046, 2 bytes, masked 0x83FF
    pub input_mode: u8,
    pub input_filter: u8,
    pub ki: u8,
    pub pressed: [bool; 64],
    pub p0: bool, // power button held
    pub p1: bool, // reset button held
    pub ghost: [u8; 8],
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            ko_mask: 0,
            ko: 0,
            input_mode: 0,
            input_filter: 0,
            ki: 0xFF,
            pressed: [false; 64],
            p0: false,
            p1: false,
            ghost: [0; 8],
        }
    }

    pub fn press(&mut self, code: u8) {
        if code == BUTTON_KIKO_POWER {
            self.p0 = true;
        } else if code == BUTTON_KIKO_RESET {
            self.p1 = true;
        } else {
            let index = (((code >> 1) & 0x38) | (code & 0x07)) as usize;
            if index < 64 {
                if !self.pressed[index] {
                    self.pressed[index] = true;
                    self.recalculate_ghost();
                }
            }
        }
        self.recalculate_ki();
    }

    pub fn release(&mut self, code: u8) {
        if code == BUTTON_KIKO_POWER {
            self.p0 = false;
        } else if code == BUTTON_KIKO_RESET {
            self.p1 = false;
        } else {
            let index = (((code >> 1) & 0x38) | (code & 0x07)) as usize;
            if index < 64 && self.pressed[index] {
                self.pressed[index] = false;
                self.recalculate_ghost();
            }
        }
        self.recalculate_ki();
    }

    pub fn release_all(&mut self) {
        let had = self.pressed.iter().any(|&p| p) || self.p0 || self.p1;
        if had {
            self.pressed = [false; 64];
            self.p0 = false;
            self.p1 = false;
            self.recalculate_ghost();
            self.recalculate_ki();
        }
    }

    // Port of Keyboard::RecalculateEmuInput's ghost-column pass (8x8 matrix).
    fn recalculate_ghost(&mut self) {
        let mut columns_conn = [0u8; 8];
        let mut ki_rows = [0u8; 8];
        for cx in 0..8usize {
            for rx in 0..8usize {
                if self.pressed[cx * 8 + rx] {
                    ki_rows[cx] |= 1 << rx;
                    for ax in 0..8usize {
                        if self.pressed[ax * 8 + rx] {
                            columns_conn[cx] |= 1 << ax;
                        }
                    }
                }
            }
        }
        let mut ghost = [0u8; 8];
        let mut seen = [false; 8];
        for cx in 0..8usize {
            if seen[cx] {
                continue;
            }
            let mut to_visit = 1u8 << cx;
            let mut ghost_mask = 1u8 << cx;
            seen[cx] = true;
            while to_visit != 0 {
                let mut new_to_visit = 0u8;
                for vx in 0..8usize {
                    if to_visit & (1 << vx) != 0 {
                        for sx in 0..8usize {
                            if columns_conn[vx] & (1 << sx) != 0 && !seen[sx] {
                                new_to_visit |= 1 << sx;
                                ghost_mask |= 1 << sx;
                                seen[sx] = true;
                            }
                        }
                    }
                }
                to_visit = new_to_visit;
            }
            for gx in 0..8usize {
                if ghost_mask & (1 << gx) != 0 {
                    ghost[gx] = ghost_mask;
                }
            }
        }
        self.ghost = ghost;
    }

    // Port of Keyboard::RecalculateKI for the simulator path (single-key
    // model; the exact kiko table for fx-580VN X is still to be extracted).
    fn recalculate_ki(&mut self) {
        let mut keyboard_out_ghosted = 0u8;
        for ix in 0..7usize {
            if self.ko & !self.ko_mask & (1 << ix) != 0 {
                keyboard_out_ghosted |= self.ghost[ix];
            }
        }
        self.ki = !self.input_mode;
        for (index, &pressed) in self.pressed.iter().enumerate() {
            if !pressed {
                continue;
            }
            let ko_bit = 1u16 << (index >> 3);
            let ki_bit = 1u8 << (index & 7);
            if ko_bit & keyboard_out_ghosted as u16 != 0 {
                self.ki &= !ki_bit;
            }
        }
        if self.ko & !self.ko_mask & (1 << 7) != 0 && self.p0 {
            self.ki &= 0x7F;
        }
        if self.ko & !self.ko_mask & (1 << 8) != 0 && self.p1 {
            self.ki &= 0x7F;
        }
    }
}

// ---------------------------------------------------------------- Screen

pub struct ScreenSfr {
    pub range: u8,     // 0xF030
    pub mode: u8,      // 0xF031
    pub contrast: u8,  // 0xF032
    pub brightness: u8, // 0xF033
    pub dspofst: u8,   // 0xF039
    pub power: u8,     // 0xF03D
}

impl ScreenSfr {
    pub fn new() -> ScreenSfr {
        ScreenSfr {
            range: 0,
            mode: 0,
            contrast: 0,
            brightness: 0,
            dspofst: 0,
            power: 0,
        }
    }
}

// ---------------------------------------------------------------- Mmu

pub struct Mmu {
    pub rom: Vec<u8>,
    pub ram: Box<[u8; RAM_SIZE]>,
    pub sim_ram: Box<[u8; SIM_RAM_SIZE]>,
    pub screen_buf: Box<[u8; SCREEN_SIZE]>,
    pub ints: Ints,
    pub timer: Timer,
    pub keyboard: Keyboard,
    pub screen: ScreenSfr,
    pub impl_last_dsr: u8,
    pub dsr_mask: u8,
    pub segment_access: bool,
    pub remap: bool,
    pub exicon: u8,
    pub flash_addr: u16,
    pub flash_segment: u8,
    // catch-all SFR banks for everything not explicitly modelled
    pub sfr_f0: Box<[u8; 0x100]>,
    pub sfr_f2: Box<[u8; 0x100]>,
    pub sfr_f4: Box<[u8; 0x100]>,
    pub real_hardware: bool,
}

impl Mmu {
    pub fn new(rom: Vec<u8>, real_hardware: bool) -> Mmu {
        Mmu {
            rom,
            ram: Box::new([0; RAM_SIZE]),
            sim_ram: Box::new([0; SIM_RAM_SIZE]),
            screen_buf: Box::new([0; SCREEN_SIZE]),
            ints: Ints::new(),
            timer: Timer::new(),
            keyboard: Keyboard::new(),
            screen: ScreenSfr::new(),
            impl_last_dsr: 0,
            dsr_mask: 0x1F,
            segment_access: false,
            remap: false,
            exicon: 0,
            flash_addr: 0,
            flash_segment: 0,
            sfr_f0: Box::new([0; 0x100]),
            sfr_f2: Box::new([0; 0x100]),
            sfr_f4: Box::new([0; 0x100]),
            real_hardware,
        }
    }

    #[inline]
    fn rom_le16(&self, offset: usize) -> u16 {
        if offset + 1 >= self.rom.len() {
            return 0xFFFF;
        }
        self.rom[offset] as u16 | ((self.rom[offset + 1] as u16) << 8)
    }

    pub fn read_code(&mut self, offset: u32) -> u16 {
        let mut segment_index = (offset >> 16) as usize;
        let segment_offset = offset & 0xFFFE;
        if self.segment_access && segment_index == 5 {
            segment_index = 0;
        }
        if segment_index < 4 {
            if self.remap {
                let add = if segment_index == 0 && segment_offset < 0x200 {
                    0xFE00
                } else {
                    0
                };
                return self.rom_le16(offset as usize + add);
            } else {
                if segment_index == 0 && segment_offset >= 0xFE00 {
                    return 0xFFFF;
                }
                return self.rom_le16(offset as usize);
            }
        }
        0
    }

    #[inline]
    pub fn read_data(&mut self, offset: u32) -> u8 {
        match offset {
            0xD000..0xF000 => self.ram[(offset - RAM_BASE) as usize],
            0x49800..0x4C000 => self.sim_ram[(offset - SIM_RAM_BASE) as usize],
            0xF800..0x10000 => self.screen_buf[(offset - SCREEN_BASE) as usize],
            0xF000..0xF100 => self.sfr_f0_read(offset - 0xF000),
            0xF200..0xF300 => self.sfr_f2[(offset - 0xF200) as usize],
            0xF400..0xF500 => self.sfr_f4[(offset - 0xF400) as usize],
            _ => 0,
        }
    }

    #[inline]
    pub fn write_data(&mut self, offset: u32, data: u8) {
        match offset {
            0xD000..0xF000 => {
                self.ram[(offset - RAM_BASE) as usize] = data;
            }
            0x49800..0x4C000 => {
                self.sim_ram[(offset - SIM_RAM_BASE) as usize] = data;
            }
            0xF800..0x10000 => {
                self.screen_buf[(offset - SCREEN_BASE) as usize] = data;
            }
            0xF000..0xF100 => self.sfr_f0_write(offset - 0xF000, data),
            0xF200..0xF300 => {
                self.sfr_f2[(offset - 0xF200) as usize] = data;
            }
            0xF400..0xF500 => {
                self.sfr_f4[(offset - 0xF400) as usize] = data;
            }
            _ => {}
        }
    }

    fn sfr_f0_read(&mut self, off: u32) -> u8 {
        let off = off as usize;
        match off {
            0x00 => self.impl_last_dsr,
            0x04 => self.segment_access as u8 & 1,
            0x10..=0x13 => ((self.ints.mask_sfr >> ((off - 0x10) * 8)) & 0xFF) as u8,
            0x14..=0x17 => ((self.ints.pending_sfr >> ((off - 0x14) * 8)) & 0xFF) as u8,
            0x18 => self.exicon,
            0x20..=0x21 => ((self.timer.interval >> ((off - 0x20) * 8)) & 0xFF) as u8,
            0x22..=0x23 => ((self.timer.counter >> ((off - 0x22) * 8)) & 0xFF) as u8,
            0x24 => self.timer.f024 & 0x0F,
            0x25 => self.timer.control & 0x01,
            0x30 => self.screen.range,
            0x31 => self.screen.mode,
            0x32 => self.screen.contrast,
            0x33 => self.screen.brightness,
            0x39 => self.screen.dspofst,
            0x3D => self.screen.power,
            0x40 => self.keyboard.ki,
            0x41 => self.keyboard.input_mode,
            0x42 => self.keyboard.input_filter,
            0x44..=0x45 => ((self.keyboard.ko_mask >> ((off - 0x44) * 8)) & 0xFF) as u8,
            0x46..=0x47 => ((self.keyboard.ko >> ((off - 0x46) * 8)) & 0xFF) as u8,
            0xE0..=0xE1 => ((self.flash_addr >> ((off - 0xE0) * 8)) & 0xFF) as u8,
            0xE6 => self.flash_segment,
            _ => self.sfr_f0[off],
        }
    }

    fn sfr_f0_write(&mut self, off: u32, data: u8) {
        let off = off as usize;
        match off {
            0x00 => self.impl_last_dsr = data & self.dsr_mask,
            0x04 => self.segment_access = data & 1 != 0,
            0x10..=0x13 => {
                let shift = (off - 0x10) * 8;
                self.ints.mask_sfr =
                    (self.ints.mask_sfr & !(0xFFu64 << shift)) | ((data as u64) << shift);
                self.ints.mask_sfr &= INT_SFR_MASK;
                for i in 0..EFFECTIVE_MI_COUNT {
                    let bit = 1u64 << (i + 1);
                    if self.ints.pending_sfr & bit != 0 {
                        if self.ints.mask_sfr & bit != 0 {
                            self.ints.raise_maskable(INT_MASKABLE + i);
                        } else {
                            self.ints.reset_maskable(INT_MASKABLE + i);
                        }
                    }
                }
                if self.ints.mask_sfr & 1 != 0 {
                    if self.ints.get_pending_bit(INT_NONMASKABLE) {
                        self.ints.raise_nonmaskable();
                    }
                } else {
                    self.ints.reset_nonmaskable();
                }
            }
            0x14..=0x17 => {
                let shift = (off - 0x14) * 8;
                self.ints.pending_sfr =
                    (self.ints.pending_sfr & !(0xFFu64 << shift)) | ((data as u64) << shift);
                self.ints.pending_sfr &= INT_SFR_MASK;
                for i in 0..EFFECTIVE_MI_COUNT {
                    let bit = 1u64 << (i + 1);
                    if self.ints.pending_sfr & bit != 0 {
                        self.ints.try_raise_maskable(i);
                    } else {
                        self.ints.reset_maskable(INT_MASKABLE + i);
                    }
                }
                if self.ints.pending_sfr & 1 != 0 {
                    if self.ints.mask_sfr & 1 != 0 {
                        self.ints.raise_nonmaskable();
                    }
                } else {
                    self.ints.reset_nonmaskable();
                }
            }
            0x18 => self.exicon = data,
            0x20..=0x21 => {
                let shift = (off - 0x20) * 8;
                self.timer.interval =
                    (self.timer.interval & !(0xFFu16 << shift)) | ((data as u16) << shift);
            }
            0x22..=0x23 => {
                // writing to TM0C clears the counter
                self.timer.counter = 0;
            }
            0x24 => {
                self.timer.f024 = data & 0x0F;
                self.timer.timer_freq_div = 1 << (data & 0x07);
            }
            0x25 => self.timer.control = data & 0x01,
            0x30 => self.screen.range = data,
            0x31 => self.screen.mode = data,
            0x32 => self.screen.contrast = data,
            0x33 => self.screen.brightness = data,
            0x39 => self.screen.dspofst = data,
            0x3D => self.screen.power = data,
            0x40 => {} // KI is read-only on the simulator path
            0x41 => {
                self.keyboard.input_mode = data;
                self.keyboard.recalculate_ki();
            }
            0x42 => self.keyboard.input_filter = data,
            0x44..=0x45 => {
                let shift = (off - 0x44) * 8;
                self.keyboard.ko_mask =
                    (self.keyboard.ko_mask & !(0xFFu16 << shift)) | ((data as u16) << shift);
                self.keyboard.ko_mask &= 0x03FF;
                if off == 0x44 {
                    self.keyboard.recalculate_ki();
                }
            }
            0x46..=0x47 => {
                let shift = (off - 0x46) * 8;
                self.keyboard.ko =
                    (self.keyboard.ko & !(0xFFu16 << shift)) | ((data as u16) << shift);
                self.keyboard.ko &= 0x83FF;
                if off == 0x46 {
                    self.keyboard.recalculate_ki();
                }
            }
            0xE0..=0xE1 => {
                let shift = (off - 0xE0) * 8;
                self.flash_addr =
                    (self.flash_addr & !(0xFFu16 << shift)) | ((data as u16) << shift);
            }
            0xE6 => self.flash_segment = data & 0x1F,
            _ => self.sfr_f0[off] = data,
        }
    }

    // Port of Chipset::AcceptInterrupt.
    pub fn accept_interrupt(&mut self, cpu: &mut Cpu) {
        if self.ints.count == 0 {
            return;
        }
        let old_exception_level = cpu.exception_level();
        let mut index = 0usize;
        let mut acceptable = true;

        if self.ints.active[INT_RESET] {
            index = INT_RESET;
        }
        if index == 0 {
            for ix in INT_SOFTWARE..INT_COUNT {
                if self.ints.active[ix] {
                    index = ix;
                    break;
                }
            }
        }
        if index == 0 && self.ints.active[INT_EMULATOR] {
            index = INT_EMULATOR;
        }
        if index == 0 && self.ints.active[INT_BREAK] {
            index = INT_BREAK;
        }
        if index == 0 && self.ints.active[INT_NONMASKABLE] {
            index = INT_NONMASKABLE;
            if old_exception_level > 2 {
                acceptable = false;
            }
        }
        if index == 0 {
            for ix in INT_MASKABLE..INT_SOFTWARE {
                if self.ints.active[ix] {
                    index = ix;
                    if old_exception_level > 1 {
                        acceptable = false;
                    }
                    break;
                }
            }
        }
        if index == 0 {
            return;
        }

        let exception_level = match index {
            INT_RESET => 0,
            INT_BREAK | INT_NONMASKABLE => 2,
            INT_EMULATOR => 3,
            _ => 1,
        };

        if index >= INT_MASKABLE && index < INT_SOFTWARE {
            if cpu.mie() && acceptable && !self.ints.is_mi_blocked {
                self.ints.set_pending_bit(index, false);
                cpu.raise(exception_level, index, self);
                self.ints.active[index] = false;
                self.ints.count -= 1;
            }
        } else if index == INT_NONMASKABLE {
            if acceptable {
                cpu.raise(exception_level, index, self);
                self.ints.set_pending_bit(INT_NONMASKABLE, false);
                self.ints.active[index] = false;
                self.ints.count -= 1;
            }
        } else {
            cpu.raise(exception_level, index, self);
            self.ints.active[index] = false;
            self.ints.count -= 1;
        }
    }

    pub fn raise_software(&mut self, index: usize) {
        self.ints.raise_software(index);
    }

    pub fn break_int(&mut self) {
        self.ints.raise_break();
    }

    pub fn raise_emulator(&mut self) {
        self.ints.raise_emulator();
    }

    pub fn tick(&mut self) {
        self.timer.tick(&mut self.ints);
    }

    pub fn reset(&mut self) {
        self.ints.reset_interrupt_sfr();
        self.ints.is_mi_blocked = false;
        self.segment_access = false;
        self.timer.reset();
        self.keyboard.release_all();
        self.screen.range = 0;
        self.screen.mode = 0;
        self.screen.contrast = 0;
        self.screen.brightness = 0;
        self.screen.dspofst = 0;
        self.screen.power = 0;
    }
}