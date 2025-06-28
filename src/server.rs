use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::Filter;

use crate::cpu::CPU;
use crate::memory::Memory;

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
}

impl Emulator {
    pub fn new() -> Self {
        Self {
            cpu: CPU::new(),
            memory: Memory::new(),
            cycles: 0,
        }
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
}

type EmulatorMap = Arc<Mutex<HashMap<String, Emulator>>>;

pub async fn run_server() {
    let emulators: EmulatorMap = Arc::new(Mutex::new(HashMap::new()));
    
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
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn with_emulators(emulators: EmulatorMap) -> impl Filter<Extract = (EmulatorMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || emulators.clone())
}

async fn create_emulator_handler(emulators: EmulatorMap) -> Result<impl warp::Reply, warp::Rejection> {
    let id = Uuid::new_v4().to_string();
    let emulator = Emulator::new();
    let state = emulator.get_state();
    
    emulators.lock().unwrap().insert(id.clone(), emulator);
    
    let response = ApiResponse::success(EmulatorState {
        id,
        cpu: state,
    });
    
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
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.get_mut(&id) {
        Some(emulator) => {
            emulator.step();
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
    let mut emulators_lock = emulators.lock().unwrap();
    
    match emulators_lock.remove(&id) {
        Some(_) => {
            let response = ApiResponse::success(format!("Emulator {} deleted", id));
            Ok(warp::reply::json(&response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Emulator not found".to_string());
            Ok(warp::reply::json(&response))
        }
    }
}