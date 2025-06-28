use crate::memory::Memory;

#[derive(Debug)]
pub struct CPU {
    // Registers
    a: u8,      // Accumulator
    x: u8,      // X Index Register
    y: u8,      // Y Index Register
    pc: u16,    // Program Counter
    sp: u8,     // Stack Pointer
    status: u8, // Status Register
    
    // Internal state
    cycles: u64,
    halted: bool,
}

// Status register flags
const CARRY_FLAG: u8 = 0x01;
const ZERO_FLAG: u8 = 0x02;
const INTERRUPT_DISABLE: u8 = 0x04;
const DECIMAL_MODE: u8 = 0x08;
const BREAK_COMMAND: u8 = 0x10;
const UNUSED_FLAG: u8 = 0x20;
const OVERFLOW_FLAG: u8 = 0x40;
const NEGATIVE_FLAG: u8 = 0x80;

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            status: UNUSED_FLAG | INTERRUPT_DISABLE,
            cycles: 0,
            halted: false,
        }
    }
    
    pub fn reset(&mut self, memory: &mut Memory) {
        let low = memory.read(0xFFFC) as u16;
        let high = memory.read(0xFFFD) as u16;
        self.pc = (high << 8) | low;
        
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = UNUSED_FLAG | INTERRUPT_DISABLE;
        self.cycles = 0;
        self.halted = false;
    }
    
    pub fn step(&mut self, memory: &mut Memory) {
        if self.halted {
            return;
        }
        
        let opcode = memory.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        
        match opcode {
            // LDA - Load Accumulator
            0xA9 => self.lda_immediate(memory),
            0xA5 => self.lda_zero_page(memory),
            0xB5 => self.lda_zero_page_x(memory),
            0xAD => self.lda_absolute(memory),
            0xBD => self.lda_absolute_x(memory),
            0xB9 => self.lda_absolute_y(memory),
            0xA1 => self.lda_indexed_indirect(memory),
            0xB1 => self.lda_indirect_indexed(memory),
            
            // LDX - Load X Register
            0xA2 => self.ldx_immediate(memory),
            0xA6 => self.ldx_zero_page(memory),
            0xB6 => self.ldx_zero_page_y(memory),
            0xAE => self.ldx_absolute(memory),
            0xBE => self.ldx_absolute_y(memory),
            
            // LDY - Load Y Register
            0xA0 => self.ldy_immediate(memory),
            0xA4 => self.ldy_zero_page(memory),
            0xB4 => self.ldy_zero_page_x(memory),
            0xAC => self.ldy_absolute(memory),
            0xBC => self.ldy_absolute_x(memory),
            
            // STA - Store Accumulator
            0x85 => self.sta_zero_page(memory),
            0x95 => self.sta_zero_page_x(memory),
            0x8D => self.sta_absolute(memory),
            0x9D => self.sta_absolute_x(memory),
            0x99 => self.sta_absolute_y(memory),
            0x81 => self.sta_indexed_indirect(memory),
            0x91 => self.sta_indirect_indexed(memory),
            
            // ADC - Add with Carry
            0x69 => self.adc_immediate(memory),
            0x65 => self.adc_zero_page(memory),
            0x75 => self.adc_zero_page_x(memory),
            0x6D => self.adc_absolute(memory),
            0x7D => self.adc_absolute_x(memory),
            0x79 => self.adc_absolute_y(memory),
            0x61 => self.adc_indexed_indirect(memory),
            0x71 => self.adc_indirect_indexed(memory),
            
            // SBC - Subtract with Carry
            0xE9 => self.sbc_immediate(memory),
            0xE5 => self.sbc_zero_page(memory),
            0xF5 => self.sbc_zero_page_x(memory),
            0xED => self.sbc_absolute(memory),
            0xFD => self.sbc_absolute_x(memory),
            0xF9 => self.sbc_absolute_y(memory),
            0xE1 => self.sbc_indexed_indirect(memory),
            0xF1 => self.sbc_indirect_indexed(memory),
            
            // CMP - Compare
            0xC9 => self.cmp_immediate(memory),
            0xC5 => self.cmp_zero_page(memory),
            0xD5 => self.cmp_zero_page_x(memory),
            0xCD => self.cmp_absolute(memory),
            0xDD => self.cmp_absolute_x(memory),
            0xD9 => self.cmp_absolute_y(memory),
            0xC1 => self.cmp_indexed_indirect(memory),
            0xD1 => self.cmp_indirect_indexed(memory),
            
            // CPX - Compare X Register
            0xE0 => self.cpx_immediate(memory),
            0xE4 => self.cpx_zero_page(memory),
            0xEC => self.cpx_absolute(memory),
            
            // CPY - Compare Y Register
            0xC0 => self.cpy_immediate(memory),
            0xC4 => self.cpy_zero_page(memory),
            0xCC => self.cpy_absolute(memory),
            
            // AND - Logical AND
            0x29 => self.and_immediate(memory),
            0x25 => self.and_zero_page(memory),
            0x35 => self.and_zero_page_x(memory),
            0x2D => self.and_absolute(memory),
            0x3D => self.and_absolute_x(memory),
            0x39 => self.and_absolute_y(memory),
            0x21 => self.and_indexed_indirect(memory),
            0x31 => self.and_indirect_indexed(memory),
            
            // ORA - Logical OR
            0x09 => self.ora_immediate(memory),
            0x05 => self.ora_zero_page(memory),
            0x15 => self.ora_zero_page_x(memory),
            0x0D => self.ora_absolute(memory),
            0x1D => self.ora_absolute_x(memory),
            0x19 => self.ora_absolute_y(memory),
            0x01 => self.ora_indexed_indirect(memory),
            0x11 => self.ora_indirect_indexed(memory),
            
            // EOR - Exclusive OR
            0x49 => self.eor_immediate(memory),
            0x45 => self.eor_zero_page(memory),
            0x55 => self.eor_zero_page_x(memory),
            0x4D => self.eor_absolute(memory),
            0x5D => self.eor_absolute_x(memory),
            0x59 => self.eor_absolute_y(memory),
            0x41 => self.eor_indexed_indirect(memory),
            0x51 => self.eor_indirect_indexed(memory),
            
            // INC - Increment Memory
            0xE6 => self.inc_zero_page(memory),
            0xF6 => self.inc_zero_page_x(memory),
            0xEE => self.inc_absolute(memory),
            0xFE => self.inc_absolute_x(memory),
            
            // DEC - Decrement Memory
            0xC6 => self.dec_zero_page(memory),
            0xD6 => self.dec_zero_page_x(memory),
            0xCE => self.dec_absolute(memory),
            0xDE => self.dec_absolute_x(memory),
            
            // INX - Increment X Register
            0xE8 => self.inx(),
            
            // INY - Increment Y Register
            0xC8 => self.iny(),
            
            // DEX - Decrement X Register
            0xCA => self.dex(),
            
            // DEY - Decrement Y Register
            0x88 => self.dey(),
            
            // TAX - Transfer A to X
            0xAA => self.tax(),
            
            // TAY - Transfer A to Y
            0xA8 => self.tay(),
            
            // TXA - Transfer X to A
            0x8A => self.txa(),
            
            // TYA - Transfer Y to A
            0x98 => self.tya(),
            
            // TSX - Transfer Stack Pointer to X
            0xBA => self.tsx(),
            
            // TXS - Transfer X to Stack Pointer
            0x9A => self.txs(),
            
            // JMP - Jump
            0x4C => self.jmp_absolute(memory),
            0x6C => self.jmp_indirect(memory),
            
            // JSR - Jump to Subroutine
            0x20 => self.jsr(memory),
            
            // RTS - Return from Subroutine
            0x60 => self.rts(memory),
            
            // BRK - Break
            0x00 => self.brk(memory),
            
            // NOP - No Operation
            0xEA => self.nop(),
            
            _ => {
                panic!("Unknown opcode: ${:02X} at PC: ${:04X}", opcode, self.pc - 1);
            }
        }
        
        self.cycles += 1;
    }
    
    // Getters
    pub fn get_register_a(&self) -> u8 { self.a }
    pub fn get_register_x(&self) -> u8 { self.x }
    pub fn get_register_y(&self) -> u8 { self.y }
    pub fn get_pc(&self) -> u16 { self.pc }
    pub fn get_sp(&self) -> u8 { self.sp }
    pub fn get_status(&self) -> u8 { self.status }
    pub fn is_halted(&self) -> bool { self.halted }
    
    // Flag operations
    fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }
    
    fn get_flag(&self, flag: u8) -> bool {
        (self.status & flag) != 0
    }
    
    fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.set_flag(ZERO_FLAG, value == 0);
        self.set_flag(NEGATIVE_FLAG, (value & 0x80) != 0);
    }
    
    // Addressing mode implementations
    fn read_immediate(&mut self, memory: &Memory) -> u8 {
        let value = memory.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }
    
    fn read_zero_page(&mut self, memory: &Memory) -> u8 {
        let addr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        memory.read(addr)
    }
    
    fn read_zero_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        memory.read(addr)
    }
    
    fn read_zero_page_y(&mut self, memory: &Memory) -> u8 {
        let addr = (memory.read(self.pc).wrapping_add(self.y)) as u16;
        self.pc = self.pc.wrapping_add(1);
        memory.read(addr)
    }
    
    fn read_absolute(&mut self, memory: &Memory) -> u8 {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = (high << 8) | low;
        self.pc = self.pc.wrapping_add(2);
        memory.read(addr)
    }
    
    fn read_absolute_x(&mut self, memory: &Memory) -> u8 {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.x as u16);
        self.pc = self.pc.wrapping_add(2);
        memory.read(addr)
    }
    
    fn read_absolute_y(&mut self, memory: &Memory) -> u8 {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        self.pc = self.pc.wrapping_add(2);
        memory.read(addr)
    }
    
    // Instruction implementations
    fn lda_immediate(&mut self, memory: &Memory) {
        self.a = self.read_immediate(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_zero_page(&mut self, memory: &Memory) {
        self.a = self.read_zero_page(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_zero_page_x(&mut self, memory: &Memory) {
        self.a = self.read_zero_page_x(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_absolute(&mut self, memory: &Memory) {
        self.a = self.read_absolute(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_absolute_x(&mut self, memory: &Memory) {
        self.a = self.read_absolute_x(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_absolute_y(&mut self, memory: &Memory) {
        self.a = self.read_absolute_y(memory);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        self.a = memory.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn lda_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        self.a = memory.read(addr);
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ldx_immediate(&mut self, memory: &Memory) {
        self.x = self.read_immediate(memory);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn ldx_zero_page(&mut self, memory: &Memory) {
        self.x = self.read_zero_page(memory);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn ldx_zero_page_y(&mut self, memory: &Memory) {
        self.x = self.read_zero_page_y(memory);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn ldx_absolute(&mut self, memory: &Memory) {
        self.x = self.read_absolute(memory);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn ldx_absolute_y(&mut self, memory: &Memory) {
        self.x = self.read_absolute_y(memory);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn ldy_immediate(&mut self, memory: &Memory) {
        self.y = self.read_immediate(memory);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn ldy_zero_page(&mut self, memory: &Memory) {
        self.y = self.read_zero_page(memory);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn ldy_zero_page_x(&mut self, memory: &Memory) {
        self.y = self.read_zero_page_x(memory);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn ldy_absolute(&mut self, memory: &Memory) {
        self.y = self.read_absolute(memory);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn ldy_absolute_x(&mut self, memory: &Memory) {
        self.y = self.read_absolute_x(memory);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn sta_zero_page(&mut self, memory: &mut Memory) {
        let addr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        memory.write(addr, self.a);
    }
    
    fn sta_zero_page_x(&mut self, memory: &mut Memory) {
        let addr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        memory.write(addr, self.a);
    }
    
    fn sta_absolute(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = (high << 8) | low;
        self.pc = self.pc.wrapping_add(2);
        memory.write(addr, self.a);
    }
    
    fn sta_absolute_x(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.x as u16);
        self.pc = self.pc.wrapping_add(2);
        memory.write(addr, self.a);
    }
    
    fn sta_absolute_y(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        self.pc = self.pc.wrapping_add(2);
        memory.write(addr, self.a);
    }
    
    fn sta_indexed_indirect(&mut self, memory: &mut Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        memory.write(addr, self.a);
    }
    
    fn sta_indirect_indexed(&mut self, memory: &mut Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        memory.write(addr, self.a);
    }
    
    fn brk(&mut self, _memory: &mut Memory) {
        self.halted = true;
    }
    
    fn nop(&mut self) {
        // Do nothing
    }
    
    // Arithmetic operations
    fn adc_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.adc(value);
    }
    
    fn adc_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.adc(value);
    }
    
    fn adc_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.adc(value);
    }
    
    fn adc_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.adc(value);
    }
    
    fn adc_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.adc(value);
    }
    
    fn adc_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.adc(value);
    }
    
    fn adc_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.adc(value);
    }
    
    fn adc_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.adc(value);
    }
    
    fn adc(&mut self, value: u8) {
        let carry = if self.get_flag(CARRY_FLAG) { 1 } else { 0 };
        let result = self.a as u16 + value as u16 + carry as u16;
        
        let overflow = (self.a ^ result as u8) & (value ^ result as u8) & 0x80 != 0;
        
        self.set_flag(CARRY_FLAG, result > 255);
        self.set_flag(OVERFLOW_FLAG, overflow);
        
        self.a = result as u8;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn sbc_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.sbc(value);
    }
    
    fn sbc_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.sbc(value);
    }
    
    fn sbc_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.sbc(value);
    }
    
    fn sbc_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.sbc(value);
    }
    
    fn sbc_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.sbc(value);
    }
    
    fn sbc_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.sbc(value);
    }
    
    fn sbc_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.sbc(value);
    }
    
    fn sbc_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.sbc(value);
    }
    
    fn sbc(&mut self, value: u8) {
        let carry = if self.get_flag(CARRY_FLAG) { 0 } else { 1 };
        let result = (self.a as i16) - (value as i16) - (carry as i16);
        
        let overflow = ((self.a as i16) ^ result) & ((self.a as i16) ^ (value as i16)) & 0x80 != 0;
        
        self.set_flag(CARRY_FLAG, result >= 0);
        self.set_flag(OVERFLOW_FLAG, overflow);
        
        self.a = result as u8;
        self.update_zero_and_negative_flags(self.a);
    }
    
    // Compare operations
    fn cmp_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.compare(self.a, value);
    }
    
    fn cmp_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.compare(self.a, value);
    }
    
    fn cmp_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.compare(self.a, value);
    }
    
    fn cpx_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.compare(self.x, value);
    }
    
    fn cpx_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.compare(self.x, value);
    }
    
    fn cpx_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.compare(self.x, value);
    }
    
    fn cpy_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.compare(self.y, value);
    }
    
    fn cpy_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.compare(self.y, value);
    }
    
    fn cpy_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.compare(self.y, value);
    }
    
    fn compare(&mut self, register: u8, value: u8) {
        let result = register.wrapping_sub(value);
        self.set_flag(CARRY_FLAG, register >= value);
        self.update_zero_and_negative_flags(result);
    }
    
    // Logical operations
    fn and_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn and_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.a &= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn ora_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.a |= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_immediate(&mut self, memory: &Memory) {
        let value = self.read_immediate(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_zero_page(&mut self, memory: &Memory) {
        let value = self.read_zero_page(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_zero_page_x(&mut self, memory: &Memory) {
        let value = self.read_zero_page_x(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_absolute(&mut self, memory: &Memory) {
        let value = self.read_absolute(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_absolute_x(&mut self, memory: &Memory) {
        let value = self.read_absolute_x(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_absolute_y(&mut self, memory: &Memory) {
        let value = self.read_absolute_y(memory);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_indexed_indirect(&mut self, memory: &Memory) {
        let ptr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = (high << 8) | low;
        let value = memory.read(addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn eor_indirect_indexed(&mut self, memory: &Memory) {
        let ptr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let low = memory.read(ptr) as u16;
        let high = memory.read(ptr.wrapping_add(1)) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.y as u16);
        let value = memory.read(addr);
        self.a ^= value;
        self.update_zero_and_negative_flags(self.a);
    }
    
    // Increment/Decrement operations
    fn inc_zero_page(&mut self, memory: &mut Memory) {
        let addr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn inc_zero_page_x(&mut self, memory: &mut Memory) {
        let addr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn inc_absolute(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = (high << 8) | low;
        self.pc = self.pc.wrapping_add(2);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn inc_absolute_x(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.x as u16);
        self.pc = self.pc.wrapping_add(2);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn dec_zero_page(&mut self, memory: &mut Memory) {
        let addr = memory.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn dec_zero_page_x(&mut self, memory: &mut Memory) {
        let addr = (memory.read(self.pc).wrapping_add(self.x)) as u16;
        self.pc = self.pc.wrapping_add(1);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn dec_absolute(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = (high << 8) | low;
        self.pc = self.pc.wrapping_add(2);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn dec_absolute_x(&mut self, memory: &mut Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        let addr = ((high << 8) | low).wrapping_add(self.x as u16);
        self.pc = self.pc.wrapping_add(2);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.update_zero_and_negative_flags(value);
    }
    
    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.y);
    }
    
    // Transfer operations
    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn tay(&mut self) {
        self.y = self.a;
        self.update_zero_and_negative_flags(self.y);
    }
    
    fn txa(&mut self) {
        self.a = self.x;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn tya(&mut self) {
        self.a = self.y;
        self.update_zero_and_negative_flags(self.a);
    }
    
    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_zero_and_negative_flags(self.x);
    }
    
    fn txs(&mut self) {
        self.sp = self.x;
    }
    
    // Jump operations
    fn jmp_absolute(&mut self, memory: &Memory) {
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        self.pc = (high << 8) | low;
    }
    
    fn jmp_indirect(&mut self, memory: &Memory) {
        let ptr_low = memory.read(self.pc) as u16;
        let ptr_high = memory.read(self.pc + 1) as u16;
        let ptr = (ptr_high << 8) | ptr_low;
        
        // 6502 bug: if ptr is at page boundary, high byte wraps around within the page
        let low = memory.read(ptr) as u16;
        let high = if ptr & 0xFF == 0xFF {
            memory.read(ptr & 0xFF00) as u16
        } else {
            memory.read(ptr + 1) as u16
        };
        
        self.pc = (high << 8) | low;
    }
    
    fn jsr(&mut self, memory: &mut Memory) {
        let return_addr = self.pc.wrapping_add(1);
        self.push_u16(memory, return_addr);
        
        let low = memory.read(self.pc) as u16;
        let high = memory.read(self.pc + 1) as u16;
        self.pc = (high << 8) | low;
    }
    
    fn rts(&mut self, memory: &Memory) {
        self.pc = self.pop_u16(memory).wrapping_add(1);
    }
    
    // Stack operations
    fn push(&mut self, memory: &mut Memory, value: u8) {
        memory.write(0x100 + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }
    
    fn pop(&mut self, memory: &Memory) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        memory.read(0x100 + self.sp as u16)
    }
    
    fn push_u16(&mut self, memory: &mut Memory, value: u16) {
        self.push(memory, (value >> 8) as u8);
        self.push(memory, (value & 0xFF) as u8);
    }
    
    fn pop_u16(&mut self, memory: &Memory) -> u16 {
        let low = self.pop(memory) as u16;
        let high = self.pop(memory) as u16;
        (high << 8) | low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lda_immediate() {
        let mut cpu = CPU::new();
        let mut memory = Memory::new();
        
        // LDA #$42
        memory.write(0x8000, 0xA9);
        memory.write(0x8001, 0x42);
        memory.write(0xFFFC, 0x00);
        memory.write(0xFFFD, 0x80);
        
        cpu.reset(&mut memory);
        cpu.step(&mut memory);
        
        assert_eq!(cpu.get_register_a(), 0x42);
        assert_eq!(cpu.get_pc(), 0x8002);
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }
    
    #[test]
    fn test_adc() {
        let mut cpu = CPU::new();
        let mut memory = Memory::new();
        
        // LDA #$10, ADC #$20
        memory.write(0x8000, 0xA9); // LDA #$10
        memory.write(0x8001, 0x10);
        memory.write(0x8002, 0x69); // ADC #$20
        memory.write(0x8003, 0x20);
        memory.write(0xFFFC, 0x00);
        memory.write(0xFFFD, 0x80);
        
        cpu.reset(&mut memory);
        cpu.step(&mut memory); // LDA
        cpu.step(&mut memory); // ADC
        
        assert_eq!(cpu.get_register_a(), 0x30);
        assert!(!cpu.get_flag(CARRY_FLAG));
        assert!(!cpu.get_flag(ZERO_FLAG));
        assert!(!cpu.get_flag(NEGATIVE_FLAG));
    }
    
    #[test]
    fn test_transfer_instructions() {
        let mut cpu = CPU::new();
        let mut memory = Memory::new();
        
        // LDA #$42, TAX, TAY
        memory.write(0x8000, 0xA9); // LDA #$42
        memory.write(0x8001, 0x42);
        memory.write(0x8002, 0xAA); // TAX
        memory.write(0x8003, 0xA8); // TAY
        memory.write(0xFFFC, 0x00);
        memory.write(0xFFFD, 0x80);
        
        cpu.reset(&mut memory);
        cpu.step(&mut memory); // LDA
        cpu.step(&mut memory); // TAX
        cpu.step(&mut memory); // TAY
        
        assert_eq!(cpu.get_register_a(), 0x42);
        assert_eq!(cpu.get_register_x(), 0x42);
        assert_eq!(cpu.get_register_y(), 0x42);
    }
}