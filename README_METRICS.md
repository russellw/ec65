# Prometheus Telemetry for MOS 6502 Emulator

This document describes the Prometheus-compatible telemetry implementation for the 6502 emulator server.

## Metrics Endpoint

The server exposes metrics at: `http://localhost:3030/metrics`

## Available Metrics

### CPU Instruction Metrics

#### `cpu_instructions_total`
- **Type**: Counter
- **Description**: Total number of CPU instructions executed by opcode
- **Labels**: 
  - `opcode`: Hexadecimal opcode (e.g., "0x18", "0xA9")
  - `instruction`: Instruction mnemonic (e.g., "CLC", "LDA")

Example:
```
cpu_instructions_total{instruction="LDA",opcode="0xA9"} 1
cpu_instructions_total{instruction="CLC",opcode="0x18"} 1
cpu_instructions_total{instruction="BNE",opcode="0xD0"} 5
```

#### `cpu_cycles_total`
- **Type**: Counter
- **Description**: Total number of CPU cycles executed across all emulators

#### `instruction_duration_seconds`
- **Type**: Histogram
- **Description**: Time spent executing instructions in seconds
- **Labels**:
  - `instruction`: Instruction mnemonic
- **Buckets**: [1μs, 5μs, 10μs, 50μs, 100μs, 500μs, 1ms]

### API Request Metrics

#### `api_requests_total`
- **Type**: Counter
- **Description**: Total number of API requests
- **Labels**:
  - `method`: HTTP method (GET, POST, PUT, DELETE)
  - `endpoint`: API endpoint pattern (e.g., "/emulator", "/emulator/:id/step")
  - `status`: HTTP status code

#### `api_request_duration_seconds`
- **Type**: Histogram
- **Description**: API request duration in seconds
- **Labels**:
  - `method`: HTTP method
  - `endpoint`: API endpoint pattern
- **Buckets**: [1ms, 5ms, 10ms, 50ms, 100ms, 500ms, 1s, 5s]

### Emulator State Metrics

#### `active_emulators_total`
- **Type**: Gauge
- **Description**: Number of active emulator instances

#### `cpu_register_value`
- **Type**: Gauge
- **Description**: Current CPU register values
- **Labels**:
  - `emulator_id`: UUID of the emulator instance
  - `register`: Register name (A, X, Y, PC, SP, STATUS)

#### `cpu_flags`
- **Type**: Gauge
- **Description**: Current CPU flag states (0 or 1)
- **Labels**:
  - `emulator_id`: UUID of the emulator instance
  - `flag`: Flag name (carry, zero, interrupt_disable, decimal_mode, break_command, overflow, negative)

## Setting up Prometheus

### 1. Install Prometheus

Download Prometheus from: https://prometheus.io/download/

### 2. Use the provided configuration

Copy the `prometheus.yml` file to your Prometheus directory or use it directly:

```bash
prometheus --config.file=prometheus.yml
```

### 3. Start the 6502 Emulator Server

```bash
cargo run -- --server
```

### 4. Access Prometheus

- Prometheus UI: http://localhost:9090
- 6502 Emulator metrics: http://localhost:3030/metrics

## Example Prometheus Queries

### Instruction Execution Rate
```promql
rate(cpu_instructions_total[5m])
```

### Most Executed Instructions
```promql
topk(10, cpu_instructions_total)
```

### Average Instruction Duration
```promql
rate(instruction_duration_seconds_sum[5m]) / rate(instruction_duration_seconds_count[5m])
```

### API Request Rate by Endpoint
```promql
rate(api_requests_total[5m])
```

### API Error Rate
```promql
rate(api_requests_total{status!="200"}[5m]) / rate(api_requests_total[5m])
```

### Active Emulator Count
```promql
active_emulators_total
```

### CPU Register Values
```promql
cpu_register_value{register="A"}
```

### CPU Flag States
```promql
cpu_flags{flag="carry"}
```

## Grafana Dashboard

These metrics can be visualized in Grafana using the Prometheus data source. Recommended panels:

1. **Instruction Execution Rate** (Graph)
   - Query: `rate(cpu_instructions_total[5m])`
   - Group by: `instruction`

2. **Most Executed Instructions** (Table)
   - Query: `topk(10, cpu_instructions_total)`

3. **API Request Rate** (Graph)
   - Query: `rate(api_requests_total[5m])`
   - Group by: `endpoint`

4. **Active Emulators** (Stat)
   - Query: `active_emulators_total`

5. **CPU State** (Gauge/Stat)
   - Queries: `cpu_register_value`, `cpu_flags`

## Alert Rules

Example alert rules for monitoring:

```yaml
groups:
- name: mos6502_emulator
  rules:
  - alert: HighAPIErrorRate
    expr: rate(api_requests_total{status!="200"}[5m]) / rate(api_requests_total[5m]) > 0.1
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "High API error rate detected"
      description: "API error rate is {{ $value }} for the last 5 minutes"
  
  - alert: TooManyActiveEmulators
    expr: active_emulators_total > 100
    for: 1m
    labels:
      severity: warning
    annotations:
      summary: "Too many active emulator instances"
      description: "Number of active emulators: {{ $value }}"
```

## Integration with Monitoring Stack

The metrics are compatible with the standard Prometheus ecosystem:

- **Prometheus**: For metrics collection and storage
- **Grafana**: For visualization and dashboards  
- **Alertmanager**: For alert routing and notification
- **Node Exporter**: For system-level metrics (can run alongside)

## Performance Impact

The telemetry implementation has minimal performance impact:
- Metrics are updated using atomic operations
- No blocking I/O in the metrics collection path
- Histogram buckets are pre-allocated
- Label cardinality is controlled to prevent explosion

The `/metrics` endpoint should be scraped at reasonable intervals (5-15 seconds) to balance observability with performance.