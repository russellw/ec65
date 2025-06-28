use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{Filter, reject};
use prometheus::Encoder;

use crate::cpu::CPU;
use crate::memory::Memory;
use crate::metrics::{
    init_metrics, record_api_request, set_active_emulators, update_cpu_registers,
    record_memory_operation, record_emulator_reset, record_program_load, Timer, REGISTRY
};
use crate::auth::{
    User, UserStore, init_default_users, with_auth, with_permission, Permission,
    LoginRequest, CreateUserRequest, CreateApiKeyRequest, AuthResponse, UserInfo,
    ApiKeyResponse, create_jwt_token, AuthError,
};
use crate::instance_types::{
    EmulatorType, EmulatorInstance, InstanceTemplate, CreateInstanceRequest,
    InstanceState, UsageStats,
};
use crate::snapshots::{
    EmulatorSnapshot, SnapshotStore, CreateSnapshotRequest, RestoreSnapshotRequest,
    SnapshotListResponse, CheckpointReason,
};

#[derive(Debug, Clone, Serialize)]
pub struct CpuState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub sp: u8,
    pub status: u8,
    pub cycles: u64,
    pub halted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmulatorState {
    pub id: String,
    pub cpu: CpuState,
}

#[derive(Debug, Deserialize)]
pub struct MemoryWrite {
    pub address: u16,
    pub value: u8,
}

#[derive(Debug, Deserialize)]
pub struct MemoryRead {
    pub address: u16,
    pub length: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct MemoryData {
    pub address: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct ProgramLoad {
    pub address: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteSteps {
    pub steps: u32,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub steps_executed: u32,
    pub halted: bool,
    pub final_state: CpuState,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

pub struct Emulator {
    pub cpu: CPU,
    pub memory: Memory,
    pub cycles: u64,
    pub instance: EmulatorInstance,
    pub last_cycle_time: std::time::Instant,
}

impl Emulator {
    pub fn new_with_instance(instance: EmulatorInstance) -> Self {
        Self {
            cpu: CPU::new(),
            memory: Memory::new(),
            cycles: 0,
            instance,
            last_cycle_time: std::time::Instant::now(),
        }
    }
    
    pub fn new() -> Self {
        // Default instance for backward compatibility
        let default_instance = EmulatorInstance::new(
            "system".to_string(),
            EmulatorType::Standard,
            Some("default".to_string()),
            None,
            None,
        );
        Self::new_with_instance(default_instance)
    }
    
    pub fn get_state(&self) -> CpuState {
        CpuState {
            a: self.cpu.get_register_a(),
            x: self.cpu.get_register_x(),
            y: self.cpu.get_register_y(),
            pc: self.cpu.get_pc(),
            sp: self.cpu.get_sp(),
            status: self.cpu.get_status(),
            cycles: self.cycles,
            halted: self.cpu.is_halted(),
        }
    }
    
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.memory);
        self.cycles = 0;
    }
    
    pub fn step(&mut self) -> bool {
        if !self.cpu.is_halted() {
            self.cpu.step(&mut self.memory);
            self.cycles += 1;
            true
        } else {
            false
        }
    }
    
    pub fn execute_steps(&mut self, steps: u32) -> ExecutionResult {
        let mut executed = 0;
        
        for _ in 0..steps {
            if !self.step() {
                break;
            }
            executed += 1;
        }
        
        ExecutionResult {
            steps_executed: executed,
            halted: self.cpu.is_halted(),
            final_state: self.get_state(),
        }
    }
    
    pub fn load_program(&mut self, address: u16, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.memory.write(address + i as u16, byte);
        }
    }
    
    pub fn read_memory(&self, address: u16, length: u16) -> Vec<u8> {
        (0..length).map(|i| self.memory.read(address + i)).collect()
    }
    
    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }
    
    pub fn get_id(&self) -> String {
        // For metrics, we'll pass this from the server context
        "unknown".to_string()
    }
}

type EmulatorMap = Arc<Mutex<HashMap<String, Emulator>>>;

