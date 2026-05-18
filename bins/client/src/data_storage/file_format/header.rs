use std::thread::sleep;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header{
    magic: [u8; 8],
    version: u8,
    pad: [u8; 4],
    block_count: u32,
    checksum: u32
}

impl Header{
    pub fn default() -> Self {
        let mut h = Header{
            magic: [b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'],
            version: 1,
            pad: [0,0,0,0],
            block_count: 0,
            checksum: 0,
        };
        h.checksum = h.compute_checksum();
        h
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.magic);
        hasher.update(&[self.version]);
        hasher.update(&self.pad);
        hasher.update(&self.block_count.to_le_bytes());
        hasher.finalize()
    }
    
    fn update_checksum(&mut self) -> &u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.magic);
        hasher.update(&[self.version]);
        hasher.update(&self.pad);
        hasher.update(&self.block_count.to_le_bytes());
        
        self.checksum = hasher.finalize();
        &self.checksum
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.version);
        bytes.extend_from_slice(&self.pad);
        bytes.extend_from_slice(&self.block_count.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    pub fn magic(&self) -> &[u8;8] {
        &self.magic
    }

    pub fn version(&self) -> &u8 {
        &self.version
    }

    pub fn pad(&self) -> &[u8;4] {
        &self.pad
    }

    pub fn block_count(&self) -> &u32 {
        &self.block_count
    }

    pub fn increment_block_count(&mut self) -> &u32 {
        self.block_count += 1;
        self.update_checksum();
        &self.checksum()
    }
    
    pub fn checksum(&self) -> &u32 {
        &self.checksum
    }

    pub fn verify(&self) -> bool {
        self.checksum == self.compute_checksum()
    }
}