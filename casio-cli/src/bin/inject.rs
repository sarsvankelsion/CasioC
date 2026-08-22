use casio_emu::{cpu::Cpu, mmu::Mmu};
use std::fs;

fn main() {
    let rom = fs::read(r"D:\casioai\hdcompiler_vn\580vnx\rom.bin").expect("rom");
    
    let raw_payload = fs::read_to_string(r"D:\casioai\tetris_rock_solid_payload.txt").unwrap_or_default();
    let payload: Vec<u8> = raw_payload
        .lines()
        .filter(|l| l.contains("==="))
        .next()
        .map(|_| {
            let mut bytes = Vec::new();
            let mut capture = false;
            for line in raw_payload.lines() {
                if line.contains("===") {
                    capture = true;
                    continue;
                }
                if capture && !line.trim().is_empty() {
                    for part in line.split_whitespace() {
                        if let Ok(b) = u8::from_str_radix(part, 16) {
                            bytes.push(b);
                        }
                    }
                }
            }
            bytes
        })
        .unwrap_or_default();
    
    println!("[ROCK-SOLID TETRIS] Loaded {} bytes payload", payload.len());
    
    let mut mmu = Mmu::new(rom, false);
    let mut cpu = Cpu::new();
    cpu.reset(&mut mmu);
    mmu.ints.raise_reset();
    mmu.accept_interrupt(&mut cpu);
    
    // Inject at E9E0
    for (i, b) in payload.iter().enumerate() {
        mmu.write_data(0xE9E0 + i as u32, *b);
    }
    
    // Launcher:
    let launcher_hex = "DA 7B 31 30 FE 02 E0 E9 30 D7 2E D7 32 89 31 30 30 30 74 1F 32 30";
    let launcher: Vec<u8> = launcher_hex.split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap()).collect();
    let launcher_sp = 0xE000u32;
    for (i, b) in launcher.iter().enumerate() {
        mmu.write_data(launcher_sp + i as u32, *b);
    }
    
    let pop_pc: u32 = 0x13324;
    cpu.pc = (pop_pc & 0xFFFF) as u16;
    cpu.csr = ((pop_pc >> 16) & 0xFF) as u16;
    cpu.sp = launcher_sp as u16;
    
    let mut steps = 0u32;
    // Run 300,000 steps across many drops and resets
    while steps < 300000 && cpu.run {
        cpu.next(&mut mmu);
        steps += 1;
        if steps % 100 == 0 { mmu.tick(); }
    }
    
    println!("[ROCK-SOLID TETRIS] Executed {} steps | pc={:04X}:{:04X} sp={:04X}", steps, cpu.csr, cpu.pc, cpu.sp);
    
    // Dump 0xF800 LCD screen buffer
    println!("=== 0xF800 LCD SCREEN BUFFER ===");
    for row in 0..16 {
        let mut line = String::new();
        for col in 0..24 {
            let b = mmu.read_data(0xF800 + (row * 24 + col) as u32);
            for bit in 0..8 {
                line.push(if b & (0x80 >> bit) != 0 { '#' } else { '.' });
            }
        }
        println!("Row {:02}: {}", row, line);
    }
}
