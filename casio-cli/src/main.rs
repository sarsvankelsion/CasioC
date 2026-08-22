use casio_emu::cpu::Cpu;
use casio_emu::mmu::Mmu;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let rom_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| r"D:\casioai\hdcompiler_vn\580vnx\rom.bin".to_string());
    let max_instr: u64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    let rom = fs::read(&rom_path).unwrap_or_else(|e| {
        eprintln!("Failed to read ROM {}: {}", rom_path, e);
        std::process::exit(1);
    });
    println!("[CLI] ROM: {} ({} bytes)", rom_path, rom.len());

    let mut mmu = Mmu::new(rom, false);
    let mut cpu = Cpu::new();
    cpu.reset(&mut mmu);
    mmu.ints.raise_reset();
    mmu.accept_interrupt(&mut cpu);
    println!(
        "[CLI] Reset: sp={:04x} pc={:06x}",
        cpu.sp,
        (cpu.csr as u32) << 16 | cpu.pc as u32
    );

    let mut instructions = 0u64;
    let tick_every: u64 = 100;
    while instructions < max_instr && cpu.run {
        cpu.next(&mut mmu);
        instructions += 1;
        if instructions % tick_every == 0 {
            mmu.tick();
        }
    }
    if !cpu.run {
        println!("[CLI] CPU halted after {} instructions", instructions);
    } else {
        println!("[CLI] Ran {} instructions", instructions);
    }
    println!(
        "[CLI] pc={:06x} sp={:04x} psw={:02x} ints={}",
        (cpu.csr as u32) << 16 | cpu.pc as u32,
        cpu.sp,
        cpu.psw(),
        mmu.ints.count
    );
    dump_screen(&mmu);
}

fn dump_screen(mmu: &Mmu) {
    let chars = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    println!("[CLI] Screen buffer {:04x}..{:04x}:", 0xF800usize, 0xF800 + 0x800);
    for row in 0..64 {
        let mut line = String::new();
        for ix in 0..24 {
            let byte = mmu.screen_buf[row * 32 + ix];
            for bit in 0..8 {
                let on = byte & (0x80 >> bit) != 0;
                line.push(chars[if on { 9 } else { 0 }]);
            }
        }
        println!("{:02x} {}", row, line);
    }
}