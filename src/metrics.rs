use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};
use std::time::{Duration, Instant};

lazy_static! {
    /// Global Prometheus registry
    pub static ref REGISTRY: Registry = Registry::new();
    
    /// Counter for total CPU instructions executed by opcode
    pub static ref CPU_INSTRUCTIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("cpu_instructions_total", "Total number of CPU instructions executed by opcode"),
        &["opcode", "instruction"]
    ).expect("Failed to create CPU instructions counter");
    
    /// Counter for CPU cycles executed
    pub static ref CPU_CYCLES_TOTAL: Counter = Counter::new(
        "cpu_cycles_total", "Total number of CPU cycles executed"
    ).expect("Failed to create CPU cycles counter");
    
    /// Histogram for instruction execution time
    pub static ref INSTRUCTION_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("instruction_duration_seconds", "Time spent executing instructions")
            .buckets(vec![0.000001, 0.000005, 0.00001, 0.00005, 0.0001, 0.0005, 0.001]),
        &["instruction"]
    ).expect("Failed to create instruction duration histogram");
    
    /// Counter for API requests by endpoint and method
    pub static ref API_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("api_requests_total", "Total number of API requests"),
        &["method", "endpoint", "status"]
    ).expect("Failed to create API requests counter");
    
    /// Histogram for API request duration
    pub static ref API_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("api_request_duration_seconds", "API request duration")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        &["method", "endpoint"]
    ).expect("Failed to create API request duration histogram");
    
    /// Gauge for active emulator instances
    pub static ref ACTIVE_EMULATORS: Gauge = Gauge::new(
        "active_emulators_total", "Number of active emulator instances"
    ).expect("Failed to create active emulators gauge");
    
    /// Gauge for CPU register values by emulator ID
    pub static ref CPU_REGISTER_VALUES: GaugeVec = GaugeVec::new(
        Opts::new("cpu_register_value", "Current CPU register values"),
        &["emulator_id", "register"]
    ).expect("Failed to create CPU register values gauge");
    
    /// Counter for memory operations
    pub static ref MEMORY_OPERATIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("memory_operations_total", "Total memory read/write operations"),
        &["operation", "emulator_id"]
    ).expect("Failed to create memory operations counter");
    
    /// Gauge for CPU flags by emulator ID
    pub static ref CPU_FLAGS: GaugeVec = GaugeVec::new(
        Opts::new("cpu_flags", "Current CPU flag states (0 or 1)"),
        &["emulator_id", "flag"]
    ).expect("Failed to create CPU flags gauge");
    
    /// Counter for emulator resets
    pub static ref EMULATOR_RESETS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("emulator_resets_total", "Total number of emulator resets"),
        &["emulator_id"]
    ).expect("Failed to create emulator resets counter");
    
    /// Counter for program loads
    pub static ref PROGRAM_LOADS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("program_loads_total", "Total number of programs loaded"),
        &["emulator_id"]
    ).expect("Failed to create program loads counter");
}

/// Initialize Prometheus metrics by registering them with the global registry
pub fn init_metrics() {
    REGISTRY
        .register(Box::new(CPU_INSTRUCTIONS_TOTAL.clone()))
        .expect("Failed to register CPU instructions counter");
    
    REGISTRY
        .register(Box::new(CPU_CYCLES_TOTAL.clone()))
        .expect("Failed to register CPU cycles counter");
    
    REGISTRY
        .register(Box::new(INSTRUCTION_DURATION.clone()))
        .expect("Failed to register instruction duration histogram");
    
    REGISTRY
        .register(Box::new(API_REQUESTS_TOTAL.clone()))
        .expect("Failed to register API requests counter");
    
    REGISTRY
        .register(Box::new(API_REQUEST_DURATION.clone()))
        .expect("Failed to register API request duration histogram");
    
    REGISTRY
        .register(Box::new(ACTIVE_EMULATORS.clone()))
        .expect("Failed to register active emulators gauge");
    
    REGISTRY
        .register(Box::new(CPU_REGISTER_VALUES.clone()))
        .expect("Failed to register CPU register values gauge");
    
    REGISTRY
        .register(Box::new(MEMORY_OPERATIONS_TOTAL.clone()))
        .expect("Failed to register memory operations counter");
    
    REGISTRY
        .register(Box::new(CPU_FLAGS.clone()))
        .expect("Failed to register CPU flags gauge");
    
    REGISTRY
        .register(Box::new(EMULATOR_RESETS_TOTAL.clone()))
        .expect("Failed to register emulator resets counter");
    
    REGISTRY
        .register(Box::new(PROGRAM_LOADS_TOTAL.clone()))
        .expect("Failed to register program loads counter");
}

