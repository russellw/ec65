use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::cpu::CPU;
use crate::memory::Memory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorSnapshot {
    pub id: String,
    pub name: String,
    pub description: String,
    pub emulator_id: String,
    pub owner_id: String,
    pub cpu_state: CpuSnapshot,
    pub memory_dump: Vec<u8>,
    pub metadata: SnapshotMetadata,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub sp: u8,
    pub status: u8,
    pub cycles: u64,
    pub halted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub emulator_type: String,
    pub template_id: Option<String>,
    pub checkpoint_reason: CheckpointReason,
    pub instruction_count: u64,
    pub execution_time_ms: u64,
    pub compression_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointReason {
    Manual,
    Automatic,
    BeforeRisk,     // Before potentially dangerous operation
    Scheduled,      // Periodic backup
    BeforeShutdown,
    Breakpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub compress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSnapshotRequest {
    pub snapshot_id: String,
    pub force: Option<bool>, // Restore even if it would overwrite running state
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotListResponse {
    pub snapshots: Vec<SnapshotSummary>,
    pub total_count: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub checkpoint_reason: CheckpointReason,
    pub tags: Vec<String>,
}

pub type SnapshotStore = std::sync::Arc<std::sync::Mutex<HashMap<String, EmulatorSnapshot>>>;

impl EmulatorSnapshot {
    pub fn create_from_emulator(
        name: String,
        description: String,
        emulator_id: String,
        owner_id: String,
        cpu: &CPU,
        memory: &Memory,
        emulator_type: String,
        template_id: Option<String>,
        reason: CheckpointReason,
        instruction_count: u64,
        execution_time_ms: u64,
        tags: Vec<String>,
    ) -> Self {
        let cpu_state = CpuSnapshot {
            a: cpu.get_register_a(),
            x: cpu.get_register_x(),
            y: cpu.get_register_y(),
            pc: cpu.get_pc(),
            sp: cpu.get_sp(),
            status: cpu.get_status(),
            cycles: 0, // We'd need to expose this from CPU
            halted: cpu.is_halted(),
        };
        
        // Create memory dump
        let mut memory_dump = Vec::with_capacity(65536);
        for addr in 0..65536 {
            memory_dump.push(memory.read(addr as u16));
        }
        
        // Compress memory if mostly zeros (common case)
        let original_size = memory_dump.len();
        let compressed_dump = compress_memory(&memory_dump);
        let compression_ratio = compressed_dump.len() as f32 / original_size as f32;
        
        let metadata = SnapshotMetadata {
            emulator_type,
            template_id,
            checkpoint_reason: reason,
            instruction_count,
            execution_time_ms,
            compression_ratio,
        };
        
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            emulator_id,
            owner_id,
            cpu_state,
            memory_dump: compressed_dump.clone(),
            metadata,
            created_at: Utc::now(),
            size_bytes: compressed_dump.len() as u64,
            tags,
        }
    }
    
    pub fn restore_to_emulator(&self, cpu: &mut CPU, memory: &mut Memory) -> Result<(), String> {
        // Restore CPU state
        cpu.set_register_a(self.cpu_state.a);
        cpu.set_register_x(self.cpu_state.x);
        cpu.set_register_y(self.cpu_state.y);
        cpu.set_pc(self.cpu_state.pc);
        cpu.set_sp(self.cpu_state.sp);
        cpu.set_status(self.cpu_state.status);
        
        if self.cpu_state.halted {
            cpu.halt();
        }
        
        // Restore memory
        let decompressed_memory = decompress_memory(&self.memory_dump)?;
        if decompressed_memory.len() != 65536 {
            return Err("Invalid memory dump size".to_string());
        }
        
        for (addr, &value) in decompressed_memory.iter().enumerate() {
            memory.write(addr as u16, value);
        }
        
        Ok(())
    }
    
    pub fn can_user_access(&self, user_id: &str, is_admin: bool) -> bool {
        is_admin || self.owner_id == user_id
    }
    
    pub fn get_summary(&self) -> SnapshotSummary {
        SnapshotSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
            size_bytes: self.size_bytes,
            checkpoint_reason: self.metadata.checkpoint_reason.clone(),
            tags: self.tags.clone(),
        }
    }
}

// Simple run-length encoding for memory compression
fn compress_memory(memory: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut i = 0;
    
    while i < memory.len() {
        let current_byte = memory[i];
        let mut count = 1;
        
        // Count consecutive identical bytes (max 255)
        while i + count < memory.len() && 
              memory[i + count] == current_byte && 
              count < 255 {
            count += 1;
        }
        
        if count > 3 || current_byte == 0 {
            // Use RLE for runs of 4+ or any zeros
            compressed.push(0xFF); // RLE marker
            compressed.push(count as u8);
            compressed.push(current_byte);
        } else {
            // Store literal bytes
            for j in 0..count {
                if memory[i + j] == 0xFF {
                    // Escape literal 0xFF
                    compressed.push(0xFF);
                    compressed.push(0x00);
                } else {
                    compressed.push(memory[i + j]);
                }
            }
        }
        
        i += count;
    }
    
    compressed
}

fn decompress_memory(compressed: &[u8]) -> Result<Vec<u8>, String> {
    let mut decompressed = Vec::with_capacity(65536);
    let mut i = 0;
    
    while i < compressed.len() {
        if compressed[i] == 0xFF {
            if i + 1 >= compressed.len() {
                return Err("Truncated RLE data".to_string());
            }
            
            if compressed[i + 1] == 0x00 {
                // Escaped literal 0xFF
                decompressed.push(0xFF);
                i += 2;
            } else {
                // RLE sequence
                if i + 2 >= compressed.len() {
                    return Err("Truncated RLE sequence".to_string());
                }
                
                let count = compressed[i + 1];
                let value = compressed[i + 2];
                
                for _ in 0..count {
                    decompressed.push(value);
                }
                
                i += 3;
            }
        } else {
            // Literal byte
            decompressed.push(compressed[i]);
            i += 1;
        }
    }
    
    if decompressed.len() != 65536 {
        return Err(format!("Decompressed size {} != 65536", decompressed.len()));
    }
    
    Ok(decompressed)
}

// Extensions to CPU for snapshot support
impl CPU {
    pub fn set_register_a(&mut self, value: u8) {
        self.a = value;
    }
    
    pub fn set_register_x(&mut self, value: u8) {
        self.x = value;
    }
    
    pub fn set_register_y(&mut self, value: u8) {
        self.y = value;
    }
    
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }
    
    pub fn set_sp(&mut self, value: u8) {
        self.sp = value;
    }
    
    pub fn set_status(&mut self, value: u8) {
        self.status = value;
    }
    
    pub fn halt(&mut self) {
        self.halted = true;
    }
    
    pub fn resume(&mut self) {
        self.halted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_compression() {
        let mut memory = vec![0u8; 65536];
        
        // Add some patterns
        memory[0x1000] = 0xFF;
        memory[0x1001] = 0xFF;
        memory[0x1002] = 0xFF;
        memory[0x1003] = 0xFF;
        
        memory[0x2000] = 0xAA;
        memory[0x2001] = 0xBB;
        memory[0x2002] = 0xCC;
        
        let compressed = compress_memory(&memory);
        let decompressed = decompress_memory(&compressed).unwrap();
        
        assert_eq!(memory, decompressed);
        assert!(compressed.len() < memory.len()); // Should be smaller
    }
    
    #[test]
    fn test_rle_escape() {
        let mut memory = vec![0x00; 65536];
        memory[0] = 0xFF;
        memory[1] = 0xFF;
        memory[2] = 0xAA;
        memory[3] = 0xFF;
        memory[4] = 0x00;
        
        let compressed = compress_memory(&memory);
        let decompressed = decompress_memory(&compressed).unwrap();
        
        assert_eq!(memory, decompressed);
    }
}