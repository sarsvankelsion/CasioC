use casio_emu::{cpu::Cpu, mmu::Mmu};
use std::fs;

fn main() {
    let rom = fs::read(r"D:\casioai\hdcompiler_vn\580vnx\rom.bin").expect("rom");
    let hex = "60 8c 30 30 34 7b 31 30 70 10 fe e9 7e 8f 30 30 7c 94 30 30 b0 3a 31 30 f4 e9 74 1f 32 30 68 65 6c 6c 6f 20 77 6f 72 6c 64";
    let payload: Vec<u8> = hex.split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap()).collect();
    let home = 0xE9E0u32;
    let mut mmu = Mmu::new(rom, false);
    let mut cpu = Cpu::new();
    cpu.reset(&mut mmu);
    mmu.ints.raise_reset();
    mmu.accept_interrupt(&mut cpu);
    // inject payload at home (RAM D000-F000 contains E9E0)
    for (i, b) in payload.iter().enumerate() {
        mmu.write_data(home + i as u32, *b);
    }
    // ROP start: SP -> payload, PC = pop pc gadget
    let pop_pc: u32 = 0x13324; // 580vnx pop pc
    cpu.pc = (pop_pc & 0xFFFF) as u16;
    cpu.csr = ((pop_pc >> 16) & 0xFF) as u16;
    cpu.sp = home as u16;
    println!("[INJECT] home={:04X} pop_pc={:05X} pc={:04X} csr={:02X} sp={:04X} len={}", home, pop_pc, cpu.pc, cpu.csr, cpu.sp, payload.len());
    let mut steps = 0u32;
    while steps < 50000 && cpu.run {
        cpu.next(&mut mmu);
        steps += 1;
        if steps % 100 == 0 { mmu.tick(); }
        // break if we loop at csc_end
        if steps > 1000 && cpu.pc == 0xE9FB && cpu.csr == 0 { break; }
    }
    println!("[INJECT] after {} steps pc={:04X}:{:04X} sp={:04X} psw={:02X}", steps, cpu.csr, cpu.pc, cpu.sp, cpu.psw());
    // dump screen
    let mut out = String::new();
    for row in 0..8 {
        for col in 0..24 {
            let b = mmu.screen_buf[row*32 + col];
            out.push_str(&format!("{:02X} ", b));
        }
        out.push_str("\n");
    }
    println!("SCREEN first 8 rows hex:\n{}", out);
    // ascii dump
    for row in 0..12 {
        let mut line = String::new();
        for col in 0..24 {
            let b = mmu.screen_buf[row*32 + col];
            for bit in 0..8 {
                line.push(if b & (0x80 >> bit) != 0 { '#' } else { '.' });
            }
        }
        println!("{:02X} {}", row, line.chars().take(96).collect::<String>());
    }
}
