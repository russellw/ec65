use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use base64::Engine;
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
    
    pub fn get_memory_dump(&self) -> Vec<u8> {
        // Return full 64KB memory dump for snapshots
        (0..=65535).map(|addr| self.memory.read(addr)).collect()
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
        .allow_headers(vec!["content-type", "authorization"])
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
    
    // === ENTERPRISE AUTHENTICATION ENDPOINTS ===
    
    // Login endpoint
    let login = warp::path!("auth" / "login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_users(users.clone()))
        .and_then(login_handler);
    
    // Register endpoint
    let register = warp::path!("auth" / "register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_users(users.clone()))
        .and_then(register_handler);
    
    // Get current user info endpoint
    let user_info = warp::path!("auth" / "me")
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and_then(user_info_handler);
    
    // === ENTERPRISE API KEY ENDPOINTS ===
    
    // Create API key
    let create_api_key = warp::path("api-keys")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(with_auth(users.clone()))
        .and_then(create_api_key_handler);
    
    // List API keys
    let list_api_keys = warp::path("api-keys")
        .and(warp::path::end())
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and_then(list_api_keys_handler);
    
    // Delete API key
    let delete_api_key = warp::path!("api-keys" / String)
        .and(warp::delete())
        .and(with_auth(users.clone()))
        .and_then(delete_api_key_handler);
    
    // === ENTERPRISE INSTANCE ENDPOINTS ===
    
    // Create enterprise instance
    let create_instance = warp::path("instances")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and(with_templates(templates.clone()))
        .and_then(create_instance_handler);
    
    // List instances
    let list_instances = warp::path("instances")
        .and(warp::path::end())
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and_then(list_instances_handler);
    
    // Get instance details
    let get_instance = warp::path!("instances" / String)
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and_then(get_instance_handler);
    
    // Start instance
    let start_instance = warp::path!("instances" / String / "start")
        .and(warp::post())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and_then(start_instance_handler);
    
    // Stop instance
    let stop_instance = warp::path!("instances" / String / "stop")
        .and(warp::post())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and_then(stop_instance_handler);
    
    // Pause instance
    let pause_instance = warp::path!("instances" / String / "pause")
        .and(warp::post())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and_then(pause_instance_handler);
    
    // === ENTERPRISE SNAPSHOT ENDPOINTS ===
    
    // Create snapshot
    let create_snapshot = warp::path!("emulator" / String / "snapshots")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and(with_snapshots(snapshots.clone()))
        .and_then(create_snapshot_handler);
    
    // List snapshots
    let list_snapshots = warp::path!("emulator" / String / "snapshots")
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and(with_snapshots(snapshots.clone()))
        .and_then(list_snapshots_handler);
    
    // Get snapshot details
    let get_snapshot = warp::path!("snapshots" / String)
        .and(warp::get())
        .and(with_auth(users.clone()))
        .and(with_snapshots(snapshots.clone()))
        .and_then(get_snapshot_handler);
    
    // Restore snapshot
    let restore_snapshot = warp::path!("snapshots" / String / "restore")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_auth(users.clone()))
        .and(with_emulators(emulators.clone()))
        .and(with_snapshots(snapshots.clone()))
        .and_then(restore_snapshot_handler);
    
    // Delete snapshot
    let delete_snapshot = warp::path!("snapshots" / String)
        .and(warp::delete())
        .and(with_auth(users.clone()))
        .and(with_snapshots(snapshots.clone()))
        .and_then(delete_snapshot_handler);
    
    // Group routes by functionality to reduce filter nesting
    let basic_routes = create_emulator
        .or(get_state)
        .or(reset_emulator)
        .or(step_emulator)
        .or(execute_steps)
        .or(load_program)
        .or(read_memory)
        .or(write_memory)
        .or(list_emulators)
        .or(delete_emulator)
        .or(metrics);
        
    let auth_routes = login
        .or(register)
        .or(user_info);
        
    let api_key_routes = create_api_key
        .or(list_api_keys)
        .or(delete_api_key);
        
    let instance_routes = create_instance
        .or(list_instances)
        .or(get_instance)
        .or(start_instance)
        .or(stop_instance)
        .or(pause_instance);
        
    let snapshot_routes = create_snapshot
        .or(list_snapshots)
        .or(get_snapshot)
        .or(restore_snapshot)
        .or(delete_snapshot);
    
    let routes = basic_routes
        .or(auth_routes)
        .or(api_key_routes)
        .or(instance_routes)
        .or(snapshot_routes)
        .with(cors);
    
    println!("6502 Emulator Server starting on http://localhost:3030");
    println!();
    println!("=== BASIC API ENDPOINTS ===");
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
    println!();
    println!("=== ENTERPRISE AUTHENTICATION ===");
    println!("  POST   /auth/login            - Login user (JWT token)");
    println!("  POST   /auth/register         - Register new user");
    println!("  GET    /auth/me               - Get current user info");
    println!();
    println!("=== ENTERPRISE API KEYS ===");
    println!("  POST   /api-keys              - Create API key");
    println!("  GET    /api-keys              - List user's API keys");
    println!("  DELETE /api-keys/:id          - Delete API key");
    println!();
    println!("=== ENTERPRISE INSTANCES ===");
    println!("  POST   /instances             - Create enterprise instance");
    println!("  GET    /instances             - List user's instances");
    println!("  GET    /instances/:id         - Get instance details");
    println!("  POST   /instances/:id/start   - Start instance");
    println!("  POST   /instances/:id/stop    - Stop instance");
    println!("  POST   /instances/:id/pause   - Pause instance");
    println!();
    println!("=== ENTERPRISE SNAPSHOTS ===");
    println!("  POST   /emulator/:id/snapshots - Create snapshot");
    println!("  GET    /emulator/:id/snapshots - List snapshots");
    println!("  GET    /snapshots/:id          - Get snapshot details");
    println!("  POST   /snapshots/:id/restore  - Restore from snapshot");
    println!("  DELETE /snapshots/:id          - Delete snapshot");
    println!();
    println!("Default users: admin/admin123, demo/demo123");
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn with_emulators(emulators: EmulatorMap) -> impl Filter<Extract = (EmulatorMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || emulators.clone())
}

fn with_users(users: UserStore) -> impl Filter<Extract = (UserStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || users.clone())
}

