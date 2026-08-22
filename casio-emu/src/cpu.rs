// nX-U16 CPU core, ported 1:1 from CasioEmuMsvc (GPL-3.0).
// See src/Chipset/CPU.cpp, CPUArithmetic.cpp, CPUControl.cpp,
// CPULoadStore.cpp, CPUPushPop.cpp in the C++ project.

use crate::mmu::Mmu;

pub const PSW_C: u8 = 0x80;
pub const PSW_Z: u8 = 0x40;
pub const PSW_S: u8 = 0x20;
pub const PSW_OV: u8 = 0x10;
pub const PSW_MIE: u8 = 0x8;
pub const PSW_HC: u8 = 0x4;
pub const PSW_ELEVEL: u8 = 0x3;

pub const H_IE: u16 = 0x0001;
pub const H_ST: u16 = 0x0002;
pub const H_DW: u16 = 0x0004;
pub const H_DS: u16 = 0x0008;
pub const H_IA: u16 = 0x0010;
pub const H_TI: u16 = 0x0020;
pub const H_WB: u16 = 0x0040;

#[derive(Clone, Copy)]
pub struct Operand {
    pub value: u64,
    pub register_index: usize,
    pub register_size: usize,
}

// (handler index, hint, opcode, operand0 {register_size, mask, shift}, operand1)
pub const OPCODE_SOURCES: [(u8, u16, u16, (usize, u16, u16), (usize, u16, u16)); 175] = [
    // Arithmetic
    (0, H_WB, 0x8001, (1, 0x000F, 8), (1, 0x000F, 4)),
    (0, H_WB, 0x1000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (1, H_WB, 0xF006, (2, 0x000E, 8), (2, 0x000E, 4)),
    (1, H_WB | H_IE, 0xE080, (2, 0x000E, 8), (0, 0x007F, 0)),
    (2, H_WB, 0x8006, (1, 0x000F, 8), (1, 0x000F, 4)),
    (2, H_WB, 0x6000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (3, H_WB, 0x8002, (1, 0x000F, 8), (1, 0x000F, 4)),
    (3, H_WB, 0x2000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (4, 0, 0x8007, (1, 0x000F, 8), (1, 0x000F, 4)),
    (4, 0, 0x7000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (5, 0, 0x8005, (1, 0x000F, 8), (1, 0x000F, 4)),
    (5, 0, 0x5000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (6, H_WB, 0xF005, (2, 0x000E, 8), (2, 0x000E, 4)),
    (6, H_WB | H_IE, 0xE000, (2, 0x000E, 8), (0, 0x007F, 0)),
    (7, H_WB, 0x8000, (1, 0x000F, 8), (1, 0x000F, 4)),
    (7, H_WB, 0x0000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (8, H_WB, 0x8003, (1, 0x000F, 8), (1, 0x000F, 4)),
    (8, H_WB, 0x3000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (9, H_WB, 0x8004, (1, 0x000F, 8), (1, 0x000F, 4)),
    (9, H_WB, 0x4000, (1, 0x000F, 8), (0, 0x00FF, 0)),
    (10, 0, 0xF007, (2, 0x000E, 8), (2, 0x000E, 4)),
    (4, H_WB, 0x8008, (1, 0x000F, 8), (1, 0x000F, 4)),
    (5, H_WB, 0x8009, (1, 0x000F, 8), (1, 0x000F, 4)),
    // Shifts
    (11, H_WB, 0x800A, (1, 0x000F, 8), (1, 0x000F, 4)),
    (11, H_WB, 0x900A, (1, 0x000F, 8), (0, 0x0007, 4)),
    (12, H_WB, 0x800B, (1, 0x000F, 8), (1, 0x000F, 4)),
    (12, H_WB, 0x900B, (1, 0x000F, 8), (0, 0x0007, 4)),
    (13, H_WB, 0x800E, (1, 0x000F, 8), (1, 0x000F, 4)),
    (13, H_WB, 0x900E, (1, 0x000F, 8), (0, 0x0007, 4)),
    (14, H_WB, 0x800C, (1, 0x000F, 8), (1, 0x000F, 4)),
    (14, H_WB, 0x900C, (1, 0x000F, 8), (0, 0x0007, 4)),
    (15, H_WB, 0x800D, (1, 0x000F, 8), (1, 0x000F, 4)),
    (15, H_WB, 0x900D, (1, 0x000F, 8), (0, 0x0007, 4)),
    // Load/store
    (16, 2 << 8, 0x9032, (0, 0x000E, 8), (0, 0, 0)),
    (16, (2 << 8) | H_IA, 0x9052, (0, 0x000E, 8), (0, 0, 0)),
    (17, 2 << 8, 0x9002, (0, 0x000E, 8), (2, 0x000E, 4)),
    (18, (2 << 8) | H_TI, 0xA008, (0, 0x000E, 8), (2, 0x000E, 4)),
    (19, 2 << 8, 0xB000, (0, 0x000E, 8), (0, 0x003F, 0)),
    (20, 2 << 8, 0xB040, (0, 0x000E, 8), (0, 0x003F, 0)),
    (21, (2 << 8) | H_TI, 0x9012, (0, 0x000E, 8), (0, 0, 0)),
    (16, 1 << 8, 0x9030, (0, 0x000F, 8), (0, 0, 0)),
    (16, (1 << 8) | H_IA, 0x9050, (0, 0x000F, 8), (0, 0, 0)),
    (17, 1 << 8, 0x9000, (0, 0x000F, 8), (2, 0x000E, 4)),
    (18, (1 << 8) | H_TI, 0x9008, (0, 0x000F, 8), (2, 0x000E, 4)),
    (19, 1 << 8, 0xD000, (0, 0x000F, 8), (0, 0x003F, 0)),
    (20, 1 << 8, 0xD040, (0, 0x000F, 8), (0, 0x003F, 0)),
    (21, (1 << 8) | H_TI, 0x9010, (0, 0x000F, 8), (0, 0, 0)),
    (16, 4 << 8, 0x9034, (0, 0x000C, 8), (0, 0, 0)),
    (16, (4 << 8) | H_IA, 0x9054, (0, 0x000C, 8), (0, 0, 0)),
    (16, 8 << 8, 0x9036, (0, 0x0008, 8), (0, 0, 0)),
    (16, (8 << 8) | H_IA, 0x9056, (0, 0x0008, 8), (0, 0, 0)),
    (16, (2 << 8) | H_ST, 0x9033, (0, 0x000E, 8), (0, 0, 0)),
    (16, (2 << 8) | H_IA | H_ST, 0x9053, (0, 0x000E, 8), (0, 0, 0)),
    (17, (2 << 8) | H_ST, 0x9003, (0, 0x000E, 8), (2, 0x000E, 4)),
    (18, (2 << 8) | H_TI | H_ST, 0xA009, (0, 0x000E, 8), (2, 0x000E, 4)),
    (19, (2 << 8) | H_ST, 0xB080, (0, 0x000E, 8), (0, 0x003F, 0)),
    (20, (2 << 8) | H_ST, 0xB0C0, (0, 0x000E, 8), (0, 0x003F, 0)),
    (21, (2 << 8) | H_TI | H_ST, 0x9013, (0, 0x000E, 8), (0, 0, 0)),
    (16, (1 << 8) | H_ST, 0x9031, (0, 0x000F, 8), (0, 0, 0)),
    (16, (1 << 8) | H_IA | H_ST, 0x9051, (0, 0x000F, 8), (0, 0, 0)),
    (17, (1 << 8) | H_ST, 0x9001, (0, 0x000F, 8), (2, 0x000E, 4)),
    (18, (1 << 8) | H_TI | H_ST, 0x9009, (0, 0x000F, 8), (2, 0x000E, 4)),
    (19, (1 << 8) | H_ST, 0xD080, (0, 0x000F, 8), (0, 0x003F, 0)),
    (20, (1 << 8) | H_ST, 0xD0C0, (0, 0x000F, 8), (0, 0x003F, 0)),
    (21, (1 << 8) | H_TI | H_ST, 0x9011, (0, 0x000F, 8), (0, 0, 0)),
    (16, (4 << 8) | H_ST, 0x9035, (0, 0x000C, 8), (0, 0, 0)),
    (16, (4 << 8) | H_IA | H_ST, 0x9055, (0, 0x000C, 8), (0, 0, 0)),
    (16, (8 << 8) | H_ST, 0x9037, (0, 0x0008, 8), (0, 0, 0)),
    (16, (8 << 8) | H_IA | H_ST, 0x9057, (0, 0x0008, 8), (0, 0, 0)),
    // Control register access
    (22, 0, 0xE100, (0, 0x00FF, 0), (0, 0, 0)),
    (23, 1 << 8, 0xA00F, (0, 0, 0), (1, 0x000F, 4)),
    (23, 2 << 8, 0xA00D, (0, 0, 0), (2, 0x000E, 8)),
    (23, 3 << 8, 0xA00C, (0, 0, 0), (1, 0x000F, 4)),
    (23, H_WB | (4 << 8), 0xA005, (2, 0x000E, 8), (0, 0, 0)),
    (23, H_WB | (5 << 8), 0xA01A, (2, 0x000E, 8), (0, 0, 0)),
    (23, 6 << 8, 0xA00B, (0, 0, 0), (1, 0x000F, 4)),
    (23, 7 << 8, 0xE900, (0, 0, 0), (0, 0x00FF, 0)),
    (23, H_WB | (8 << 8), 0xA007, (1, 0x000F, 8), (0, 0, 0)),
    (23, H_WB | (9 << 8), 0xA004, (1, 0x000F, 8), (0, 0, 0)),
    (23, H_WB | (10 << 8), 0xA003, (1, 0x000F, 8), (0, 0, 0)),
    (23, 11 << 8, 0xA10A, (0, 0, 0), (2, 0x000E, 4)),
    // PUSH/POP
    (24, 0, 0xF05E, (0, 0, 0), (2, 0x000E, 8)),
    (24, 0, 0xF07E, (0, 0, 0), (8, 0x0008, 8)),
    (24, 0, 0xF04E, (0, 0, 0), (1, 0x000F, 8)),
    (24, 0, 0xF06E, (0, 0, 0), (4, 0x000C, 8)),
    (25, 0, 0xF0CE, (0, 0, 0), (0, 0x000F, 8)),
    (26, H_WB, 0xF01E, (2, 0x000E, 8), (0, 0, 0)),
    (26, H_WB, 0xF03E, (8, 0x0008, 8), (0, 0, 0)),
    (26, H_WB, 0xF00E, (1, 0x000F, 8), (0, 0, 0)),
    (26, H_WB, 0xF02E, (4, 0x000C, 8), (0, 0, 0)),
    (27, 0, 0xF08E, (0, 0x000F, 8), (0, 0, 0)),
    // Coprocessor data transfer
    (28, 0, 0xA00E, (0, 0x000F, 8), (0, 0x000F, 4)),
    (29, 2 << 8, 0xF02D, (0, 0, 0), (0, 0x000E, 8)),
    (29, (2 << 8) | H_IA, 0xF03D, (0, 0, 0), (0, 0x000E, 8)),
    (29, 1 << 8, 0xF00D, (0, 0, 0), (0, 0x000F, 8)),
    (29, (1 << 8) | H_IA, 0xF01D, (0, 0, 0), (0, 0x000F, 8)),
    (29, 4 << 8, 0xF04D, (0, 0, 0), (0, 0x000C, 8)),
    (29, (4 << 8) | H_IA, 0xF05D, (0, 0, 0), (0, 0x000C, 8)),
    (29, 8 << 8, 0xF06D, (0, 0, 0), (0, 0x0008, 8)),
    (29, (8 << 8) | H_IA, 0xF07D, (0, 0, 0), (0, 0x0008, 8)),
    (28, H_ST, 0xA006, (0, 0x000F, 8), (0, 0x000F, 4)),
    (29, (2 << 8) | H_ST, 0xF0AD, (0, 0x000E, 8), (0, 0, 0)),
    (29, (2 << 8) | H_IA | H_ST, 0xF0BD, (0, 0x000E, 8), (0, 0, 0)),
    (29, (1 << 8) | H_ST, 0xF08D, (0, 0x000F, 8), (0, 0, 0)),
    (29, (1 << 8) | H_IA | H_ST, 0xF09D, (0, 0x000F, 8), (0, 0, 0)),
    (29, (4 << 8) | H_ST, 0xF0CD, (0, 0x000C, 8), (0, 0, 0)),
    (29, (4 << 8) | H_IA | H_ST, 0xF0DD, (0, 0x000C, 8), (0, 0, 0)),
    (29, (8 << 8) | H_ST, 0xF0ED, (0, 0x0008, 8), (0, 0, 0)),
    (29, (8 << 8) | H_IA | H_ST, 0xF0FD, (0, 0x0008, 8), (0, 0, 0)),
    // EA register data transfer
    (30, 0, 0xF00A, (0, 0, 0), (2, 0x000E, 4)),
    (30, H_TI, 0xF00B, (0, 0, 0), (2, 0x000E, 4)),
    (30, H_TI, 0xF00C, (0, 0, 0), (0, 0, 0)),
    // ALU
    (31, H_WB, 0x801F, (1, 0x000F, 8), (0, 0, 0)),
    (32, H_WB, 0x803F, (1, 0x000F, 8), (0, 0, 0)),
    (33, H_WB, 0x805F, (1, 0x000F, 8), (0, 0, 0)),
    // Bit access
    (34, 0, 0xA000, (0, 0x000F, 8), (0, 0x0007, 4)),
    (34, H_TI, 0xA080, (0, 0, 0), (0, 0x0007, 4)),
    (34, 0, 0xA002, (0, 0x000F, 8), (0, 0x0007, 4)),
    (34, H_TI, 0xA082, (0, 0, 0), (0, 0x0007, 4)),
    (34, 0, 0xA001, (0, 0x000F, 8), (0, 0x0007, 4)),
    (34, H_TI, 0xA081, (0, 0, 0), (0, 0x0007, 4)),
    // PSW access
    (35, 0, 0xED08, (0, 0, 0), (0, 0, 0)),
    (36, 0, 0xEBF7, (0, 0, 0), (0, 0, 0)),
    (35, 0, 0xED80, (0, 0, 0), (0, 0, 0)),
    (36, 0, 0xEB7F, (0, 0, 0), (0, 0, 0)),
    (37, 0, 0xFECF, (0, 0, 0), (0, 0, 0)),
    // Conditional relative branches
    (38, 0, 0xC000, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC100, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC200, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC300, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC400, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC500, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC600, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC700, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC800, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xC900, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xCA00, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xCB00, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xCC00, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xCD00, (0, 0x00FF, 0), (0, 0, 0)),
    (38, 0, 0xCE00, (0, 0x00FF, 0), (0, 0, 0)),
    // Sign extension
    (39, 0, 0x810F, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x832F, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x854F, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x876F, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x898F, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x8BAF, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x8DCF, (0, 0, 0), (0, 0, 0)),
    (39, 0, 0x8FEF, (0, 0, 0), (0, 0, 0)),
    // Software interrupts
    (40, 0, 0xE500, (0, 0x00FF, 0), (0, 0, 0)),
    (41, 0, 0xFFFF, (0, 0, 0), (0, 0, 0)),
    // Branch
    (42, H_TI, 0xF000, (0, 0x000F, 4), (0, 0x000F, 8)),
    (42, 0, 0xF002, (0, 0x000F, 8), (2, 0x000E, 4)),
    (43, H_TI, 0xF001, (0, 0x000F, 4), (0, 0x000F, 8)),
    (43, 0, 0xF003, (0, 0x000F, 8), (2, 0x000E, 4)),
    // Multiply / divide
    (44, H_WB, 0xF004, (2, 0x000E, 8), (1, 0x000F, 4)),
    (45, H_WB, 0xF009, (2, 0x000E, 8), (1, 0x000F, 4)),
    // Miscellaneous
    (46, 0, 0xFE2F, (0, 0, 0), (0, 0, 0)),
    (47, 0, 0xFE3F, (0, 0, 0), (0, 0, 0)),
    (48, 0, 0xFE1F, (0, 0, 0), (0, 0, 0)),
    (49, 0, 0xFE0F, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFE8F, (0, 0, 0), (0, 0, 0)),
    (51, H_DS, 0xFE9F, (0, 0, 0), (0, 0, 0)),
    (51, H_DS | H_DW, 0xE300, (0, 0x00FF, 0), (0, 0, 0)),
    (51, H_DS | H_DW, 0x900F, (1, 0x000F, 4), (0, 0, 0)),
    // Undocumented
    (52, 0, 0xFE6F, (0, 0, 0), (0, 0, 0)),
    (52, 0, 0xFE7F, (0, 0, 0), (0, 0, 0)),
    (53, 0, 0xFEFF, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFE4F, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFE5F, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFE8F, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFEAF, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFEBF, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFEDF, (0, 0, 0), (0, 0, 0)),
    (50, 0, 0xFEEF, (0, 0, 0), (0, 0, 0)),
];

/// 0..54 handler indices
pub const HANDLER_NONE: u16 = 0xFFFF;

pub fn build_dispatch() -> Vec<u16> {
    let mut dispatch = vec![HANDLER_NONE; 0x10000];
    let mut permutation = vec![0u16; 0x10000];
    for (idx, src) in OPCODE_SOURCES.iter().enumerate() {
        let mut varying_bits: u32 = 0;
        varying_bits |= (src.3.1 as u32) << src.3.2;
        varying_bits |= (src.4.1 as u32) << src.4.2;
        let mut count = 1usize;
        permutation[0] = src.2;
        let mut checkbit: u16 = 0x8000;
        while checkbit != 0 {
            if (varying_bits & checkbit as u32) != 0 {
                for px in 0..count {
                    permutation[px + count] = permutation[px] | checkbit;
                }
                count <<= 1;
            }
            checkbit >>= 1;
        }
        for px in 0..count {
            let op = permutation[px] as usize;
            if dispatch[op] == HANDLER_NONE {
                dispatch[op] = idx as u16;
            }
        }
    }
    dispatch
}

pub struct Cpu {
    pub r: [u8; 16],
    pub cr: [u8; 16],
    pub pc: u16,
    pub elr: [u16; 4],
    pub csr: u16,
    pub ecsr: [u16; 4],
    pub epsw: [u8; 4],
    pub sp: u16,
    pub ea: u16,
    pub dsr: u8,
    pub fetch_addition: u16,
    pub run: bool,
    pub dsr_mask: u8,
    pub csr_mask: u16,
    pub nx_u16: bool,
    pub mm_large: bool,
    dispatch: Vec<u16>,
    // impl temporaries
    pub flags_changed: u8,
    pub flags_out: u8,
    pub flags_in: u8,
    pub shift_buffer: u8,
    pub opcode: u16,
    pub long_imm: u16,
    pub operands: [Operand; 2],
    pub hint: u16,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            r: [0; 16],
            cr: [0; 16],
            pc: 0,
            elr: [0; 4],
            csr: 0,
            ecsr: [0; 4],
            epsw: [0; 4],
            sp: 0,
            ea: 0,
            dsr: 0,
            fetch_addition: 2,
            run: true,
            dsr_mask: 0x1F,
            csr_mask: 0x000F,
            nx_u16: true,
            mm_large: true,
            dispatch: build_dispatch(),
            flags_changed: 0,
            flags_out: 0,
            flags_in: 0,
            shift_buffer: 0,
            opcode: 0,
            long_imm: 0,
            operands: [Operand { value: 0, register_index: 0, register_size: 0 }; 2],
            hint: 0,
        }
    }

    #[inline]
    pub fn psw(&self) -> u8 {
        self.epsw[0]
    }

    #[inline]
    pub fn set_psw(&mut self, v: u8) {
        self.epsw[0] = v;
    }

    #[inline]
    pub fn lr(&self) -> u16 {
        self.elr[0]
    }

    #[inline]
    pub fn lcsr(&self) -> u16 {
        self.ecsr[0]
    }

    pub fn fetch(&mut self, mmu: &mut Mmu) -> u16 {
        if self.csr & !self.csr_mask != 0 {
            self.csr &= self.csr_mask;
        }
        if self.pc & 1 != 0 {
            self.pc &= !1;
        }
        let opcode = mmu.read_code(((self.csr as u32) << 16) | self.pc as u32);
        self.pc = self.pc.wrapping_add(self.fetch_addition);
        self.fetch_addition = 2;
        opcode
    }

    pub fn next(&mut self, mmu: &mut Mmu) {
        if !self.run {
            return;
        }
        self.dsr = 0;
        mmu.ints.is_mi_blocked = false;
        let pc_before = ((self.csr as u32) << 16) | self.pc as u32;

        loop {
            self.opcode = self.fetch(mmu);
            let handler_index = self.dispatch[self.opcode as usize];
            if handler_index == HANDLER_NONE {
                eprintln!(
                    "[CPU][Warn] Unknown Inst at 0x{:06x}: {:04x}",
                    pc_before, self.opcode
                );
                continue;
            }
            let src = OPCODE_SOURCES[handler_index as usize];
            self.long_imm = 0;
            if src.1 & H_TI != 0 {
                self.long_imm = self.fetch(mmu);
            }
            for ix in 0..2usize {
                let (rs, mask, shift) = if ix == 0 { src.3 } else { src.4 };
                self.operands[ix].value = (((self.opcode >> shift) & mask) & 0xFFFF) as u64;
                self.operands[ix].register_index = self.operands[ix].value as usize;
                self.operands[ix].register_size = rs;
                if rs != 0 {
                    self.operands[ix].value = 0;
                    for bx in 0..rs {
                        let idx = self.operands[ix].register_index + bx;
                        self.operands[ix].value |= (self.r[idx] as u64) << (bx * 8);
                    }
                }
            }
            self.hint = src.1;
            self.flags_changed = 0;
            self.flags_in = self.psw();
            self.flags_out = PSW_Z;
            run_handler(self, mmu, src.0 as usize);
            self.set_psw(
                (self.psw() & !self.flags_changed) | (self.flags_out & self.flags_changed),
            );
            if src.1 & H_WB != 0 && self.operands[0].register_size != 0 {
                for bx in 0..self.operands[0].register_size {
                    let idx = self.operands[0].register_index + bx;
                    self.r[idx] = (self.operands[0].value >> (bx * 8)) as u8;
                }
            }
            if src.1 & H_DS == 0 {
                break;
            }
        }
        self.dsr = 0;
    }

    pub fn reset(&mut self, mmu: &mut Mmu) {
        self.sp = mmu.read_code(0);
        self.dsr = 0;
        self.set_psw(0);
        self.fetch_addition = 2;
        self.run = true;
    }

    /// exception_level 0..3, index = vector table word index
    pub fn raise(&mut self, exception_level: usize, index: usize, mmu: &mut Mmu) {
        self.epsw[exception_level] = self.psw();
        self.elr[exception_level] = self.pc;
        self.ecsr[exception_level] = self.csr;
        if exception_level == 1 {
            self.set_psw(self.psw() & !PSW_MIE);
        }
        self.set_psw((self.psw() & !PSW_ELEVEL) | exception_level as u8);
        self.csr = 0;
        self.pc = mmu.read_code((index * 2) as u32);
    }

    #[inline]
    pub fn exception_level(&self) -> usize {
        (self.psw() & PSW_ELEVEL) as usize
    }

    #[inline]
    pub fn mie(&self) -> bool {
        self.psw() & PSW_MIE != 0
    }

    // ---------------- helper: memory access through dsr ----------------
    #[inline]
    fn read_d(&mut self, mmu: &mut Mmu, offset: u16) -> u8 {
        mmu.read_data(((self.dsr as u32) << 16) | offset as u32)
    }

    #[inline]
    fn write_d(&mut self, mmu: &mut Mmu, offset: u16, data: u8) {
        mmu.write_data(((self.dsr as u32) << 16) | offset as u32, data);
    }

    // ---------------- instruction implementations ----------------
    fn op_add(&mut self, _mmu: &mut Mmu) {
        self.flags_in &= !PSW_C;
        self.flags_in |= PSW_Z;
        self.op_addc(_mmu);
    }

    fn op_add16(&mut self, _mmu: &mut Mmu) {
        if self.hint & H_IE != 0 {
            self.operands[1].value |= if self.operands[1].value & 0x40 != 0 { 0xFF80 } else { 0 };
        }
        self.flags_in &= !PSW_C;
        let op_high_0 = (self.operands[0].value >> 8) as u8;
        let op_high_1 = (self.operands[1].value >> 8) as u8;
        self.add8();
        self.zs_check();
        self.flags_in = (self.flags_in & !PSW_C) | (self.flags_out & PSW_C);
        let op_low_0 = self.operands[0].value as u8;
        self.operands[0].value = op_high_0 as u64;
        self.operands[1].value = op_high_1 as u64;
        self.add8();
        self.zs_check();
        self.operands[0].value = (self.operands[0].value << 8) | op_low_0 as u64;
    }

    fn op_addc(&mut self, _mmu: &mut Mmu) {
        self.add8();
        if self.flags_in & PSW_Z == 0 {
            self.flags_out &= !PSW_Z;
        }
        self.zs_check();
    }

    fn op_and(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value &= self.operands[1].value & 0xFF;
        self.zs_check();
    }

    fn op_mov16(&mut self, _mmu: &mut Mmu) {
        if self.hint & H_IE != 0 {
            self.operands[1].value |= if self.operands[1].value & 0x40 != 0 { 0xFF80 } else { 0 };
        }
        self.operands[0].value = self.operands[1].value & 0xFF;
        self.zs_check();
        let op_low_0 = self.operands[0].value as u8;
        self.operands[0].value = (self.operands[1].value >> 8) & 0xFF;
        self.zs_check();
        self.operands[0].value = (self.operands[0].value << 8) | op_low_0 as u64;
    }

    fn op_mov(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value = self.operands[1].value & 0xFF;
        self.zs_check();
    }

    fn op_or(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value |= self.operands[1].value & 0xFF;
        self.zs_check();
    }

    fn op_xor(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value ^= self.operands[1].value & 0xFF;
        self.zs_check();
    }

    fn op_cmp16(&mut self, _mmu: &mut Mmu) {
        self.flags_in &= !PSW_C;
        let op_high_0 = (self.operands[0].value >> 8) as u8;
        let op_high_1 = (self.operands[1].value >> 8) as u8;
        self.operands[0].value ^= 0xFF;
        self.add8();
        self.operands[0].value ^= 0xFF;
        self.zs_check();
        self.flags_in = (self.flags_in & !PSW_C) | (self.flags_out & PSW_C);
        let op_low_0 = self.operands[0].value as u8;
        self.operands[0].value = op_high_0 as u64;
        self.operands[1].value = op_high_1 as u64;
        self.operands[0].value ^= 0xFF;
        self.add8();
        self.operands[0].value ^= 0xFF;
        self.zs_check();
        self.operands[0].value = (self.operands[0].value << 8) | op_low_0 as u64;
    }

    fn op_sub(&mut self, mmu: &mut Mmu) {
        self.flags_in &= !PSW_C;
        self.flags_in |= PSW_Z;
        self.op_subc(mmu);
    }

    fn op_subc(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value ^= 0xFF;
        self.add8();
        self.operands[0].value ^= 0xFF;
        if self.flags_in & PSW_Z == 0 {
            self.flags_out &= !PSW_Z;
        }
        self.zs_check();
    }

    fn op_sll(&mut self, _mmu: &mut Mmu) {
        self.shift_buffer = 0;
        self.shift_left8();
    }

    fn op_sllc(&mut self, _mmu: &mut Mmu) {
        let ext = (self.operands[0].register_index - 1) & 15;
        self.shift_buffer = self.r[ext];
        self.shift_left8();
    }

    fn op_sra(&mut self, _mmu: &mut Mmu) {
        let shift_by = (self.operands[1].value & 7) as usize;
        let msb = self.operands[0].value & 0x80;
        self.shift_buffer = 0;
        self.shift_right8();
        if msb != 0 {
            self.operands[0].value |= (0xFF_u64 >> shift_by) ^ 0xFF;
        }
    }

    fn op_srl(&mut self, _mmu: &mut Mmu) {
        self.shift_buffer = 0;
        self.shift_right8();
    }

    fn op_srlc(&mut self, _mmu: &mut Mmu) {
        let ext = (self.operands[0].register_index + 1) & 15;
        self.shift_buffer = self.r[ext];
        self.shift_right8();
    }

    fn op_ls_ea(&mut self, mmu: &mut Mmu) {
        let length = (self.hint >> 8) as usize;
        self.load_store(self.ea, length, mmu);
    }

    fn op_ls_r(&mut self, mmu: &mut Mmu) {
        let length = (self.hint >> 8) as usize;
        self.load_store((self.operands[1].value & 0xFFFF) as u16, length, mmu);
    }

    fn op_ls_i_r(&mut self, mmu: &mut Mmu) {
        let length = (self.hint >> 8) as usize;
        let value = ((self.operands[1].value as u16).wrapping_add(self.long_imm)) as u16;
        self.load_store(value, length, mmu);
    }

    fn op_ls_bp(&mut self, mmu: &mut Mmu) {
        self.operands[1].value |= if self.operands[1].value & 0x20 != 0 { 0xFFC0 } else { 0 };
        let base = (self.r[12] as u16) | ((self.r[13] as u16) << 8);
        let length = (self.hint >> 8) as usize;
        let value = (self.operands[1].value as u16).wrapping_add(base);
        self.load_store(value, length, mmu);
    }

    fn op_ls_fp(&mut self, mmu: &mut Mmu) {
        self.operands[1].value |= if self.operands[1].value & 0x20 != 0 { 0xFFC0 } else { 0 };
        let base = (self.r[14] as u16) | ((self.r[15] as u16) << 8);
        let length = (self.hint >> 8) as usize;
        let value = (self.operands[1].value as u16).wrapping_add(base);
        self.load_store(value, length, mmu);
    }

    fn op_ls_i(&mut self, mmu: &mut Mmu) {
        let length = (self.hint >> 8) as usize;
        self.load_store(self.long_imm, length, mmu);
    }

    fn load_store(&mut self, mut offset: u16, length: usize, mmu: &mut Mmu) {
        if length % 2 == 0 {
            offset &= !1;
        }
        let reg_base = self.operands[0].value as usize;
        if self.hint & H_ST != 0 {
            for ix in (0..length).rev() {
                self.write_d(mmu, offset.wrapping_add(ix as u16), self.r[reg_base + ix]);
            }
        } else {
            for ix in 0..length {
                self.operands[0].value =
                    self.read_d(mmu, offset.wrapping_add(ix as u16)) as u64;
                self.zs_check();
                self.r[reg_base + ix] = self.operands[0].value as u8;
            }
        }
        if self.hint & H_IA != 0 {
            self.bump_ea(length);
        }
    }

    fn op_addsp(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value |= if self.operands[0].value & 0x80 != 0 { 0xFF00 } else { 0 };
        self.sp = self.sp.wrapping_add(self.operands[0].value as u16);
        if self.nx_u16 {
            self.sp &= 0xFFFE;
        }
    }

    fn op_ctrl(&mut self, _mmu: &mut Mmu) {
        let level = (self.psw() & PSW_ELEVEL) as usize;
        match self.hint >> 8 {
            1 => {
                self.ecsr[level] = self.operands[1].value as u16;
            }
            2 => {
                self.elr[level] = self.operands[1].value as u16;
            }
            3 => {
                if self.psw() & PSW_ELEVEL != 0 {
                    self.epsw[level] = self.operands[1].value as u8;
                }
            }
            4 => {
                self.operands[0].value = self.elr[level] as u64;
            }
            5 => {
                self.operands[0].value = self.sp as u64;
            }
            6 | 7 => {
                self.set_psw(self.operands[1].value as u8);
            }
            8 => {
                self.operands[0].value = self.ecsr[level] as u64;
            }
            9 => {
                if self.psw() & PSW_ELEVEL != 0 {
                    self.operands[0].value = self.epsw[level] as u64;
                } else if self.nx_u16 {
                    self.operands[0].value = 0xFF;
                }
            }
            10 => {
                self.operands[0].value = self.psw() as u64;
            }
            11 => {
                self.sp = self.operands[1].value as u16;
                if self.nx_u16 {
                    self.sp &= 0xFFFE;
                }
            }
            _ => {}
        }
    }

    fn op_lea(&mut self, _mmu: &mut Mmu) {
        self.ea = 0;
        if self.operands[1].register_size != 0 {
            self.ea = self.ea.wrapping_add(self.operands[1].value as u16);
        }
        if self.hint & H_TI != 0 {
            self.ea = self.ea.wrapping_add(self.long_imm);
        }
    }

    fn op_cr_r(&mut self, _mmu: &mut Mmu) {
        let op0 = ((self.opcode >> 8) & 0x000F) as usize;
        let op1 = ((self.opcode >> 4) & 0x000F) as usize;
        if self.hint & H_ST != 0 {
            self.r[op0] = self.cr[op1];
        } else {
            self.cr[op0] = self.r[op1];
        }
    }

    fn op_cr_ea(&mut self, mmu: &mut Mmu) {
        let op0 = ((self.opcode >> 8) & 0x000F) as usize;
        let size = (self.hint >> 8) as usize;
        if self.hint & H_ST != 0 {
            for ix in (0..size).rev() {
                let addr = self.ea.wrapping_add(ix as u16);
                self.write_d(mmu, addr, self.cr[op0 + ix]);
            }
        } else {
            for ix in 0..size {
                let addr = self.ea.wrapping_add(ix as u16);
                self.cr[op0 + ix] = self.read_d(mmu, addr);
            }
        }
        if self.hint & H_IA != 0 {
            self.bump_ea(size);
        }
    }

    fn bump_ea(&mut self, value_size: usize) {
        self.ea = self.ea.wrapping_add(value_size as u16);
        if value_size != 1 {
            self.ea &= !1;
        }
    }

    fn op_daa(&mut self, mmu: &mut Mmu) {
        self.operands[1].value = 0;
        if self.operands[0].value & 0x0F > 0x09 || self.flags_in & PSW_HC != 0 {
            self.operands[1].value |= 0x06;
        }
        if self.operands[0].value & 0xF0 > 0x90 || self.flags_in & PSW_C != 0 {
            self.operands[1].value |= 0x60;
        }
        if self.operands[0].value & 0xF0 == 0x90
            && self.operands[0].value & 0x0F > 0x09
            && self.flags_in & PSW_HC == 0
        {
            self.operands[1].value |= 0x60;
        }
        let flags_in_backup = self.flags_in;
        self.op_add(mmu);
        self.flags_out |= flags_in_backup & PSW_C;
        self.flags_changed &= !PSW_OV;
    }

    fn op_das(&mut self, mmu: &mut Mmu) {
        self.operands[1].value = 0;
        if self.operands[0].value & 0x0F > 0x09 || self.flags_in & PSW_HC != 0 {
            self.operands[1].value |= 0x06;
        }
        if self.operands[0].value & 0xF0 > 0x90 || self.flags_in & PSW_C != 0 {
            self.operands[1].value |= 0x60;
        }
        let flags_in_backup = self.flags_in;
        self.op_sub(mmu);
        self.flags_out |= flags_in_backup & PSW_C;
        self.flags_changed &= !PSW_OV;
    }

    fn op_neg(&mut self, mmu: &mut Mmu) {
        self.operands[1].value = self.operands[0].value;
        self.operands[0].value = 0;
        self.op_sub(mmu);
    }

    fn op_bitmod(&mut self, mmu: &mut Mmu) {
        let bit_in = 1u64 << self.operands[1].value;
        let src_index: usize;
        if self.hint & H_TI != 0 {
            src_index = self.long_imm as usize;
            self.operands[0].value = self.read_d(mmu, self.long_imm) as u64;
        } else {
            src_index = self.operands[0].value as usize;
            self.operands[0].value = self.r[src_index] as u64;
        }
        self.flags_changed |= PSW_Z;
        self.flags_out = if self.operands[0].value & bit_in != 0 { 0 } else { PSW_Z };
        match self.opcode & 0x000F {
            0 => self.operands[0].value |= bit_in,
            2 => self.operands[0].value &= !bit_in,
            _ => {}
        }
        if self.opcode & 0x000F != 1 {
            if self.hint & H_TI != 0 {
                self.write_d(mmu, self.long_imm, self.operands[0].value as u8);
            } else {
                self.r[src_index] = self.operands[0].value as u8;
            }
        }
    }

    fn op_extbw(&mut self, _mmu: &mut Mmu) {
        let index = ((self.opcode & 0x00E0) >> 4) as usize;
        self.operands[0].value = if self.r[index] & 0x80 != 0 { 0xFF } else { 0x00 };
        self.r[index + 1] = self.operands[0].value as u8;
        self.zs_check();
    }

    fn op_mul(&mut self, _mmu: &mut Mmu) {
        self.operands[0].value &= 0xFF;
        self.operands[0].value *= self.operands[1].value;
        self.flags_changed |= PSW_Z;
        self.flags_out = if self.operands[0].value != 0 { 0 } else { PSW_Z };
    }

    fn op_div(&mut self, _mmu: &mut Mmu) {
        self.flags_changed |= PSW_Z | PSW_C;
        if self.operands[1].value == 0 {
            self.flags_out |= PSW_C;
            return;
        }
        let quotient = self.operands[0].value / self.operands[1].value;
        let remainder = self.operands[0].value % self.operands[1].value;
        self.operands[0].value = quotient;
        if quotient != 0 {
            self.flags_out &= !PSW_Z;
        }
        let remainder_reg = ((self.opcode >> 4) & 0x000F) as usize;
        self.r[remainder_reg] = (remainder & 0xFF) as u8;
    }

    fn op_inc_ea(&mut self, mmu: &mut Mmu) {
        self.operands[0].value = self.read_d(mmu, self.ea) as u64;
        self.operands[1].value = 1;
        self.op_add(mmu);
        self.flags_changed &= !PSW_C;
        self.write_d(mmu, self.ea, self.operands[0].value as u8);
    }

    fn op_dec_ea(&mut self, mmu: &mut Mmu) {
        self.operands[0].value = self.read_d(mmu, self.ea) as u64;
        self.operands[1].value = 1;
        self.op_sub(mmu);
        self.flags_changed &= !PSW_C;
        self.write_d(mmu, self.ea, self.operands[0].value as u8);
    }

    fn op_push(&mut self, mmu: &mut Mmu) {
        let mut push_size = self.operands[1].register_size;
        if push_size == 1 {
            push_size = 2;
        }
        self.sp = self.sp.wrapping_sub(push_size as u16);
        for ix in (0..self.operands[1].register_size).rev() {
            let data = (self.operands[1].value >> (8 * ix)) as u8;
            self.write_d(mmu, self.sp.wrapping_add(ix as u16), data);
        }
    }

    fn op_pushl(&mut self, mmu: &mut Mmu) {
        let level = (self.psw() & PSW_ELEVEL) as usize;
        if self.operands[1].value & 2 != 0 {
            if self.mm_large {
                self.push16(mmu, self.ecsr[level]);
            }
            self.push16(mmu, self.elr[level]);
        }
        if self.operands[1].value & 4 != 0 {
            self.push16(mmu, self.epsw[level] as u16);
        }
        if self.operands[1].value & 8 != 0 {
            if self.mm_large {
                self.push16(mmu, self.lcsr());
            }
            self.push16(mmu, self.lr());
        }
        if self.operands[1].value & 1 != 0 {
            self.push16(mmu, self.ea);
        }
    }

    fn op_pop(&mut self, mmu: &mut Mmu) {
        let mut pop_size = self.operands[0].register_size;
        if pop_size == 1 {
            pop_size = 2;
        }
        self.operands[0].value = 0;
        for ix in 0..self.operands[0].register_size {
            let data = self.read_d(mmu, self.sp.wrapping_add(ix as u16)) as u64;
            self.operands[0].value |= data << (8 * ix);
        }
        self.sp = self.sp.wrapping_add(pop_size as u16);
    }

    fn op_popl(&mut self, mmu: &mut Mmu) {
        if self.operands[0].value & 1 != 0 {
            self.ea = self.pop16(mmu);
        }
        if self.operands[0].value & 8 != 0 {
            self.elr[0] = self.pop16(mmu);
            if self.mm_large {
                self.ecsr[0] = self.pop16(mmu) & 0x000F;
            }
        }
        if self.operands[0].value & 4 != 0 {
            let psw = self.pop16(mmu) as u8;
            self.set_psw(psw);
        }
        if self.operands[0].value & 2 != 0 {
            self.pc = self.pop16(mmu);
            if self.mm_large {
                self.csr = self.pop16(mmu) & 0x000F;
            }
        }
    }

    fn push16(&mut self, mmu: &mut Mmu, data: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.write_d(mmu, self.sp.wrapping_add(1), (data >> 8) as u8);
        self.write_d(mmu, self.sp, (data & 0xFF) as u8);
    }

    fn pop16(&mut self, mmu: &mut Mmu) -> u16 {
        let lo = self.read_d(mmu, self.sp);
        let hi = self.read_d(mmu, self.sp.wrapping_add(1));
        self.sp = self.sp.wrapping_add(2);
        lo as u16 | ((hi as u16) << 8)
    }

    fn op_psw_or(&mut self, mmu: &mut Mmu) {
        self.set_psw(self.psw() | (self.opcode & 0xFF) as u8);
        if self.opcode & 0x0008 != 0 {
            mmu.ints.is_mi_blocked = true;
        }
    }

    fn op_psw_and(&mut self, _mmu: &mut Mmu) {
        self.set_psw(self.psw() & (self.opcode & 0xFF) as u8);
    }

    fn op_cplc(&mut self, _mmu: &mut Mmu) {
        self.set_psw(self.psw() ^ PSW_C);
    }

    fn op_bc(&mut self, _mmu: &mut Mmu) {
        let c = self.flags_in & PSW_C != 0;
        let z = self.flags_in & PSW_Z != 0;
        let s = self.flags_in & PSW_S != 0;
        let ov = self.flags_in & PSW_OV != 0;
        let le = z || c;
        let lts = ov ^ s;
        let les = lts || z;
        let branch = match (self.opcode >> 8) & 0x000F {
            0 => !c,
            1 => c,
            2 => !le,
            3 => le,
            4 => !lts,
            5 => lts,
            6 => !les,
            7 => les,
            8 => !z,
            9 => z,
            10 => !ov,
            11 => ov,
            12 => !s,
            13 => s,
            14 => true,
            _ => false,
        };
        if branch {
            self.operands[0].value |= if self.operands[0].value & 0x80 != 0 { 0x7F00 } else { 0 };
            let delta = (self.operands[0].value as u16).wrapping_shl(1);
            self.pc = self.pc.wrapping_add(delta);
        }
    }

    fn op_swi(&mut self, mmu: &mut Mmu) {
        mmu.raise_software((self.operands[0].value & 0x3F) as usize);
    }

    fn op_brk(&mut self, mmu: &mut Mmu) {
        mmu.break_int();
    }

    fn op_iceswi(&mut self, mmu: &mut Mmu) {
        mmu.raise_emulator();
    }

    fn op_rtice(&mut self, mmu: &mut Mmu) {
        self.op_rti(mmu);
    }

    fn op_b(&mut self, _mmu: &mut Mmu) {
        if self.hint & H_TI != 0 {
            self.csr = self.operands[1].value as u16;
            self.pc = self.long_imm;
        } else {
            self.pc = self.operands[1].value as u16;
        }
    }

    fn op_bl(&mut self, mmu: &mut Mmu) {
        self.elr[0] = self.pc;
        self.ecsr[0] = self.csr;
        self.op_b(mmu);
    }

    fn op_rt(&mut self, _mmu: &mut Mmu) {
        self.csr = self.lcsr();
        self.pc = self.lr();
    }

    fn op_rti(&mut self, _mmu: &mut Mmu) {
        let level = (self.psw() & PSW_ELEVEL) as usize;
        self.csr = self.ecsr[level];
        self.pc = self.elr[level];
        self.set_psw(self.epsw[level]);
        self.epsw[0] = self.epsw[level];
        // note: psw is the epsw[0] alias; after set_psw the level reg is
        // unchanged, so nothing else to do
    }

    fn op_nop(&mut self, _mmu: &mut Mmu) {}

    fn op_dsr(&mut self, mmu: &mut Mmu) {
        if self.hint & H_DW != 0 {
            mmu.impl_last_dsr = self.operands[0].value as u8;
        }
        mmu.impl_last_dsr &= self.dsr_mask;
        self.dsr = mmu.impl_last_dsr;
    }

    // ---------------- shared helpers ----------------
    fn add8(&mut self) {
        let op8_0 = self.operands[0].value as u8;
        let op8_1 = self.operands[1].value as u8;
        let c_in: u16 = if self.flags_in & PSW_C != 0 { 1 } else { 0 };
        let carry_8 = (((op8_0 as u16) & 0xFF) + (op8_1 as u16) + c_in) >> 8 != 0;
        let carry_7 = (((op8_0 as u16) & 0x7F) + (op8_1 as u16) & 0x7F + c_in) >> 7 != 0;
        let carry_4 = (((op8_0 as u16) & 0x0F) + (op8_1 as u16) & 0x0F + c_in) >> 4 != 0;
        self.flags_changed |= PSW_C | PSW_OV | PSW_HC;
        self.flags_out = (self.flags_out & !PSW_C) | if carry_8 { PSW_C } else { 0 };
        self.flags_out = (self.flags_out & !PSW_OV) | if carry_8 ^ carry_7 { PSW_OV } else { 0 };
        self.flags_out = (self.flags_out & !PSW_HC) | if carry_4 { PSW_HC } else { 0 };
        self.operands[0].value = (op8_0.wrapping_add(op8_1).wrapping_add(c_in as u8)) as u64;
    }

    fn zs_check(&mut self) {
        self.flags_changed |= PSW_Z | PSW_S;
        if self.operands[0].value & 0xFF != 0 {
            self.flags_out &= !PSW_Z;
        }
        self.flags_out = (self.flags_out & !PSW_S)
            | if self.operands[0].value & 0x80 != 0 { PSW_S } else { 0 };
    }

    fn shift_left8(&mut self) {
        self.operands[0].value &= 0xFF;
        let shift_by = (self.operands[1].value & 7) as u32;
        let mut result = (self.operands[0].value as u16) << shift_by;
        result |= (self.shift_buffer as u16) >> (8 - shift_by);
        self.flags_changed |= PSW_C;
        if result & 0x100 != 0 {
            self.flags_out |= PSW_C;
        }
        self.operands[0].value = (result & 0xFF) as u64;
    }

    fn shift_right8(&mut self) {
        self.operands[0].value &= 0xFF;
        let shift_by = (self.operands[1].value & 7) as u32;
        // C++ computes in (possibly truncated) uint16_t; use u32 + mask so a
        // shift-by-16 (shift_by == 0) behaves like the C++ int promotion.
        let mut result = (self.operands[0].value as u32) << (8 - shift_by);
        result |= (self.shift_buffer as u32) << (16 - shift_by);
        result &= 0xFFFF;
        self.flags_changed |= PSW_C;
        if result & 0x80 != 0 {
            self.flags_out |= PSW_C;
        }
        self.operands[0].value = ((result >> 8) & 0xFF) as u64;
    }
}

fn run_handler(cpu: &mut Cpu, mmu: &mut Mmu, index: usize) {
    match index {
        0 => cpu.op_add(mmu),
        1 => cpu.op_add16(mmu),
        2 => cpu.op_addc(mmu),
        3 => cpu.op_and(mmu),
        4 => cpu.op_sub(mmu),
        5 => cpu.op_subc(mmu),
        6 => cpu.op_mov16(mmu),
        7 => cpu.op_mov(mmu),
        8 => cpu.op_or(mmu),
        9 => cpu.op_xor(mmu),
        10 => cpu.op_cmp16(mmu),
        11 => cpu.op_sll(mmu),
        12 => cpu.op_sllc(mmu),
        13 => cpu.op_sra(mmu),
        14 => cpu.op_srl(mmu),
        15 => cpu.op_srlc(mmu),
        16 => cpu.op_ls_ea(mmu),
        17 => cpu.op_ls_r(mmu),
        18 => cpu.op_ls_i_r(mmu),
        19 => cpu.op_ls_bp(mmu),
        20 => cpu.op_ls_fp(mmu),
        21 => cpu.op_ls_i(mmu),
        22 => cpu.op_addsp(mmu),
        23 => cpu.op_ctrl(mmu),
        24 => cpu.op_push(mmu),
        25 => cpu.op_pushl(mmu),
        26 => cpu.op_pop(mmu),
        27 => cpu.op_popl(mmu),
        28 => cpu.op_cr_r(mmu),
        29 => cpu.op_cr_ea(mmu),
        30 => cpu.op_lea(mmu),
        31 => cpu.op_daa(mmu),
        32 => cpu.op_das(mmu),
        33 => cpu.op_neg(mmu),
        34 => cpu.op_bitmod(mmu),
        35 => cpu.op_psw_or(mmu),
        36 => cpu.op_psw_and(mmu),
        37 => cpu.op_cplc(mmu),
        38 => cpu.op_bc(mmu),
        39 => cpu.op_extbw(mmu),
        40 => cpu.op_swi(mmu),
        41 => cpu.op_brk(mmu),
        42 => cpu.op_b(mmu),
        43 => cpu.op_bl(mmu),
        44 => cpu.op_mul(mmu),
        45 => cpu.op_div(mmu),
        46 => cpu.op_inc_ea(mmu),
        47 => cpu.op_dec_ea(mmu),
        48 => cpu.op_rt(mmu),
        49 => cpu.op_rti(mmu),
        50 => cpu.op_nop(mmu),
        51 => cpu.op_dsr(mmu),
        52 => cpu.op_rtice(mmu),
        53 => cpu.op_iceswi(mmu),
        _ => {}
    }
}