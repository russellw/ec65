pub struct Memory {
    data: [u8; 65536], // 64KB memory space
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            data: [0; 65536],
        }
    }
    
    pub fn read(&self, address: u16) -> u8 {
        self.data[address as usize]
    }
    
    pub fn write(&mut self, address: u16, value: u8) {
        self.data[address as usize] = value;
    }
    
    // Load ROM data into memory
    pub fn load_rom(&mut self, data: &[u8], start_address: u16) {
        let start = start_address as usize;
        let end = (start + data.len()).min(65536);
        let len = end - start;
        self.data[start..end].copy_from_slice(&data[..len]);
    }
    
    // Read a 16-bit value in little-endian format
    pub fn read_u16(&self, address: u16) -> u16 {
        let low = self.read(address) as u16;
        let high = self.read(address.wrapping_add(1)) as u16;
        (high << 8) | low
    }
    
    // Write a 16-bit value in little-endian format
    pub fn write_u16(&mut self, address: u16, value: u16) {
        self.write(address, (value & 0xFF) as u8);
        self.write(address.wrapping_add(1), (value >> 8) as u8);
    }
}