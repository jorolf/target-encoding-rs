
pub struct Utf8Encoder<I: Iterator<Item = char>> {
    iter: I,
    buffer_index: usize,
    buffer_size: usize,
    buffer: [u8; 4],
}

impl<I: Iterator<Item = char>> Utf8Encoder<I> {
    pub fn new(iter: I) -> Utf8Encoder<I> {
        Utf8Encoder { 
            iter,
            buffer_index: 0,
            buffer_size: 0,
            buffer: [0; 4]
        }
    }
}

impl<T> Iterator for Utf8Encoder<T>
where
    T: Iterator<Item = char>
{
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
            let str = next_char.encode_utf8(&mut self.buffer);
            if str.len() != 1 {
                self.buffer_size = str.len();
                self.buffer_index = 1;
            }
            Some(self.buffer[0])
        } else {
            None
        }
    }
}
