use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    Micro,      // 100K cycles/sec, 16KB memory
    Small,      // 500K cycles/sec, 32KB memory  
    Standard,   // 1M cycles/sec, 64KB memory
    Performance,// 5M cycles/sec, 64KB memory
    Turbo,      // 10M cycles/sec, 64KB memory
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rom_image: Vec<u8>,
    pub reset_vector: u16,
    pub initial_memory: HashMap<u16, u8>,
    pub emulator_type: EmulatorType,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub is_public: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorSpecs {
    pub max_cycles_per_second: u64,
    pub memory_size: u64,
    pub execution_timeout_ms: u64,
    pub pricing_tier: PricingTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingTier {
    Free,
    Basic,
    Standard,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    pub template_id: Option<String>,
    pub emulator_type: EmulatorType,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub auto_start: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorInstance {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub emulator_type: EmulatorType,
    pub template_id: Option<String>,
    pub state: InstanceState,
    pub specs: EmulatorSpecs,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub last_activity: DateTime<Utc>,
    pub tags: Vec<String>,
    pub usage_stats: UsageStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Paused,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_cycles: u64,
    pub total_instructions: u64,
    pub runtime_seconds: u64,
    pub api_calls: u64,
    pub last_reset: DateTime<Utc>,
}

impl EmulatorType {
    pub fn get_specs(&self) -> EmulatorSpecs {
        match self {
            EmulatorType::Micro => EmulatorSpecs {
                max_cycles_per_second: 100_000,
                memory_size: 16 * 1024,
                execution_timeout_ms: 1000,
                pricing_tier: PricingTier::Free,
            },
            EmulatorType::Small => EmulatorSpecs {
                max_cycles_per_second: 500_000,
                memory_size: 32 * 1024,
                execution_timeout_ms: 5000,
                pricing_tier: PricingTier::Basic,
            },
            EmulatorType::Standard => EmulatorSpecs {
                max_cycles_per_second: 1_000_000,
                memory_size: 64 * 1024,
                execution_timeout_ms: 10000,
                pricing_tier: PricingTier::Standard,
            },
            EmulatorType::Performance => EmulatorSpecs {
                max_cycles_per_second: 5_000_000,
                memory_size: 64 * 1024,
                execution_timeout_ms: 30000,
                pricing_tier: PricingTier::Standard,
            },
            EmulatorType::Turbo => EmulatorSpecs {
                max_cycles_per_second: 10_000_000,
                memory_size: 64 * 1024,
                execution_timeout_ms: 60000,
                pricing_tier: PricingTier::Premium,
            },
        }
    }
    
    pub fn to_string(&self) -> &'static str {
        match self {
            EmulatorType::Micro => "micro",
            EmulatorType::Small => "small",
            EmulatorType::Standard => "standard",
            EmulatorType::Performance => "performance",
            EmulatorType::Turbo => "turbo",
        }
    }
}

impl Default for EmulatorType {
    fn default() -> Self {
        EmulatorType::Standard
    }
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            total_cycles: 0,
            total_instructions: 0,
            runtime_seconds: 0,
            api_calls: 0,
            last_reset: Utc::now(),
        }
    }
}

