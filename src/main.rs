mod cpu;
mod memory;

use cpu::CPU;
use memory::Memory;

fn main() {
    let mut memory = Memory::new();
    let mut cpu = CPU::new();
    
    // Example: Load a simple program
    memory.write(0x8000, 0xA9); // LDA #$42
    memory.write(0x8001, 0x42);
    memory.write(0x8002, 0x00); // BRK
    
    // Set reset vector to 0x8000
    memory.write(0xFFFC, 0x00);
    memory.write(0xFFfd, 0x80);
    
    cpu.reset(&mut memory);
    
    // Run a few cycles
    for _ in 0..10 {
        cpu.step(&mut memory);
        if cpu.is_halted() {
            break;
        }
    }
    
    println!("CPU State:");
    println!("A: ${:02X}", cpu.get_register_a());
    println!("X: ${:02X}", cpu.get_register_x());
    println!("Y: ${:02X}", cpu.get_register_y());
    println!("PC: ${:04X}", cpu.get_pc());
    println!("SP: ${:02X}", cpu.get_sp());
    println!("Status: ${:02X}", cpu.get_status());
}
