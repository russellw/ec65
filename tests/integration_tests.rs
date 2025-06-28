use mos6502_emulator::cpu::CPU;
use mos6502_emulator::memory::Memory;

#[test]
fn test_simple_calculation() {
    let mut cpu = CPU::new();
    let mut memory = Memory::new();
    
    // Calculate 10 + 20 + 30 and store result
    let program = [
        0xA9, 0x0A,       // LDA #$0A    ; A = 10
        0x69, 0x14,       // ADC #$14    ; A = A + 20 = 30
        0x69, 0x1E,       // ADC #$1E    ; A = A + 30 = 60
        0x85, 0x50,       // STA $50     ; Store result at $50
        0x00,             // BRK
    ];
    
    // Load program at $8000
    for (i, &byte) in program.iter().enumerate() {
        memory.write(0x8000 + i as u16, byte);
    }
    
    // Set reset vector
    memory.write(0xFFFC, 0x00);
    memory.write(0xFFFD, 0x80);
    
    cpu.reset(&mut memory);
    
    // Execute program
    for _ in 0..10 {
        if cpu.is_halted() {
            break;
        }
        cpu.step(&mut memory);
    }
    
    // Check result
    assert_eq!(cpu.get_register_a(), 60);
    assert_eq!(memory.read(0x50), 60);
    assert!(cpu.is_halted());
}

#[test]
fn test_memory_copy_single_byte() {
    let mut cpu = CPU::new();
    let mut memory = Memory::new();
    
    // Store test data
    memory.write(0x60, 0x42);
    
    let program = [
        0xA5, 0x60,       // LDA $60     ; Load from source
        0x85, 0x70,       // STA $70     ; Store to destination
        0x00,             // BRK
    ];
    
    // Load program
    for (i, &byte) in program.iter().enumerate() {
        memory.write(0x8000 + i as u16, byte);
    }
    
    memory.write(0xFFFC, 0x00);
    memory.write(0xFFFD, 0x80);
    
    cpu.reset(&mut memory);
    
    // Execute the copy
    cpu.step(&mut memory); // LDA $60
    cpu.step(&mut memory); // STA $70
    cpu.step(&mut memory); // BRK
    
    // Verify the copy
    assert_eq!(memory.read(0x70), 0x42);
    assert_eq!(cpu.get_register_a(), 0x42);
    assert!(cpu.is_halted());
}

#[test]
fn test_indexed_memory_access() {
    let mut cpu = CPU::new();
    let mut memory = Memory::new();
    
    // Store test data at multiple locations
    memory.write(0x50, 0x10);
    memory.write(0x51, 0x20);
    memory.write(0x52, 0x30);
    
    let program = [
        0xA2, 0x02,       // LDX #$02    ; X = 2
        0xB5, 0x50,       // LDA $50,X   ; Load from $50 + X = $52
        0x95, 0x60,       // STA $60,X   ; Store to $60 + X = $62
        0x00,             // BRK
    ];
    
    // Load program
    for (i, &byte) in program.iter().enumerate() {
        memory.write(0x8000 + i as u16, byte);
    }
    
    memory.write(0xFFFC, 0x00);
    memory.write(0xFFFD, 0x80);
    
    cpu.reset(&mut memory);
    
    // Execute
    cpu.step(&mut memory); // LDX #$02
    cpu.step(&mut memory); // LDA $50,X
    cpu.step(&mut memory); // STA $60,X
    cpu.step(&mut memory); // BRK
    
    // Verify
    assert_eq!(cpu.get_register_x(), 2);
    assert_eq!(cpu.get_register_a(), 0x30); // Value from $52
    assert_eq!(memory.read(0x62), 0x30);    // Stored at $60 + 2
    assert!(cpu.is_halted());
}