fn read_u16(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| format!("truncated at {at}"))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("truncated at {at}"))
}

fn read_c_string(bytes: &[u8], at: usize) -> Result<String, String> {
    let tail = bytes.get(at..).ok_or_else(|| format!("string out of bounds at {at}"))?;
    let end = tail.iter().position(|&b| b == 0).ok_or("unterminated export name")?;
    String::from_utf8(tail[..end].to_vec()).map_err(|error| error.to_string())
}

struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_offset: u32,
}

fn file_offset(sections: &[Section], rva: u32) -> Result<usize, String> {
    sections
        .iter()
        .find(|s| rva >= s.virtual_address && rva < s.virtual_address + s.virtual_size.max(1))
        .map(|s| (rva - s.virtual_address + s.raw_offset) as usize)
        .ok_or_else(|| format!("rva {rva:#x} is in no section"))
}

pub fn export_names(bytes: &[u8]) -> Result<Vec<String>, String> {
    let pe = read_u32(bytes, 0x3c)? as usize;
    if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("not a PE file".into());
    }
    let section_count = read_u16(bytes, pe + 6)? as usize;
    let optional_size = read_u16(bytes, pe + 20)? as usize;
    let optional = pe + 24;
    let data_directories = match read_u16(bytes, optional)? {
        0x20b => optional + 112,
        0x10b => optional + 96,
        magic => return Err(format!("unknown optional header magic {magic:#x}")),
    };
    let export_rva = read_u32(bytes, data_directories)?;
    let section_table = optional + optional_size;
    let sections = (0..section_count)
        .map(|index| {
            let at = section_table + index * 40;
            Ok(Section {
                virtual_size: read_u32(bytes, at + 8)?,
                virtual_address: read_u32(bytes, at + 12)?,
                raw_offset: read_u32(bytes, at + 20)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if export_rva == 0 {
        return Ok(Vec::new());
    }
    let directory = file_offset(&sections, export_rva)?;
    let name_count = read_u32(bytes, directory + 24)? as usize;
    let names = file_offset(&sections, read_u32(bytes, directory + 32)?)?;
    (0..name_count)
        .map(|index| read_c_string(bytes, file_offset(&sections, read_u32(bytes, names + index * 4)?)?))
        .collect()
}
