use winapi::um::{winnls, stringapiset};
use std::io::{Error, ErrorKind, Result};
use std::iter::Iterator;
use std::mem::MaybeUninit;

pub struct ByteDecoder<I: Iterator<Item = u8>> {
    iter: I,
    codepage: u32,
    buf: Option<u8>,
    default_character: u16,
}

pub struct ByteEncoder<I: Iterator<Item = char>> {
    iter: I,
    codepage: u32,
    buffer_index: usize,
    buffer_size: usize,
    buffer: [u8; 4],
}

impl<I: Iterator<Item = u8>> ByteDecoder<I> {
    pub fn new(iter: I, codepage: u32) -> Result<Self> {
        let default_character;

        unsafe {
            let mut cp_info: MaybeUninit<winnls::CPINFOEXA> = MaybeUninit::uninit();
            if winnls::GetCPInfoExA(codepage, 0, cp_info.as_mut_ptr()) == 0 {
                return Err(Error::last_os_error());
            }

            default_character = cp_info.assume_init().UnicodeDefaultChar;
        }

        Ok(ByteDecoder {
            iter,
            codepage,
            buf: None,
            default_character,
        })
    }
}

impl<I: Iterator<Item = char>> ByteEncoder<I> {
    pub fn new(iter: I, codepage: u32) -> Self {
        ByteEncoder {
            iter,
            codepage,
            buffer: [0; 4],
            buffer_index: 0,
            buffer_size: 0
        }
    }
}

impl<I: Iterator<Item = u8>> Iterator for ByteDecoder<I> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        const INVALID_BYTE_SEQUENCE_ERR: &str = "Invalid byte sequence!";

        let mut byte_buf = [0u8; 8];
        let mut char_buf = [0u16; 3];

        byte_buf[0] = match self.buf.take() {
            Some(byte) => byte,
            None => self.iter.next()?,
        };

        for byte_count in 1..=byte_buf.len() {
            if byte_count > 1 {
                let next = self.iter.next();
                if let Some(byte) = next {
                    byte_buf[byte_count - 1] = byte;
                } else {
                    return Some(Err(Error::new(
                        ErrorKind::InvalidData,
                        INVALID_BYTE_SEQUENCE_ERR,
                    )));
                }
            }

            let char_count;

            unsafe {
                char_count = stringapiset::MultiByteToWideChar(
                    self.codepage,
                    8, // ERR_INVALID_CHARS
                    byte_buf.as_ptr().cast(),
                    byte_count as i32,
                    char_buf.as_mut_ptr(),
                    char_buf.len() as i32,
                );
            }

            match char_count {
                1 => {
                    unsafe {
                        // Should be safe, MultiByteToWideChar should only return valid utf16
                        return Some(Ok(char::from_u32_unchecked(char_buf[0] as u32)));
                    }
                }
                2 => {
                    // First codepoint is singular
                    if char_buf[0] < 0xD800 || char_buf[0] > 0xDFFF {
                        self.buf = Some(byte_buf[byte_count - 1]);

                        if char_buf[0] == self.default_character {
                            return Some(Err(Error::new(
                                ErrorKind::InvalidData,
                                INVALID_BYTE_SEQUENCE_ERR,
                            )));
                        } else {
                            unsafe {
                                // Should be safe, MultiByteToWideChar should only return valid utf16
                                return Some(Ok(char::from_u32_unchecked(char_buf[0] as u32)));
                            }
                        }
                    } else {
                        let c = (((char_buf[0] - 0xD800) as u32) << 10
                            | (char_buf[1] - 0xDC00) as u32)
                            + 0x1_0000;
                        unsafe {
                            return Some(Ok(char::from_u32_unchecked(c)));
                        }
                    }
                }
                3 => {
                    debug_assert_eq!(char_buf[0], 0xDFFF);
                    self.buf = Some(byte_buf[byte_count - 1]);
                    return Some(Err(Error::new(
                        ErrorKind::InvalidInput,
                        INVALID_BYTE_SEQUENCE_ERR,
                    )));
                }
                _ => {
                    if !unsafe {
                        winnls::IsDBCSLeadByteEx(self.codepage, byte_buf[0]) != 0
                    } {
                        return Some(Err(Error::last_os_error()));
                    }
                }
            }
        }

        Some(Err(Error::new(
            ErrorKind::Other,
            "Single character is longer than 8 bytes!",
        )))
    }
}

impl<I: Iterator<Item = char>> Iterator for ByteEncoder<I> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer_size != 0 {
            let byte = self.buffer[self.buffer_index];
            self.buffer_index += 1;
            if self.buffer_index >= self.buffer_size {
                self.buffer_size = 0;
            }
            return Some(byte);
        }

        if let Some(next_char) = self.iter.next() {

            let mut utf16_buf = [0u16; 2];
            let utf16_char = next_char.encode_utf16(&mut utf16_buf);

            unsafe {
                self.buffer_size = stringapiset::WideCharToMultiByte(
                    self.codepage,
                    0,
                    utf16_char.as_ptr(),
                    utf16_char.len() as i32,
                    self.buffer.as_mut_ptr().cast(),
                    self.buffer.len() as i32,
                    std::ptr::null(),
                    std::ptr::null_mut()
                ) as usize;
            }

            match self.buffer_size {
                0 => {
                    panic!("Failed to encode char '{}': {}", next_char, Error::last_os_error());
                }
                1 => {
                    self.buffer_size = 0;
                    Some(self.buffer[0])
                }
                _ => {
                    self.buffer_index = 1;
                    Some(self.buffer[0])
                }
            }
        } else {
            None
        }
    }
}
