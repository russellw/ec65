#!/usr/bin/env python3
"""
Python client test program for the 6502 Emulator REST API.

This script demonstrates how to interact with the 6502 emulator server
by creating an emulator instance, loading a program, and executing it.
"""

import requests
import json
import time
import sys
from typing import Dict, Any, Optional

class Mos6502Client:
    """Client for interacting with the MOS 6502 emulator REST API."""
    
    def __init__(self, base_url: str = "http://localhost:3030"):
        self.base_url = base_url
        self.session = requests.Session()
        self.emulator_id: Optional[str] = None
    
    def _make_request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """Make an HTTP request and return parsed JSON response."""
        url = f"{self.base_url}{endpoint}"
        
        try:
            response = self.session.request(method, url, **kwargs)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            print(f"Request failed: {e}")
            sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Failed to parse JSON response: {e}")
            sys.exit(1)
    
    def create_emulator(self) -> str:
        """Create a new emulator instance."""
        print("Creating new emulator instance...")
        result = self._make_request("POST", "/emulator")
        
        if result["success"]:
            self.emulator_id = result["data"]["id"]
            print(f"✓ Created emulator with ID: {self.emulator_id}")
            return self.emulator_id
        else:
            print(f"✗ Failed to create emulator: {result['error']}")
            sys.exit(1)
    
    def get_state(self) -> Dict[str, Any]:
        """Get the current CPU state."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        result = self._make_request("GET", f"/emulator/{self.emulator_id}")
        
        if result["success"]:
            return result["data"]["cpu"]
        else:
            print(f"✗ Failed to get state: {result['error']}")
            sys.exit(1)
    
    def reset(self) -> None:
        """Reset the CPU."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        print("Resetting CPU...")
        result = self._make_request("POST", f"/emulator/{self.emulator_id}/reset")
        
        if result["success"]:
            print("✓ CPU reset complete")
        else:
            print(f"✗ Failed to reset: {result['error']}")
            sys.exit(1)
    
    def step(self) -> Dict[str, Any]:
        """Execute a single instruction."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        result = self._make_request("POST", f"/emulator/{self.emulator_id}/step")
        
        if result["success"]:
            return result["data"]["cpu"]
        else:
            print(f"✗ Failed to step: {result['error']}")
            sys.exit(1)
    
    def execute_steps(self, steps: int) -> Dict[str, Any]:
        """Execute multiple instructions."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        data = {"steps": steps}
        result = self._make_request("POST", f"/emulator/{self.emulator_id}/execute", 
                                  json=data)
        
        if result["success"]:
            return result["data"]
        else:
            print(f"✗ Failed to execute steps: {result['error']}")
            sys.exit(1)
    
    def write_memory(self, address: int, value: int) -> None:
        """Write a byte to memory."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        data = {"address": address, "value": value}
        result = self._make_request("POST", f"/emulator/{self.emulator_id}/memory", 
                                  json=data)
        
        if not result["success"]:
            print(f"✗ Failed to write memory: {result['error']}")
            sys.exit(1)
    
    def read_memory(self, address: int, length: int = 1) -> bytes:
        """Read bytes from memory."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        params = {"address": address, "length": length}
        result = self._make_request("GET", f"/emulator/{self.emulator_id}/memory", 
                                  params=params)
        
        if result["success"]:
            return bytes(result["data"]["data"])
        else:
            print(f"✗ Failed to read memory: {result['error']}")
            sys.exit(1)
    
    def load_program(self, address: int, data: bytes) -> None:
        """Load a program into memory."""
        if not self.emulator_id:
            raise ValueError("No emulator instance created")
        
        payload = {"address": address, "data": list(data)}
        result = self._make_request("POST", f"/emulator/{self.emulator_id}/program", 
                                  json=payload)
        
        if result["success"]:
            print(f"✓ Loaded {len(data)} bytes at address ${address:04X}")
        else:
            print(f"✗ Failed to load program: {result['error']}")
            sys.exit(1)
    
    def print_state(self, state: Dict[str, Any]) -> None:
        """Print CPU state in a readable format."""
        print(f"CPU State:")
        print(f"  A: ${state['a']:02X}   X: ${state['x']:02X}   Y: ${state['y']:02X}")
        print(f"  PC: ${state['pc']:04X}   SP: ${state['sp']:02X}")
        print(f"  Status: ${state['status']:02X}   Cycles: {state['cycles']}")
        print(f"  Halted: {state['halted']}")


