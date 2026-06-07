use std::hint::unlikely;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChecksumError {
    OutOfRange,
}

pub struct Checksum<
    const HASHED_SIZE: usize,
    const STREAM_BUFFER_SIZE: usize,
> {
    hashed: [u32; HASHED_SIZE],
    hashed_cursor: usize,
    stream_buf: [u8; STREAM_BUFFER_SIZE],
    stream_buf_cursor: usize,
}

impl<
    const HASHED_SIZE: usize,
    const STREAM_BUFFER_SIZE: usize
> Checksum<HASHED_SIZE, STREAM_BUFFER_SIZE> {
    pub const fn new() -> Self {
        Self {
            hashed: [0u32; HASHED_SIZE],
            stream_buf: [0u8; STREAM_BUFFER_SIZE],
            hashed_cursor: 0,
            stream_buf_cursor: 0,
        }
    }

    pub fn hash(&mut self, bytes: &[u8]) -> Result<u32, ChecksumError> {
        let cursor = self.hashed_cursor;

        if unlikely(cursor > HASHED_SIZE) {
            return Err(ChecksumError::OutOfRange)
        }

        let hash = crc32fast::hash(bytes);
        self.hashed[cursor] = hash;
        self.hashed_cursor += 1;
        Ok(hash)
    }

    pub fn contains_hash(&self, hash: &u32) -> bool {
        self.hashed[..self.hashed_cursor].contains(hash)
    }

    pub fn update(&mut self, bytes: &[u8]) -> Result<(), ChecksumError> {
        let cursor = self.stream_buf_cursor;

        if unlikely(cursor + bytes.len() > STREAM_BUFFER_SIZE) {
            return Err(ChecksumError::OutOfRange)
        }

        self.stream_buf[cursor..cursor + bytes.len()].copy_from_slice(bytes);
        self.stream_buf_cursor += bytes.len();
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<u32, ChecksumError> {
        if unlikely(self.hashed_cursor > HASHED_SIZE) {
            return Err(ChecksumError::OutOfRange)
        }

        let buf = &self.stream_buf[..self.stream_buf_cursor];
        let hash = crc32fast::hash(buf);
        self.hashed[self.hashed_cursor] = hash;
        self.hashed_cursor += 1;

        self.stream_buf_cursor = 0;
        Ok(hash)
    }

    pub fn clear(&mut self) {
        self.hashed_cursor = 0;
        self.stream_buf_cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.hashed_cursor == 0
    }

    pub fn len(&self) -> usize {
        self.hashed_cursor
    }

    pub fn hashed_slices(&self) -> &[u32] {
        &self.hashed[..self.hashed_cursor]
    }

    pub fn stream_remaining_capacity(&self) -> usize {
        STREAM_BUFFER_SIZE - self.stream_buf_cursor
    }

    pub fn get_hash(&self, index: usize) -> Option<u32> {
        if index < self.hashed_cursor {
            Some(self.hashed[index])
        } else {
            None
        }
    }

    pub fn contains_bytes(&self, bytes: &[u8]) -> bool {
        let target_hash = crc32fast::hash(bytes);

        self.contains_hash(&target_hash)
    }
}

impl<
    const HASHED_SIZE: usize,
    const STREAM_BUFFER_SIZE: usize
> std::io::Write for Checksum<HASHED_SIZE, STREAM_BUFFER_SIZE> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.update(buf)
            .map(|_| buf.len())
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "Stream buffer overflow"
            ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
