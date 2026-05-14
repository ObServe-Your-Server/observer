pub struct BmfHeader {
    magic: [u8; 8],
    version: u16,
    pad: [u8; 4],
    block_count: u32,
    checksum: u32,
}

impl BmfHeader {
    pub fn default() -> Self {
        BmfHeader {
            magic: [b'B', b'M', b'F', b'O', b'b', b's', b'r', b'v'],
            version: 1,
            pad: [0, 0, 0, 0],
            block_count: 0,
            checksum: 0,
        }
    }
}
