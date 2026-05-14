pub struct Header{
    magic: [u8; 8],
    version: u8,
    pad: [u8; 4],
}

impl Header{
    pub fn default() -> Self {
        Header{
            magic: [b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'],
            version: 1,
            pad: [0,0,0,0]
        }
    }
}