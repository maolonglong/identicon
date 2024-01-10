use md5::{Digest, Md5};

pub fn md5(data: &[u8]) -> [u8; 16] {
    // https://github.com/rust-lang/rust-analyzer/issues/15242
    let mut hasher = <Md5 as Digest>::new();
    hasher.update(data);
    hasher.finalize().into()
}
