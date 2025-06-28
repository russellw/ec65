# EC65 - Enterprise 6502 Emulator

A complete enterprise-grade MOS 6502 CPU emulator with REST API, multi-tenancy, authentication, and comprehensive monitoring.

## 🚀 Features

### Core 6502 Emulation
- **Complete 6502 instruction set** with accurate cycle timing
- **Historic authenticity** including original 6502 bugs (JMP indirect page boundary)
- **Multiple addressing modes** (immediate, zero page, absolute, indexed, indirect)
- **Stack operations** and subroutine calls
- **Full flag handling** for arithmetic and logic operations

### Enterprise Platform
- **JWT Authentication** with bcrypt password hashing
- **API Key Management** with granular permissions
- **Multi-tenant Architecture** with user isolation
- **Instance Management** with 5 performance tiers (Micro to Turbo)
- **Snapshot/Checkpoint System** with RLE compression
- **Prometheus Metrics** for monitoring and observability
- **RESTful API** with comprehensive error handling

### Performance Tiers
- **Micro**: 100K cycles/sec, 8KB memory, $0.001/hour
- **Small**: 500K cycles/sec, 16KB memory, $0.005/hour  
- **Standard**: 1M cycles/sec, 32KB memory, $0.01/hour
- **Performance**: 2M cycles/sec, 64KB memory, $0.02/hour
- **Turbo**: 5M cycles/sec, 64KB memory, $0.05/hour

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   REST API      │    │  Authentication │    │    Metrics     │
│  (Warp + JSON)  │◄──►│ (JWT + API Keys)│◄──►│  (Prometheus)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Instance Mgmt   │    │   Snapshots     │    │  6502 Cores     │
│ (Multi-tenant)  │◄──►│ (RLE Compress)  │◄──►│  (CPU + Memory) │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ 
- Python 3.7+ (for test client)

### Installation
```bash
git clone <repository>
cd ec65
cargo build --release
```

### Start the Server
```bash
cargo run
# Server starts on http://localhost:3030
```

### Test with Enterprise Client
```bash
# Install Python dependencies
pip install requests

# Run comprehensive tests
python3 enterprise_client.py
```

## 📡 API Endpoints

### Authentication
- `POST /auth/login` - JWT token authentication
- `POST /auth/register` - User registration  
- `GET /auth/user` - Get current user info

### API Key Management
- `POST /api-keys` - Create API key with permissions
- `GET /api-keys` - List user's API keys
- `DELETE /api-keys/{id}` - Revoke API key

### Basic Emulator Operations
- `POST /emulator` - Create new emulator instance
- `GET /emulator/{id}` - Get emulator state
- `POST /emulator/{id}/reset` - Reset emulator
- `POST /emulator/{id}/step` - Execute single instruction
- `POST /emulator/{id}/execute` - Execute multiple steps
- `POST /emulator/{id}/program` - Load program into memory
- `GET /emulator/{id}/memory` - Read memory range
- `POST /emulator/{id}/memory` - Write single byte
- `GET /emulators` - List all instances
- `DELETE /emulator/{id}` - Delete instance

### Enterprise Instance Management  
- `POST /instances` - Create enterprise instance with tier
- `GET /instances` - List user's instances
- `GET /instances/{id}` - Get instance details *(planned)*
- `POST /instances/{id}/start` - Start instance *(planned)*
- `POST /instances/{id}/stop` - Stop instance *(planned)*
- `POST /instances/{id}/pause` - Pause instance *(planned)*

### Snapshot Management
- `POST /snapshots` - Create snapshot with compression
- `GET /snapshots` - List snapshots for emulator
- `GET /snapshots/{id}` - Get snapshot details *(planned)*
- `POST /snapshots/{id}/restore` - Restore from snapshot *(planned)*
- `DELETE /snapshots/{id}` - Delete snapshot *(planned)*

### Monitoring
- `GET /metrics` - Prometheus metrics endpoint
- `GET /instances/{id}/stats` - Usage statistics *(planned)*

## 💡 Usage Examples

### Basic 6502 Programming
```bash
# Create emulator
curl -X POST http://localhost:3030/emulator

# Load simple program (LDA #$42, STA $6000, BRK)
curl -X POST http://localhost:3030/emulator/{id}/program \
  -H "Content-Type: application/json" \
  -d '{"address": 32768, "data": [169, 66, 141, 0, 96, 0]}'

# Execute program
curl -X POST http://localhost:3030/emulator/{id}/execute \
  -H "Content-Type: application/json" \
  -d '{"steps": 10}'

# Read result from memory
curl "http://localhost:3030/emulator/{id}/memory?address=24576&length=1"
```

### Enterprise Authentication
```bash
# Login and get JWT token
curl -X POST http://localhost:3030/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# Create API key
curl -X POST http://localhost:3030/api-keys \
  -H "Authorization: Bearer <jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My API Key", "permissions": ["CreateEmulator", "ReadEmulator"]}'

# Use API key for requests
curl -X POST http://localhost:3030/emulator \
  -H "Authorization: ApiKey mos6502_<key>"
```

## 📊 Monitoring

The emulator provides comprehensive Prometheus metrics:

- **Request metrics**: Duration, count, error rates by endpoint
- **Emulator metrics**: Active instances, CPU cycles, memory usage
- **Business metrics**: User registrations, API key usage, snapshot operations

Access metrics at: `http://localhost:3030/metrics`

## 🧪 Testing

### Unit Tests
```bash
cargo test
```

### Enterprise Test Client
```bash
python3 enterprise_client.py
```

The test client exercises all implemented features and provides detailed status reporting.

## 🏗️ Implementation Status

### ✅ Implemented & Working
- Complete 6502 CPU emulation with all instructions
- JWT authentication system with bcrypt
- API key management with permissions
- Multi-tenant emulator instances  
- Basic CRUD operations for emulators
- Enterprise instance creation and listing
- Snapshot creation and listing with RLE compression
- Comprehensive Prometheus metrics (990+ metrics)
- RESTful API with proper error handling
- CORS support for web frontends

### 🚧 Planned Features
- Instance lifecycle management (start/stop/pause)
- Snapshot restore and delete operations
- Usage statistics and billing integration
- Rate limiting and quota enforcement
- Admin panel and management interface
- WebSocket support for real-time monitoring

## 🛠️ Development

### Project Structure
```
src/
├── lib.rs          # Library entry point
├── main.rs         # Server entry point  
├── cpu.rs          # 6502 CPU implementation
├── memory.rs       # Memory management
├── server.rs       # HTTP API and routes
├── auth.rs         # Authentication & authorization
├── metrics.rs      # Prometheus metrics
├── instance_types.rs  # Enterprise tiers & quotas
└── snapshots.rs    # Checkpoint system
```

### Key Dependencies
- **warp**: HTTP server framework
- **tokio**: Async runtime
- **serde**: JSON serialization
- **jsonwebtoken**: JWT authentication
- **bcrypt**: Password hashing
- **prometheus**: Metrics collection
- **uuid**: Instance identifiers

## 📝 License

MIT License