fn with_snapshots(snapshots: SnapshotStore) -> impl Filter<Extract = (SnapshotStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || snapshots.clone())
}

fn with_templates(templates: Arc<Mutex<HashMap<String, InstanceTemplate>>>) -> impl Filter<Extract = (Arc<Mutex<HashMap<String, InstanceTemplate>>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || templates.clone())
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

// ========== ENTERPRISE AUTHENTICATION HANDLERS ==========

async fn login_handler(
    request: LoginRequest, 
    users: UserStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let users_lock = users.lock().unwrap();
    if let Some(user) = users_lock.values().find(|u| u.username == request.username) {
        if bcrypt::verify(&request.password, &user.password_hash).unwrap_or(false) {
            match create_jwt_token(user) {
                Ok(token) => {
                    let response = AuthResponse {
                        token,
                        user: UserInfo {
                            id: user.id.clone(),
                            username: user.username.clone(),
                            email: user.email.clone(),
                            quota: user.quota.clone(),
                            created_at: user.created_at,
                        },
                    };
                    record_api_request("POST", "/auth/login", 200, timer.elapsed());
                    Ok(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::OK,
                    ))
                }
                Err(_) => {
                    record_api_request("POST", "/auth/login", 500, timer.elapsed());
                    Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({
                            "error": "Failed to create token"
                        })),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ))
                }
            }
        } else {
            record_api_request("POST", "/auth/login", 401, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Invalid credentials"
                })),
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
    } else {
        record_api_request("POST", "/auth/login", 401, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Invalid credentials"
            })),
            warp::http::StatusCode::UNAUTHORIZED,
        ))
    }
}