/// Record a CPU instruction execution
pub fn record_instruction(opcode: u8, instruction_name: &str, duration: Duration) {
    CPU_INSTRUCTIONS_TOTAL
        .with_label_values(&[&format!("0x{:02X}", opcode), instruction_name])
        .inc();
    
    CPU_CYCLES_TOTAL.inc();
    
    INSTRUCTION_DURATION
        .with_label_values(&[instruction_name])
        .observe(duration.as_secs_f64());
}

/// Record an API request
pub fn record_api_request(method: &str, endpoint: &str, status: u16, duration: Duration) {
    API_REQUESTS_TOTAL
        .with_label_values(&[method, endpoint, &status.to_string()])
        .inc();
    
    API_REQUEST_DURATION
        .with_label_values(&[method, endpoint])
        .observe(duration.as_secs_f64());
}

/// Update emulator count
pub fn set_active_emulators(count: usize) {
    ACTIVE_EMULATORS.set(count as f64);
}

/// Update CPU register metrics for an emulator
pub fn update_cpu_registers(emulator_id: &str, a: u8, x: u8, y: u8, pc: u16, sp: u8, status: u8) {
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "A"])
        .set(a as f64);
    
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "X"])
        .set(x as f64);
    
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "Y"])
        .set(y as f64);
    
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "PC"])
        .set(pc as f64);
    
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "SP"])
        .set(sp as f64);
    
    CPU_REGISTER_VALUES
        .with_label_values(&[emulator_id, "STATUS"])
        .set(status as f64);
    
    // Update individual flag states
    update_cpu_flags(emulator_id, status);
}

/// Update CPU flag metrics for an emulator
pub fn update_cpu_flags(emulator_id: &str, status: u8) {
    CPU_FLAGS
        .with_label_values(&[emulator_id, "carry"])
        .set(if status & 0x01 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "zero"])
        .set(if status & 0x02 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "interrupt_disable"])
        .set(if status & 0x04 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "decimal_mode"])
        .set(if status & 0x08 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "break_command"])
        .set(if status & 0x10 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "overflow"])
        .set(if status & 0x40 != 0 { 1.0 } else { 0.0 });
    
    CPU_FLAGS
        .with_label_values(&[emulator_id, "negative"])
        .set(if status & 0x80 != 0 { 1.0 } else { 0.0 });
}

/// Record a memory operation
pub fn record_memory_operation(operation: &str, emulator_id: &str) {
    MEMORY_OPERATIONS_TOTAL
        .with_label_values(&[operation, emulator_id])
        .inc();
}

/// Record an emulator reset
pub fn record_emulator_reset(emulator_id: &str) {
    EMULATOR_RESETS_TOTAL
        .with_label_values(&[emulator_id])
        .inc();
}

/// Record a program load
pub fn record_program_load(emulator_id: &str) {
    PROGRAM_LOADS_TOTAL
        .with_label_values(&[emulator_id])
        .inc();
}

/// Helper struct for timing operations
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Get opcode name for metrics
pub fn get_instruction_name(opcode: u8) -> &'static str {
    match opcode {
        // Load instructions
        0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => "LDA",
        0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => "LDX",
        0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => "LDY",
        
        // Store instructions
        0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => "STA",
        0x86 | 0x96 | 0x8E => "STX",
        0x84 | 0x94 | 0x8C => "STY",
        
        // Arithmetic
        0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => "ADC",
        0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => "SBC",
        
        // Compare
        0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => "CMP",
        0xE0 | 0xE4 | 0xEC => "CPX",
        0xC0 | 0xC4 | 0xCC => "CPY",
        
        // Logical
        0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => "AND",
        0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => "ORA",
        0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => "EOR",
        
        // Increment/Decrement
        0xE6 | 0xF6 | 0xEE | 0xFE => "INC",
        0xC6 | 0xD6 | 0xCE | 0xDE => "DEC",
        0xE8 => "INX",
        0xC8 => "INY",
        0xCA => "DEX",
        0x88 => "DEY",
        
        // Transfer
        0xAA => "TAX",
        0xA8 => "TAY",
        0x8A => "TXA",
        0x98 => "TYA",
        0xBA => "TSX",
        0x9A => "TXS",
        
        // Jump/Call
        0x4C | 0x6C => "JMP",
        0x20 => "JSR",
        0x60 => "RTS",
        
        // Flag manipulation
        0x18 => "CLC",
        0x38 => "SEC",
        0x58 => "CLI",
        0x78 => "SEI",
        0xD8 => "CLD",
        0xF8 => "SED",
        0xB8 => "CLV",
        
        // Branch
        0x90 => "BCC",
        0xB0 => "BCS",
        0xF0 => "BEQ",
        0xD0 => "BNE",
        0x30 => "BMI",
        0x10 => "BPL",
        0x50 => "BVC",
        0x70 => "BVS",
        
        // Other
        0x00 => "BRK",
        0xEA => "NOP",
        
        _ => "UNKNOWN",
    }
}