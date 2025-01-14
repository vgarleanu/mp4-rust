use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct TrunBox {
    pub version: u8,
    pub flags: u32,
    pub sample_count: u32,
    pub data_offset: Option<i32>,
    pub first_sample_flags: Option<u32>,

    #[serde(skip_serializing)]
    pub sample_durations: Vec<u32>,
    #[serde(skip_serializing)]
    pub sample_sizes: Vec<u32>,
    #[serde(skip_serializing)]
    pub sample_flags: Vec<u32>,
    #[serde(skip_serializing)]
    pub sample_cts: Vec<u32>,
}

impl TrunBox {
    pub const FLAG_DATA_OFFSET: u32 = 0x01;
    pub const FLAG_FIRST_SAMPLE_FLAGS: u32 = 0x04;
    pub const FLAG_SAMPLE_DURATION: u32 = 0x100;
    pub const FLAG_SAMPLE_SIZE: u32 = 0x200;
    pub const FLAG_SAMPLE_FLAGS: u32 = 0x400;
    pub const FLAG_SAMPLE_CTS: u32 = 0x800;

    pub fn get_type(&self) -> BoxType {
        BoxType::TrunBox
    }

    pub fn get_size(&self) -> u64 {
        let mut sum = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        if TrunBox::FLAG_DATA_OFFSET & self.flags > 0 {
            sum += 4;
        }
        if TrunBox::FLAG_FIRST_SAMPLE_FLAGS & self.flags > 0 {
            sum += 4;
        }
        if TrunBox::FLAG_SAMPLE_DURATION & self.flags > 0 {
            sum += 4 * self.sample_count as u64;
            assert_eq!(self.sample_count as usize, self.sample_durations.len());
        }
        if TrunBox::FLAG_SAMPLE_SIZE & self.flags > 0 {
            sum += 4 * self.sample_count as u64;
            assert_eq!(self.sample_count as usize, self.sample_sizes.len());
        }
        if TrunBox::FLAG_SAMPLE_FLAGS & self.flags > 0 {
            sum += 4 * self.sample_count as u64;
            assert_eq!(self.sample_count as usize, self.sample_flags.len());
        }
        if TrunBox::FLAG_SAMPLE_CTS & self.flags > 0 {
            sum += 4 * self.sample_count as u64;
            assert_eq!(self.sample_count as usize, self.sample_cts.len());
        }
        sum
    }
}

impl Mp4Box for TrunBox {
    fn box_type(&self) -> BoxType {
        return self.get_type();
    }

