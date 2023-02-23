use crate::TArray;
use std::{
    char::decode_utf16,
    fmt,
    ops::{Deref, DerefMut},
};

#[repr(transparent)]
pub struct FString {
    data: TArray<u16>,
}

impl fmt::Display for FString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chr in decode_utf16(self.data.iter().copied()) {
            match chr {
                Ok(chr) => write!(f, "{chr}")?,
                Err(_) => write!(f, "{}", char::REPLACEMENT_CHARACTER)?,
            }
        }

        Ok(())
    }
}

impl Deref for FString {
    type Target = TArray<u16>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for FString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl PartialEq<str> for FString {
    fn eq(&self, other: &str) -> bool {
        self.data.len() == other.len()
            && other
                .encode_utf16()
                .zip(self.data.as_slice())
                .all(|(a, b)| a == *b)
    }
}