impl InstanceTemplate {
    pub fn create_basic_templates() -> Vec<InstanceTemplate> {
        vec![
            InstanceTemplate {
                id: "basic-6502".to_string(),
                name: "Basic 6502 System".to_string(),
                description: "Clean 6502 system with reset vector at $8000".to_string(),
                rom_image: vec![],
                reset_vector: 0x8000,
                initial_memory: {
                    let mut mem = HashMap::new();
                    mem.insert(0xFFFC, 0x00); // Reset vector low
                    mem.insert(0xFFFD, 0x80); // Reset vector high
                    mem
                },
                emulator_type: EmulatorType::Standard,
                created_by: "system".to_string(),
                created_at: Utc::now(),
                is_public: true,
                tags: vec!["basic".to_string(), "clean".to_string()],
            },
            InstanceTemplate {
                id: "apple-ii-like".to_string(),
                name: "Apple II-like System".to_string(),
                description: "6502 system similar to Apple II with ROM at $F000".to_string(),
                rom_image: vec![],
                reset_vector: 0xF000,
                initial_memory: {
                    let mut mem = HashMap::new();
                    // Apple II-like memory map
                    mem.insert(0xFFFC, 0x00); // Reset vector low
                    mem.insert(0xFFFD, 0xF0); // Reset vector high
                    mem
                },
                emulator_type: EmulatorType::Standard,
                created_by: "system".to_string(),
                created_at: Utc::now(),
                is_public: true,
                tags: vec!["apple-ii".to_string(), "retro".to_string()],
            },
            InstanceTemplate {
                id: "nes-like".to_string(),
                name: "NES-like System".to_string(),
                description: "6502 system similar to NES/Famicom".to_string(),
                rom_image: vec![],
                reset_vector: 0xC000,
                initial_memory: {
                    let mut mem = HashMap::new();
                    // NES-like memory map
                    mem.insert(0xFFFC, 0x00); // Reset vector low
                    mem.insert(0xFFFD, 0xC0); // Reset vector high
                    mem
                },
                emulator_type: EmulatorType::Performance,
                created_by: "system".to_string(),
                created_at: Utc::now(),
                is_public: true,
                tags: vec!["nes".to_string(), "gaming".to_string()],
            },
            InstanceTemplate {
                id: "commodore-64-like".to_string(),
                name: "Commodore 64-like System".to_string(),
                description: "6502 system with C64-like memory layout".to_string(),
                rom_image: vec![],
                reset_vector: 0xE000,
                initial_memory: {
                    let mut mem = HashMap::new();
                    mem.insert(0xFFFC, 0x00); // Reset vector low
                    mem.insert(0xFFFD, 0xE0); // Reset vector high
                    mem
                },
                emulator_type: EmulatorType::Standard,
                created_by: "system".to_string(),
                created_at: Utc::now(),
                is_public: true,
                tags: vec!["c64".to_string(), "commodore".to_string()],
            },
            InstanceTemplate {
                id: "development".to_string(),
                name: "Development System".to_string(),
                description: "Fast development environment with debugging support".to_string(),
                rom_image: vec![],
                reset_vector: 0x8000,
                initial_memory: {
                    let mut mem = HashMap::new();
                    mem.insert(0xFFFC, 0x00);
                    mem.insert(0xFFFD, 0x80);
                    // Add some development utilities
                    mem.insert(0x00, 0xEA); // NOP for debugging
                    mem
                },
                emulator_type: EmulatorType::Turbo,
                created_by: "system".to_string(),
                created_at: Utc::now(),
                is_public: true,
                tags: vec!["development".to_string(), "debugging".to_string(), "fast".to_string()],
            },
        ]
    }
}

impl EmulatorInstance {
    pub fn new(
        owner_id: String,
        emulator_type: EmulatorType,
        name: Option<String>,
        template_id: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        let specs = emulator_type.get_specs();
        let instance_name = name.unwrap_or_else(|| {
            format!("{}-{}", emulator_type.to_string(), uuid::Uuid::new_v4().to_string()[..8].to_string())
        });
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: instance_name,
            owner_id,
            emulator_type,
            template_id,
            state: InstanceState::Stopped,
            specs,
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
            last_activity: Utc::now(),
            tags: tags.unwrap_or_default(),
            usage_stats: UsageStats::default(),
        }
    }
    
    pub fn start(&mut self) {
        self.state = InstanceState::Running;
        self.started_at = Some(Utc::now());
        self.last_activity = Utc::now();
    }
    
    pub fn stop(&mut self) {
        self.state = InstanceState::Stopped;
        self.stopped_at = Some(Utc::now());
        self.last_activity = Utc::now();
    }
    
    pub fn pause(&mut self) {
        self.state = InstanceState::Paused;
        self.last_activity = Utc::now();
    }
    
    pub fn record_activity(&mut self) {
        self.last_activity = Utc::now();
    }
    
    pub fn record_cycle(&mut self) {
        self.usage_stats.total_cycles += 1;
    }
    
    pub fn record_instruction(&mut self) {
        self.usage_stats.total_instructions += 1;
    }
    
    pub fn record_api_call(&mut self) {
        self.usage_stats.api_calls += 1;
    }
    
    pub fn get_runtime_seconds(&self) -> u64 {
        if let Some(started) = self.started_at {
            let end_time = if matches!(self.state, InstanceState::Running) {
                Utc::now()
            } else {
                self.stopped_at.unwrap_or(Utc::now())
            };
            
            (end_time - started).num_seconds() as u64
        } else {
            0
        }
    }
    
    pub fn is_idle(&self, idle_threshold_minutes: i64) -> bool {
        let idle_duration = Utc::now() - self.last_activity;
        idle_duration.num_minutes() > idle_threshold_minutes
    }
    
    pub fn can_user_access(&self, user_id: &str, is_admin: bool) -> bool {
        is_admin || self.owner_id == user_id
    }
}