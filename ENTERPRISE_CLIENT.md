# Enterprise 6502 Emulator Test Client

A comprehensive Python test client that exercises both available and planned enterprise features of the 6502 emulator API.

## Features Tested

### ✅ Currently Available Features
- **Basic Emulator Operations**: Create, load programs, execute, memory read/write, reset, delete
- **Metrics Endpoint**: Prometheus metrics collection
- **Multi-emulator Management**: Create and manage multiple emulator instances

### ❌ Enterprise Features (Infrastructure exists, endpoints missing)
- **Authentication & User Management**: JWT login, user registration, user info
- **API Key Management**: Create, list, revoke API keys with permissions
- **Enterprise Instance Types**: Micro, Small, Standard, Performance, Turbo
- **Instance Lifecycle**: Start, stop, pause instances with detailed stats
- **Snapshot/Checkpoint System**: Create, restore, manage emulator snapshots
- **Advanced Monitoring**: Usage statistics, quotas, rate limiting

## Requirements

- Python 3.7+
- `requests` library
- Running 6502 emulator server (on localhost:3030)

## Installation

```bash
# Install Python dependencies
pip install requests

# Make client executable
chmod +x enterprise_client.py
```

## Usage

### Start the 6502 Emulator Server
```bash
cargo run
```

### Run the Enterprise Test Client
```bash
python3 enterprise_client.py
```

Or directly:
```bash
./enterprise_client.py
```

## What the Client Tests

### 1. Authentication & User Management
- Attempts to register new users
- Tries JWT token authentication  
- Tests user info retrieval
- **Status**: Shows "404 - Endpoint not implemented"

### 2. API Key Management
- Creates API keys with specific permissions
- Lists user API keys
- Tests API key authentication
- **Status**: Shows "404 - Endpoint not implemented"

### 3. Basic Emulator Operations ✅
- Creates emulator instances
- Loads and executes sample 6502 programs:
  - Simple addition (LDA, ADC, STA)
  - Loop counter with branches
  - Memory copy operations
- Reads/writes memory locations
- Resets emulator state
- **Status**: Working correctly

### 4. Enterprise Instance Management
- Creates instances with specific types (Micro, Small, Standard, Performance, Turbo)
- Tests instance lifecycle (start, stop, pause)
- Retrieves detailed usage statistics
- **Status**: Shows "404 - Endpoint not implemented"

### 5. Snapshot/Checkpoint System
- Creates named snapshots with compression
- Lists available snapshots
- Restores from snapshots
- Manages snapshot lifecycle
- **Status**: Shows "404 - Endpoint not implemented"

### 6. Metrics & Monitoring
- Retrieves Prometheus metrics ✅
- Gets detailed usage statistics per instance
- **Status**: Basic metrics work, advanced stats missing

### 7. Stress Testing
- Creates multiple concurrent emulators
- Loads different programs into each
- Executes parallel operations
- Cleans up resources
- **Status**: Working for available features

## Sample Programs

The client includes three sample 6502 assembly programs:

1. **Simple Add**: Adds 5 + 3 and stores result in memory
2. **Loop Counter**: Counts down from 10 using branches
3. **Memory Copy**: Copies 8 bytes from one location to another

## Enterprise Infrastructure Status

While the HTTP endpoints are missing, the complete enterprise infrastructure exists:

### Available in Codebase
- ✅ **Authentication System** (`src/auth.rs`)
  - JWT token generation/validation
  - bcrypt password hashing
  - User registration/login logic
  - Permission-based access control

- ✅ **API Key System** (`src/auth.rs`)
  - API key generation with permissions
  - Key expiration and revocation
  - Multiple permission levels

- ✅ **Instance Types** (`src/instance_types.rs`)
  - 5 tier system (Micro to Turbo)
  - CPU cycle limits and pricing
  - Memory quotas per tier

- ✅ **Snapshot System** (`src/snapshots.rs`)
  - RLE compression algorithm
  - CPU state capture
  - Metadata and tagging
  - Checkpoint reasons tracking

- ✅ **Metrics System** (`src/metrics.rs`)
  - Prometheus integration
  - Counter and gauge metrics
  - Registry management

### Missing Components
- ❌ **HTTP Route Handlers**: Server routes for enterprise endpoints
- ❌ **Authentication Middleware**: JWT/API key validation
- ❌ **Rate Limiting**: Request throttling and quotas
- ❌ **Admin Panel**: Enterprise administration endpoints

## Expected Output

```
🚀 Enterprise 6502 Emulator Test Client
=====================================
[2024-01-15 10:30:15] INFO: ✅ Server is running and accessible

============================================================
STARTING COMPREHENSIVE ENTERPRISE FEATURE TESTS
============================================================

🔐 TESTING AUTHENTICATION & USER MANAGEMENT
--------------------------------------------------
[2024-01-15 10:30:15] INFO: Registering new user: testuser
[2024-01-15 10:30:15] ERROR: ❌ User registration (testuser) failed: Endpoint not implemented (404)

🔑 TESTING API KEY MANAGEMENT
--------------------------------------------------
[2024-01-15 10:30:15] INFO: Creating API key: Test API Key
[2024-01-15 10:30:15] ERROR: ❌ Create API key (Test API Key) failed: Endpoint not implemented (404)

🖥️  TESTING BASIC EMULATOR OPERATIONS (AVAILABLE)
--------------------------------------------------
[2024-01-15 10:30:15] INFO: Creating emulator: Test Emulator
[2024-01-15 10:30:15] INFO: ✅ Create emulator (Test Emulator) successful
[2024-01-15 10:30:15] INFO: Emulator created with ID: a1b2c3d4-e5f6-7890-abcd-ef1234567890

... [many more test results] ...

RESULTS SUMMARY:
✅ Basic emulator operations are working
✅ Metrics endpoint is available and working
❌ Enterprise endpoints not implemented yet
```

## Next Steps for Full Enterprise Support

1. **Implement HTTP Routes**: Add missing endpoints to `src/server.rs`
2. **Add Authentication Middleware**: Protect endpoints with JWT/API key validation
3. **Add Rate Limiting**: Implement quotas and request throttling
4. **Add Error Handling**: Comprehensive error responses
5. **Add Admin Panel**: Enterprise administration endpoints

This client serves as both a test tool and a specification for the complete enterprise API that should be implemented.