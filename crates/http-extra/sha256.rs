use crate::HttpExtraError;
use base64ct::{Base64, Encoding};
use faster_hex::hex_decode;
use memmap2::Mmap;
use sha2::{Digest, Sha256};
use std::{fs::File, path::Path};

pub fn checksum(file: impl AsRef<Path>, digest: impl AsRef<str>) -> Result<bool, HttpExtraError> {
    let file = File::open(file)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let hash = sha2::Sha256::digest(&mmap[..]);
    let digest = digest.as_ref().as_bytes();
    let mut digest_byte = vec![0u8; digest.len() / 2];
    hex_decode(digest, digest_byte.as_mut_slice())?;
    Ok(hash.as_slice().eq(&digest_byte))
}

pub fn digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    Base64::encode_string(&hash)
}

#[cfg(test)]
mod tests {
    use super::checksum;
    use std::io::Write;

    #[test]
    fn test_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path();
        let text = dir_path.join("1.txt");
        let mut file = std::fs::File::create(&text).unwrap();
        let hello = b"Hello, World!";
        file.write_all(hello).unwrap();
        assert!(
            checksum(
                text,
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
            )
            .unwrap()
        )
    }
}
