use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use crate::format::v1::block::BmfBlock;
use crate::format::v1::footer::BmfFooter;
use crate::format::v1::header::BmfHeader;

// the format is BMF Binary Metrics Format
pub struct BmfFile{
    pub header: BmfHeader,
    pub data: Vec<BmfBlock>,
    pub footer: BmfFooter,
}

impl BmfFile {
    pub fn new_file<P: AsRef<Path>>(path: P) -> Result<(), io::Error>{
        let mut file = File::create(path)?;
        BmfFile::write_header(file);
        Ok(())
    }
    fn write_header(mut file: File){
        let x: i32 = 40;
        let y = x.to_le_bytes();
        //let header = BmfHeader
        todo!()
        //let res = file.write_vectored()
    }
}