pub async fn run_server() {
    // Initialize Prometheus metrics
    init_metrics();
    
    // Initialize stores
    let emulators: EmulatorMap = Arc::new(Mutex::new(HashMap::new()));
    let users: UserStore = Arc::new(Mutex::new(HashMap::new()));
    let snapshots: SnapshotStore = Arc::new(Mutex::new(HashMap::new()));
    let templates: Arc<Mutex<HashMap<String, InstanceTemplate>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // Initialize default users and templates
    init_default_users(users.clone());
    init_default_templates(templates.clone());
    
    println!("=== 6502 Cloud Computing Platform ===");
    println!("Enterprise-grade 6502 emulation service starting...");
    
    // CORS
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);
    
    // Create new emulator instance
    let create_emulator = warp::path("emulator")
        .and(warp::path::end())
        .and(warp::post())
        .and(with_emulators(emulators.clone()))
        .and_then(create_emulator_handler);
    
    // Get emulator state
    let get_state = warp::path!("emulator" / String)
        .and(warp::get())
        .and(with_emulators(emulators.clone()))
        .and_then(get_state_handler);
    
    // Reset emulator
    let reset_emulator = warp::path!("emulator" / String / "reset")
        .and(warp::post())
        .and(with_emulators(emulators.clone()))
        .and_then(reset_handler);
    
    // Step execution
    let step_emulator = warp::path!("emulator" / String / "step")
        .and(warp::post())
        .and(with_emulators(emulators.clone()))
        .and_then(step_handler);
    
    // Execute multiple steps
    let execute_steps = warp::path!("emulator" / String / "execute")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_emulators(emulators.clone()))
        .and_then(execute_handler);
    
    // Load program
    let load_program = warp::path!("emulator" / String / "program")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_emulators(emulators.clone()))
        .and_then(load_program_handler);
    
    // Read memory
    let read_memory = warp::path!("emulator" / String / "memory")
        .and(warp::get())
        .and(warp::query::<MemoryRead>())
        .and(with_emulators(emulators.clone()))
        .and_then(read_memory_handler);
    
    // Write memory
    let write_memory = warp::path!("emulator" / String / "memory")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_emulators(emulators.clone()))
        .and_then(write_memory_handler);
    
    // List emulators
    let list_emulators = warp::path("emulators")
        .and(warp::path::end())
        .and(warp::get())
        .and(with_emulators(emulators.clone()))
        .and_then(list_emulators_handler);
    
    // Delete emulator
    let delete_emulator = warp::path!("emulator" / String)
        .and(warp::delete())
        .and(with_emulators(emulators.clone()))
        .and_then(delete_emulator_handler);
    
    // Metrics endpoint
    let metrics = warp::path("metrics")
        .and(warp::path::end())
        .and(warp::get())
        .and_then(metrics_handler);
    
    let routes = create_emulator
        .or(get_state)
        .or(reset_emulator)
        .or(step_emulator)
        .or(execute_steps)
        .or(load_program)
        .or(read_memory)
        .or(write_memory)
        .or(list_emulators)
        .or(delete_emulator)
        .or(metrics)
        .with(cors);
    
    println!("6502 Emulator Server starting on http://localhost:3030");
    println!("API Documentation:");
    println!("  POST   /emulator              - Create new emulator instance");
    println!("  GET    /emulator/:id          - Get emulator state");
    println!("  POST   /emulator/:id/reset    - Reset emulator");
    println!("  POST   /emulator/:id/step     - Execute single step");
    println!("  POST   /emulator/:id/execute  - Execute multiple steps");
    println!("  POST   /emulator/:id/program  - Load program");
    println!("  GET    /emulator/:id/memory   - Read memory");
    println!("  POST   /emulator/:id/memory   - Write memory");
    println!("  GET    /emulators             - List all emulator instances");
    println!("  DELETE /emulator/:id          - Delete emulator instance");
    println!("  GET    /metrics               - Prometheus metrics endpoint");
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn with_emulators(emulators: EmulatorMap) -> impl Filter<Extract = (EmulatorMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || emulators.clone())
}

