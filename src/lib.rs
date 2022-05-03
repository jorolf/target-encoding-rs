use std::io::Result;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::{ByteDecoder, ByteEncoder};
#[cfg(windows)]
use winapi::um::winnls;

#[cfg(not(windows))]
mod not_windows;
#[cfg(not(windows))]
use not_windows::Utf8Encoder;
#[cfg(not(windows))]
use utf8_decode::Decoder as Utf8Decoder;

pub trait LocalDecode<I>: Iterator<Item = u8> + Sized
where
    I: Iterator<Item = Result<char>>,
{
    fn local_console_decode(self) -> I;
    fn local_file_decode(self) -> I;
}

pub trait LocalEncode<I>: Iterator<Item = char> + Sized
where
    I: Iterator<Item = u8>,
{
    fn local_console_encode(self) -> I;
    fn local_file_encode(self) -> I;
}

#[cfg(windows)]
impl<T> LocalDecode<ByteDecoder<T>> for T
where
    T: Sized + Iterator<Item = u8>,
{
    fn local_console_decode(self) -> ByteDecoder<Self> {
        ByteDecoder::new(self, winnls::CP_OEMCP).unwrap()
    }

    fn local_file_decode(self) -> ByteDecoder<Self> {
        ByteDecoder::new(self, winnls::CP_ACP).unwrap()
    }
}

#[cfg(windows)]
impl<T: Iterator<Item = char>> LocalEncode<ByteEncoder<T>> for T {
    fn local_console_encode(self) -> ByteEncoder<T> {
        ByteEncoder::new(self, winnls::CP_OEMCP)
    }

    fn local_file_encode(self) -> ByteEncoder<T> {
        ByteEncoder::new(self, winnls::CP_ACP)
    }
}

#[cfg(not(windows))]
impl<T> LocalDecode<Utf8Decoder<T>> for T
where
    T: Sized + Iterator<Item = u8>,
{
    fn local_console_decode(self) -> Utf8Decoder<Self> {
        Utf8Decoder::new(self)
    }

    fn local_file_decode(self) -> Utf8Decoder<Self> {
        Utf8Decoder::new(self)
    }
}

#[cfg(not(windows))]
impl<T> LocalEncode<Utf8Encoder<T>> for T
where
    T: Sized + Iterator<Item = char>,
{
    fn local_console_encode(self) -> Utf8Encoder<Self> {
        Utf8Encoder::new(self)
    }

    fn local_file_encode(self) -> Utf8Encoder<Self> {
        Utf8Encoder::new(self)
    }
}

#[cfg(test)]
mod decode_tests {
    use crate::LocalDecode;

    #[test]
    fn test_basic_decode() {
        let iterator = (*b"Test").into_iter().local_console_decode();
        let cleaned = iterator.map(|c| c.unwrap());
        assert_eq!(cleaned.eq("Test".chars()), true);
    }

    #[cfg(not(windows))]
    #[test]
    fn test_invalid_decode() {
        let iterator = (*b"Te\xc3\x28st").into_iter().local_console_decode();
        let cleaned = iterator.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));
        assert_eq!(cleaned.eq("Te�st".chars()), true);
    }

    #[cfg(windows)]
    #[test]
    fn test_cp708_decode() {
        use crate::windows::ByteDecoder;

        let iterator = ByteDecoder::new((*b"T\x82st \xbf").into_iter(), 708).unwrap();
        let cleaned = iterator.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));
        let string = String::from_iter(cleaned);

        assert_eq!(string, "Tést ؟");
    }

    #[cfg(windows)]
    #[test]
    fn test_cp866_decode() {
        use crate::windows::ByteDecoder;

        let iterator = ByteDecoder::new((*b"\x92\xA5\xE1\xE2").into_iter(), 866).unwrap();
        let cleaned = iterator.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));
        let string = String::from_iter(cleaned);

        assert_eq!(string, "Тест");
    }

    #[cfg(windows)]
    #[test]
    fn test_cp932_decode() {
        use crate::windows::ByteDecoder;

        let iterator = ByteDecoder::new([140, 142].into_iter(), 932).unwrap();
        let cleaned = iterator.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));
        let string = String::from_iter(cleaned);

        assert_eq!(string, "月");
    }

    #[cfg(all(windows))]
    #[test]
    fn test_invalid_decode() {
        use crate::windows::ByteDecoder;

        let iterator = ByteDecoder::new((*b"Te\xd5st").into_iter(), 857).unwrap();
        let cleaned = iterator.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));
        let string = String::from_iter(cleaned);

        assert_eq!(string, "Te�st");
    }
}

#[cfg(test)]
mod encode_tests {
    use crate::LocalEncode;

    #[test]
    fn test_basic_encode() {
        let iterator = "Test".chars().into_iter().local_console_encode();
        let vec: Vec<u8> = iterator.collect();
        assert_eq!(vec, b"Test");
    }

    #[cfg(not(windows))]
    #[test]
    fn test_utf8_encode() {
        let iterator = "月".chars().into_iter().local_console_encode();
        let vec: Vec<u8> = iterator.collect();
        assert_eq!(vec, b"\xE6\x9C\x88");
    }

    #[cfg(windows)]
    #[test]
    fn test_cp708_encode() {
        use crate::windows::ByteEncoder;

        let iterator = ByteEncoder::new("Tést ؟".chars(), 708);
        let vec: Vec<u8> = iterator.collect();

        assert_eq!(vec.as_slice(), b"T\x82st \xbf");
    }

    #[cfg(windows)]
    #[test]
    fn test_cp866_encode() {
        use crate::windows::ByteEncoder;

        let iterator = ByteEncoder::new("Тест".chars(), 866);
        let vec: Vec<u8> = iterator.collect();

        assert_eq!(vec.as_slice(), b"\x92\xA5\xE1\xE2");
    }

    #[cfg(windows)]
    #[test]
    fn test_cp932_encode() {
        use crate::windows::ByteEncoder;

        let iterator = ByteEncoder::new("月".chars(), 932);
        let vec: Vec<u8> = iterator.collect();

        println!("{:?}", vec);
        assert_eq!(vec.as_slice(), b"\x8c\x8e");
    }
}
