//! # MOS 6502 CPU Emulator
//! 
//! A complete implementation of the MOS 6502 8-bit microprocessor in Rust.
//! This emulator provides accurate instruction execution, flag handling, and
//! memory management for the 6502 CPU as used in classic computers like the
//! Apple II, Commodore 64, and NES.
//! 
//! ## Features
//! 
//! - Complete 6502 instruction set implementation
//! - Accurate flag handling for all arithmetic and logic operations
//! - Multiple addressing modes (immediate, zero page, absolute, indexed, indirect)
//! - Stack operations and subroutine calls
//! - Historic 6502 bugs for authenticity (JMP indirect page boundary bug)
//! - Comprehensive test suite
//! 
//! ## Example
//! 
//! ```rust
//! use mos6502_emulator::cpu::CPU;
//! use mos6502_emulator::memory::Memory;
//! 
//! let mut cpu = CPU::new();
//! let mut memory = Memory::new();
//! 
//! // Load a simple program: LDA #$42, BRK
//! memory.write(0x8000, 0xA9); // LDA #$42
//! memory.write(0x8001, 0x42);
//! memory.write(0x8002, 0x00); // BRK
//! 
//! // Set reset vector
//! memory.write(0xFFFC, 0x00);
//! memory.write(0xFFFD, 0x80);
//! 
//! cpu.reset(&mut memory);
//! cpu.step(&mut memory); // Execute LDA
//! 
//! assert_eq!(cpu.get_register_a(), 0x42);
//! ```

#![recursion_limit = "2048"]

pub mod cpu;
pub mod memory;
pub mod server;
pub mod metrics;
pub mod auth;
pub mod instance_types;
pub mod snapshots;

pub use cpu::CPU;
pub use memory::Memory;