    fn box_size(&self) -> u64 {
        return self.get_size();
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("sample_size={}", self.sample_count);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for TrunBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let (version, flags) = read_box_header_ext(reader)?;

        let sample_count = reader.read_u32::<BigEndian>()?;

        let data_offset = if TrunBox::FLAG_DATA_OFFSET & flags > 0 {
            Some(reader.read_i32::<BigEndian>()?)
        } else {
            None
        };

        let first_sample_flags = if TrunBox::FLAG_FIRST_SAMPLE_FLAGS & flags > 0 {
            Some(reader.read_u32::<BigEndian>()?)
        } else {
            None
        };

        let mut sample_durations = Vec::with_capacity(sample_count as usize);
        let mut sample_sizes = Vec::with_capacity(sample_count as usize);
        let mut sample_flags = Vec::with_capacity(sample_count as usize);
        let mut sample_cts = Vec::with_capacity(sample_count as usize);
        for _ in 0..sample_count {
            if TrunBox::FLAG_SAMPLE_DURATION & flags > 0 {
                let duration = reader.read_u32::<BigEndian>()?;
                sample_durations.push(duration);
            }

            if TrunBox::FLAG_SAMPLE_SIZE & flags > 0 {
                let sample_size = reader.read_u32::<BigEndian>()?;
                sample_sizes.push(sample_size);
            }

            if TrunBox::FLAG_SAMPLE_FLAGS & flags > 0 {
                let sample_flag = reader.read_u32::<BigEndian>()?;
                sample_flags.push(sample_flag);
            }

            if TrunBox::FLAG_SAMPLE_CTS & flags > 0 {
                let cts = reader.read_u32::<BigEndian>()?;
                sample_cts.push(cts);
            }
        }

        skip_bytes_to(reader, start + size)?;

        Ok(TrunBox {
            version,
            flags,
            sample_count,
            data_offset,
            first_sample_flags,
            sample_durations,
            sample_sizes,
            sample_flags,
            sample_cts,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for TrunBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let mut real_len = 0;
        let size = self.box_size();
        real_len += BoxHeader::new(self.box_type(), size).write(writer)?;

        real_len += write_box_header_ext(writer, self.version, self.flags)?;

        real_len += 4;
        writer.write_u32::<BigEndian>(self.sample_count)?;
        if let Some(v) = self.data_offset {
            real_len += 4;
            writer.write_i32::<BigEndian>(v)?;
        } else if TrunBox::FLAG_DATA_OFFSET & self.flags > 0 {
            println!("got flag data offset but self.data_offset is None");
        }

        if let Some(v) = self.first_sample_flags {
            real_len += 4;
            writer.write_u32::<BigEndian>(v)?;
        } else if TrunBox::FLAG_FIRST_SAMPLE_FLAGS & self.flags > 0 {
            println!("got flag_first_sample_flags but self.first_sample_flags is None.");
        }

        for i in 0..self.sample_sizes.len() as usize {
            if TrunBox::FLAG_SAMPLE_DURATION & self.flags > 0 {
                real_len += 4;
                writer.write_u32::<BigEndian>(self.sample_durations[i])?;
            }
            if TrunBox::FLAG_SAMPLE_SIZE & self.flags > 0 {
                real_len += 4;
                writer.write_u32::<BigEndian>(self.sample_sizes[i])?;
            }
            if TrunBox::FLAG_SAMPLE_FLAGS & self.flags > 0 {
                real_len += 4;
                writer.write_u32::<BigEndian>(self.sample_flags[i])?;
            }
            if TrunBox::FLAG_SAMPLE_CTS & self.flags > 0 {
                real_len += 4;
                writer.write_u32::<BigEndian>(self.sample_cts[i])?;
            }
        }

        // sanity check
        // assert_eq!(real_len, size);
        // FIXME: Ugly hack, it seems that sometimes the writer writes less bytes than it expects
        // which makes no fucking sense btw, anyway we detect this and fill it with zeroes.
        //
        // NOTE: 95% sure this is UB.
        if real_len < size {
            for _ in 0..(size - real_len) {
                writer.write_u8(0)?;
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_trun_same_size() {
        let src_box = TrunBox {
            version: 0,
            flags: 0,
            data_offset: None,
            sample_count: 0,
            sample_sizes: vec![],
            sample_flags: vec![],
            first_sample_flags: None,
            sample_durations: vec![],
            sample_cts: vec![],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::TrunBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TrunBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_trun_many_sizes() {
        let src_box = TrunBox {
            version: 0,
            flags: TrunBox::FLAG_SAMPLE_DURATION
                | TrunBox::FLAG_SAMPLE_SIZE
                | TrunBox::FLAG_SAMPLE_FLAGS
                | TrunBox::FLAG_SAMPLE_CTS,
            data_offset: None,
            sample_count: 9,
            sample_sizes: vec![1165, 11, 11, 8545, 10126, 10866, 9643, 9351, 7730],
            sample_flags: vec![1165, 11, 11, 8545, 10126, 10866, 9643, 9351, 7730],
            first_sample_flags: None,
            sample_durations: vec![1165, 11, 11, 8545, 10126, 10866, 9643, 9351, 7730],
            sample_cts: vec![1165, 11, 11, 8545, 10126, 10866, 9643, 9351, 7730],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::TrunBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TrunBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