async fn create_emulator_handler(emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    let id = Uuid::new_v4().to_string();
    let emulator = Emulator::new();
    let state = emulator.get_state();
    
    {
        let mut emulators_lock = emulators.lock().unwrap();
        emulators_lock.insert(id.clone(), emulator);
        set_active_emulators(emulators_lock.len());
    }
    
    // Update CPU metrics for the new emulator
    update_cpu_registers(&id, state.a, state.x, state.y, state.pc, state.sp, state.status);
    
    let response = ApiResponse::success(EmulatorState {
        id,
        cpu: state,
    });
    
    record_api_request("POST", "/emulator", 200, timer.elapsed());
    Ok(warp::reply::json(&response))
}

async fn get_state_handler(id: String, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get(&id) {
        Some(emulator) => {
            let response = ApiResponse::success(EmulatorState {
                id: id.clone(),
                cpu: emulator.get_state(),
            });
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<EmulatorState> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn reset_handler(id: String, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            emulator.reset();
            let response = ApiResponse::success(EmulatorState {
                id: id.clone(),
                cpu: emulator.get_state(),
            });
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<EmulatorState> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn step_handler(id: String, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    let mut emulators_lock = emulators.lock().unwrap();
    
    let result = match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            emulator.step();
            let state = emulator.get_state();
            
            // Update CPU metrics
            update_cpu_registers(&id, state.a, state.x, state.y, state.pc, state.sp, state.status);
            
            let response = ApiResponse::success(EmulatorState {
                id: id.clone(),
                cpu: state,
            });
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<EmulatorState> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    };
    
    record_api_request("POST", "/emulator/:id/step", 200, timer.elapsed());
    result
}

async fn execute_handler(id: String, request: ExecuteSteps, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            let result = emulator.execute_steps(request.steps);
            let response = ApiResponse::success(result);
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<ExecutionResult> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn load_program_handler(id: String, request: ProgramLoad, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            emulator.load_program(request.address, &request.data);
            let response = ApiResponse::success(format!("Loaded {} bytes at address ${:04X}", request.data.len(), request.address));
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn read_memory_handler(id: String, query: MemoryRead, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get(&id) {
        Some(emulator) => {
            let length = query.length.unwrap_or(1);
            let data = emulator.read_memory(query.address, length);
            let response = ApiResponse::success(MemoryData {
                address: query.address,
                data,
            });
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<MemoryData> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn write_memory_handler(id: String, request: MemoryWrite, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            emulator.write_memory(request.address, request.value);
            let response = ApiResponse::success(format!("Wrote ${:02X} to address ${:04X}", request.value, request.address));
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}

async fn list_emulators_handler(emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let emulators_lock = emulators.lock().unwrap();
    
    let emulator_list: Vec<EmulatorState> = emulators_lock
        .iter()
        .map(|(id, emulator)| EmulatorState {
            id: id.clone(),
            cpu: emulator.get_state(),
        })
        .collect();
    
    let response = ApiResponse::success(emulator_list);
    Ok(warp::reply::json(&response))
}

async fn delete_emulator_handler(id: String, emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    let mut emulators_lock = emulators.lock().unwrap();
    
    let result = match emulators_lock.remove(&id) {
        Some(_) => {
            set_active_emulators(emulators_lock.len());
            let response = ApiResponse::success(format!("Emulator {} deleted", id));
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    };
    
    record_api_request("DELETE", "/emulator/:id", 200, timer.elapsed());
    result
}

async fn metrics_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(metrics_text) => {
            record_api_request("GET", "/metrics", 200, timer.elapsed());
            Ok(warp::reply::with_header(
                metrics_text,
                "content-type",
                "text/plain; version=0.0.4",
            ))
        }
        Err(_) => {
            record_api_request("GET", "/metrics", 500, timer.elapsed());
            Ok(warp::reply::with_header(
                "Error encoding metrics".to_string(),
                "content-type",
                "text/plain",
            ))
        }
    }
}

fn init_default_templates(templates: Arc<Mutex<HashMap<String, InstanceTemplate>>>) {
    let mut templates_lock = templates.lock().unwrap();
    let default_templates = InstanceTemplate::create_basic_templates();
    
    for template in default_templates {
        templates_lock.insert(template.id.clone(), template);
    }
    
    println!("Initialized {} default instance templates", templates_lock.len());
}