def test_arithmetic_program():
    """Test program that demonstrates basic arithmetic operations."""
    print("=== Testing Arithmetic Program ===\n")
    
    client = Mos6502Client()
    client.create_emulator()
    
    # Program: Add two numbers and store result
    # LDA #$10     ; Load 16 into A
    # CLC          ; Clear carry
    # ADC #$05     ; Add 5 to A (result: 21)
    # STA $00      ; Store result at zero page
    # BRK          ; Break
    program = bytes([
        0xA9, 0x10,  # LDA #$10
        0x18,        # CLC
        0x69, 0x05,  # ADC #$05
        0x85, 0x00,  # STA $00
        0x00         # BRK
    ])
    
    # Load program at $8000
    client.load_program(0x8000, program)
    
    # Set reset vector to point to our program
    client.write_memory(0xFFFC, 0x00)  # Low byte
    client.write_memory(0xFFFD, 0x80)  # High byte
    
    # Reset CPU
    client.reset()
    
    print("Initial state:")
    state = client.get_state()
    client.print_state(state)
    print()
    
    # Execute the program step by step
    instructions = ["LDA #$10", "CLC", "ADC #$05", "STA $00", "BRK"]
    
    for i, instruction in enumerate(instructions):
        print(f"Executing: {instruction}")
        state = client.step()
        client.print_state(state)
        
        if i == 0:  # After LDA
            assert state['a'] == 0x10, f"Expected A=0x10, got A=0x{state['a']:02X}"
            print("✓ LDA instruction correct")
        elif i == 2:  # After ADC
            assert state['a'] == 0x15, f"Expected A=0x15, got A=0x{state['a']:02X}"
            print("✓ Addition result correct")
        
        print()
        
        if state['halted']:
            break
    
    # Check the stored result
    result = client.read_memory(0x00, 1)
    print(f"Value stored at $00: ${result[0]:02X}")
    assert result[0] == 0x15, f"Expected stored value 0x15, got 0x{result[0]:02X}"
    print("✓ Store instruction correct")


def test_loop_program():
    """Test program that demonstrates a simple counting loop."""
    print("\n=== Testing Loop Program ===\n")
    
    client = Mos6502Client()
    client.create_emulator()
    
    # Program: Count from 0 to 5
    # LDX #$00     ; Load 0 into X (counter)
    # INX          ; Increment X
    # CPX #$05     ; Compare X with 5
    # BNE $8001    ; Branch if not equal (loop)
    # BRK          ; Break when done
    program = bytes([
        0xA2, 0x00,  # LDX #$00
        0xE8,        # INX
        0xE0, 0x05,  # CPX #$05
        0xD0, 0xFB,  # BNE $8001 (relative -5)
        0x00         # BRK
    ])
    
    # Load program at $8000
    client.load_program(0x8000, program)
    
    # Set reset vector
    client.write_memory(0xFFFC, 0x00)
    client.write_memory(0xFFFD, 0x80)
    
    # Reset CPU
    client.reset()
    
    print("Running loop program (will execute until completion)...")
    state = client.get_state()
    client.print_state(state)
    print()
    
    # Execute until halt or max iterations
    max_steps = 50
    step_count = 0
    
    while not state['halted'] and step_count < max_steps:
        state = client.step()
        step_count += 1
        
        # Print state every few steps to show progress
        if step_count % 5 == 0 or state['halted']:
            print(f"Step {step_count}:")
            client.print_state(state)
            print()
    
    if state['halted']:
        print(f"✓ Program completed after {step_count} steps")
        print(f"✓ Final X register value: ${state['x']:02X} (expected: $05)")
        assert state['x'] == 0x05, f"Expected X=0x05, got X=0x{state['x']:02X}"
    else:
        print(f"✗ Program did not complete within {max_steps} steps")


def test_multiple_emulators():
    """Test creating and managing multiple emulator instances."""
    print("\n=== Testing Multiple Emulators ===\n")
    
    # Create two separate emulator instances
    client1 = Mos6502Client()
    client2 = Mos6502Client()
    
    id1 = client1.create_emulator()
    id2 = client2.create_emulator()
    
    print(f"Created two emulators: {id1[:8]}... and {id2[:8]}...")
    
    # Load different values into each emulator
    client1.write_memory(0x00, 0xAA)
    client2.write_memory(0x00, 0xBB)
    
    # Verify they're independent
    mem1 = client1.read_memory(0x00, 1)
    mem2 = client2.read_memory(0x00, 1)
    
    print(f"Emulator 1 memory[0x00]: ${mem1[0]:02X}")
    print(f"Emulator 2 memory[0x00]: ${mem2[0]:02X}")
    
    assert mem1[0] == 0xAA, "Emulator 1 memory incorrect"
    assert mem2[0] == 0xBB, "Emulator 2 memory incorrect"
    print("✓ Multiple emulators are independent")


def main():
    """Run all test programs."""
    print("6502 Emulator Python Client Test Program")
    print("=" * 50)
    
    try:
        # Test connection
        response = requests.get("http://localhost:3030/emulators", timeout=5)
        response.raise_for_status()
        print("✓ Successfully connected to emulator server\n")
    except requests.exceptions.RequestException as e:
        print(f"✗ Cannot connect to emulator server: {e}")
        print("Make sure the server is running with: cargo run -- --server")
        sys.exit(1)
    
    try:
        test_arithmetic_program()
        test_loop_program()
        test_multiple_emulators()
        
        print("\n" + "=" * 50)
        print("✓ All tests passed successfully!")
        
    except AssertionError as e:
        print(f"\n✗ Test assertion failed: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n✗ Unexpected error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()