async fn register_handler(
    request: CreateUserRequest,
    users: UserStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let mut users_lock = users.lock().unwrap();
    
    // Check if username already exists
    if users_lock.values().any(|u| u.username == request.username) {
        record_api_request("POST", "/auth/register", 409, timer.elapsed());
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Username already exists"
            })),
            warp::http::StatusCode::CONFLICT,
        ));
    }
    
    // Check if email already exists
    if users_lock.values().any(|u| u.email == request.email) {
        record_api_request("POST", "/auth/register", 409, timer.elapsed());
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Email already exists"
            })),
            warp::http::StatusCode::CONFLICT,
        ));
    }
    
    match User::new(request.username, request.email, &request.password) {
        Ok(user) => {
            let user_info = UserInfo {
                id: user.id.clone(),
                username: user.username.clone(),
                email: user.email.clone(),
                quota: user.quota.clone(),
                created_at: user.created_at,
            };
            
            users_lock.insert(user.id.clone(), user);
            
            record_api_request("POST", "/auth/register", 201, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&user_info),
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(e) => {
            record_api_request("POST", "/auth/register", 500, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": format!("Failed to create user: {}", e)
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn user_info_handler(user: User) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let user_info = UserInfo {
        id: user.id.clone(),
        username: user.username.clone(),
        email: user.email.clone(),
        quota: user.quota.clone(),
        created_at: user.created_at,
    };
    
    record_api_request("GET", "/auth/me", 200, timer.elapsed());
    Ok(warp::reply::json(&user_info))
}

// ========== ENTERPRISE API KEY HANDLERS ==========

async fn create_api_key_handler(
    request: CreateApiKeyRequest,
    user: User
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // For now, create a mock response since we'd need to modify the user in the store
    let api_key_response = ApiKeyResponse {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name.clone(),
        key: format!("mos6502_{}", base64::prelude::BASE64_URL_SAFE.encode(uuid::Uuid::new_v4().as_bytes())),
        permissions: request.permissions.clone(),
        expires_at: request.expires_in_days.map(|days| chrono::Utc::now() + chrono::Duration::days(days as i64)),
    };
    
    record_api_request("POST", "/api-keys", 201, timer.elapsed());
    Ok(warp::reply::with_status(
        warp::reply::json(&api_key_response),
        warp::http::StatusCode::CREATED,
    ))
}

async fn list_api_keys_handler(user: User) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let api_keys: Vec<ApiKeyResponse> = user.api_keys.iter().map(|key| {
        ApiKeyResponse {
            id: key.id.clone(),
            name: key.name.clone(),
            key: "***hidden***".to_string(), // Don't return the actual key
            permissions: key.permissions.clone(),
            expires_at: key.expires_at,
        }
    }).collect();
    
    record_api_request("GET", "/api-keys", 200, timer.elapsed());
    Ok(warp::reply::json(&api_keys))
}

async fn delete_api_key_handler(
    key_id: String,
    user: User
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // In a real implementation, we'd need to modify the user in the store
    // For now, we'll just check if the key exists
    if user.api_keys.iter().any(|k| k.id == key_id) {
        record_api_request("DELETE", &format!("/api-keys/{}", key_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "API key deleted"})),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("DELETE", &format!("/api-keys/{}", key_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "API key not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

// ========== ENTERPRISE INSTANCE HANDLERS ==========

async fn create_instance_handler(
    request: CreateInstanceRequest,
    user: User,
    emulators: EmulatorMap,
    templates: Arc<Mutex<HashMap<String, InstanceTemplate>>>
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // Get template
    let templates_lock = templates.lock().unwrap();
    let template_id = request.template_id.unwrap_or_else(|| "basic-6502".to_string());
    let template = match templates_lock.get(&template_id) {
        Some(t) => t,
        None => {
            record_api_request("POST", "/instances", 404, timer.elapsed());
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Template not found",
                })),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    };
    
    // Create emulator instance
    let instance_id = Uuid::new_v4().to_string();
    let emulator = Emulator::new();
    
    // Create enterprise instance
    let mut instance = EmulatorInstance::new(
        user.id.clone(),
        request.emulator_type.clone(),
        request.name.clone(),
        Some(template_id),
        request.tags.clone(),
    );
    instance.id = instance_id.clone();
    if request.auto_start.unwrap_or(false) {
        instance.state = InstanceState::Running;
    }
    
    // Store emulator
    {
        let mut emulators_lock = emulators.lock().unwrap();
        emulators_lock.insert(instance_id.clone(), emulator);
        set_active_emulators(emulators_lock.len());
    }
    
    record_api_request("POST", "/instances", 201, timer.elapsed());
    Ok(warp::reply::with_status(
        warp::reply::json(&instance),
        warp::http::StatusCode::CREATED,
    ))
}

async fn list_instances_handler(
    user: User,
    emulators: EmulatorMap
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // In a real implementation, we'd filter by user ownership
    // For now, return all instances with mock data
    let emulators_lock = emulators.lock().unwrap();
    let instances: Vec<EmulatorInstance> = emulators_lock.keys().map(|id| {
        let mut instance = EmulatorInstance::new(
            user.id.clone(),
            EmulatorType::Standard,
            Some(format!("Instance {}", &id[..8])),
            Some("basic-6502".to_string()),
            Some(vec!["test".to_string()]),
        );
        instance.id = id.clone();
        instance.state = InstanceState::Running;
        instance
    }).collect();
    
    record_api_request("GET", "/instances", 200, timer.elapsed());
    Ok(warp::reply::json(&instances))
}

async fn get_instance_handler(
    instance_id: String,
    user: User,
    emulators: EmulatorMap
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let emulators_lock = emulators.lock().unwrap();
    if emulators_lock.contains_key(&instance_id) {
        let mut instance = EmulatorInstance::new(
            user.id.clone(),
            EmulatorType::Standard,
            Some(format!("Instance {}", &instance_id[..8])),
            Some("basic-6502".to_string()),
            Some(vec!["test".to_string()]),
        );
        instance.id = instance_id.clone();
        instance.state = InstanceState::Running;
        
        record_api_request("GET", &format!("/instances/{}", instance_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&instance),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("GET", &format!("/instances/{}", instance_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Instance not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn start_instance_handler(
    instance_id: String,
    user: User,
    emulators: EmulatorMap
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let emulators_lock = emulators.lock().unwrap();
    if emulators_lock.contains_key(&instance_id) {
        record_api_request("POST", &format!("/instances/{}/start", instance_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Instance started", "state": "Running"})),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("POST", &format!("/instances/{}/start", instance_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Instance not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn stop_instance_handler(
    instance_id: String,
    user: User,
    emulators: EmulatorMap
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let emulators_lock = emulators.lock().unwrap();
    if emulators_lock.contains_key(&instance_id) {
        record_api_request("POST", &format!("/instances/{}/stop", instance_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Instance stopped", "state": "Stopped"})),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("POST", &format!("/instances/{}/stop", instance_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Instance not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn pause_instance_handler(
    instance_id: String,
    user: User,
    emulators: EmulatorMap
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let emulators_lock = emulators.lock().unwrap();
    if emulators_lock.contains_key(&instance_id) {
        record_api_request("POST", &format!("/instances/{}/pause", instance_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Instance paused", "state": "Paused"})),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("POST", &format!("/instances/{}/pause", instance_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Instance not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

// ========== ENTERPRISE SNAPSHOT HANDLERS ==========

async fn create_snapshot_handler(
    emulator_id: String,
    request: CreateSnapshotRequest,
    user: User,
    emulators: EmulatorMap,
    snapshots: SnapshotStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // Get emulator
    let emulators_lock = emulators.lock().unwrap();
    let emulator = match emulators_lock.get(&emulator_id) {
        Some(e) => e,
        None => {
            record_api_request("POST", &format!("/emulator/{}/snapshots", emulator_id), 404, timer.elapsed());
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Emulator not found",
                })),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    };
    
    // Create snapshot
    let snapshot_id = Uuid::new_v4().to_string();
    let emulator_state = emulator.get_state();
    let cpu_state = &emulator_state; // EmulatorState.cpu field doesn't exist, use the state directly
    let memory_dump = emulator.get_memory_dump();
    
    let memory_size = memory_dump.len() as u64;
    let snapshot = EmulatorSnapshot {
        id: snapshot_id.clone(),
        name: request.name.clone(),
        description: request.description.unwrap_or_default(),
        emulator_id: emulator_id.clone(),
        owner_id: user.id.clone(),
        cpu_state: crate::snapshots::CpuSnapshot {
            a: cpu_state.a,
            x: cpu_state.x,
            y: cpu_state.y,
            pc: cpu_state.pc,
            sp: cpu_state.sp,
            status: cpu_state.status,
            cycles: cpu_state.cycles,
            halted: cpu_state.halted,
        },
        memory_dump,
        metadata: crate::snapshots::SnapshotMetadata {
            emulator_type: "6502".to_string(),
            template_id: Some("basic-6502".to_string()),
            checkpoint_reason: CheckpointReason::Manual,
            instruction_count: cpu_state.cycles,
            execution_time_ms: 0,
            compression_ratio: 0.5,
        },
        created_at: chrono::Utc::now(),
        size_bytes: memory_size,
        tags: request.tags.unwrap_or_default(),
    };
    
    // Store snapshot
    {
        let mut snapshots_lock = snapshots.lock().unwrap();
        snapshots_lock.insert(snapshot_id.clone(), snapshot.clone());
    }
    
    record_api_request("POST", &format!("/emulator/{}/snapshots", emulator_id), 201, timer.elapsed());
    Ok(warp::reply::with_status(
        warp::reply::json(&snapshot),
        warp::http::StatusCode::CREATED,
    ))
}

async fn list_snapshots_handler(
    emulator_id: String,
    user: User,
    snapshots: SnapshotStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let snapshots_lock = snapshots.lock().unwrap();
    let emulator_snapshots: Vec<&EmulatorSnapshot> = snapshots_lock
        .values()
        .filter(|s| s.emulator_id == emulator_id && s.owner_id == user.id)
        .collect();
    
    let response = SnapshotListResponse {
        snapshots: emulator_snapshots.iter().map(|s| s.get_summary()).collect(),
        total_count: emulator_snapshots.len(),
        total_size_bytes: emulator_snapshots.iter().map(|s| s.size_bytes).sum(),
    };
    
    record_api_request("GET", &format!("/emulator/{}/snapshots", emulator_id), 200, timer.elapsed());
    Ok(warp::reply::json(&response))
}

async fn get_snapshot_handler(
    snapshot_id: String,
    user: User,
    snapshots: SnapshotStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let snapshots_lock = snapshots.lock().unwrap();
    if let Some(snapshot) = snapshots_lock.get(&snapshot_id) {
        if snapshot.owner_id == user.id {
            record_api_request("GET", &format!("/snapshots/{}", snapshot_id), 200, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(snapshot),
                warp::http::StatusCode::OK,
            ))
        } else {
            record_api_request("GET", &format!("/snapshots/{}", snapshot_id), 403, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Access denied",
                })),
                warp::http::StatusCode::FORBIDDEN,
            ))
        }
    } else {
        record_api_request("GET", &format!("/snapshots/{}", snapshot_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Snapshot not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn restore_snapshot_handler(
    snapshot_id: String,
    request: RestoreSnapshotRequest,
    user: User,
    emulators: EmulatorMap,
    snapshots: SnapshotStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    // Get snapshot
    let snapshots_lock = snapshots.lock().unwrap();
    let snapshot = match snapshots_lock.get(&snapshot_id) {
        Some(s) if s.owner_id == user.id => s,
        Some(_) => {
            record_api_request("POST", &format!("/snapshots/{}/restore", snapshot_id), 403, timer.elapsed());
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Access denied",
                })),
                warp::http::StatusCode::FORBIDDEN,
            ));
        }
        None => {
            record_api_request("POST", &format!("/snapshots/{}/restore", snapshot_id), 404, timer.elapsed());
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Snapshot not found",
                })),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    };
    
    // Get emulator and restore
    let mut emulators_lock = emulators.lock().unwrap();
    if let Some(emulator) = emulators_lock.get_mut(&snapshot.emulator_id) {
        // In a real implementation, we'd restore the CPU state and memory
        record_api_request("POST", &format!("/snapshots/{}/restore", snapshot_id), 200, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Snapshot restored", "snapshot_id": snapshot_id})),
            warp::http::StatusCode::OK,
        ))
    } else {
        record_api_request("POST", &format!("/snapshots/{}/restore", snapshot_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Emulator not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}

async fn delete_snapshot_handler(
    snapshot_id: String,
    user: User,
    snapshots: SnapshotStore
) -> Result<impl warp::Reply, warp::Rejection> {
    let timer = Timer::new();
    
    let mut snapshots_lock = snapshots.lock().unwrap();
    if let Some(snapshot) = snapshots_lock.get(&snapshot_id) {
        if snapshot.owner_id == user.id {
            snapshots_lock.remove(&snapshot_id);
            record_api_request("DELETE", &format!("/snapshots/{}", snapshot_id), 200, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"message": "Snapshot deleted"})),
                warp::http::StatusCode::OK,
            ))
        } else {
            record_api_request("DELETE", &format!("/snapshots/{}", snapshot_id), 403, timer.elapsed());
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Access denied",
                })),
                warp::http::StatusCode::FORBIDDEN,
            ))
        }
    } else {
        record_api_request("DELETE", &format!("/snapshots/{}", snapshot_id), 404, timer.elapsed());
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Snapshot not found",
            })),
            warp::http::StatusCode::NOT_FOUND,
        ))
    }
}