// [INPUT]: ring 的 AES-256-GCM/系统随机数、保险库主密钥和标准流。
// [OUTPUT]: 独立归档密钥的有界认证帧；验证结尾、序号、帧长度与截断。
// [POS]: 压缩流的加密边界；每个归档重新生成数据密钥，不自实现密码算法。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    rand::{SecureRandom, SystemRandom},
};
use std::io::{self, Read, Write};
use zeroize::Zeroizing;
const MAGIC: &[u8; 8] = b"BAGAID01";
const BLOCK: usize = 64 * 1024;
fn err() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "身份归档无法验证，未修改当前身份。",
    )
}
fn key(bytes: &[u8]) -> io::Result<LessSafeKey> {
    Ok(LessSafeKey::new(
        UnboundKey::new(&aead::AES_256_GCM, bytes).map_err(|_| err())?,
    ))
}
fn nonce(counter: u64) -> Nonce {
    let mut n = [0; 12];
    n[4..].copy_from_slice(&counter.to_be_bytes());
    Nonce::assume_unique_for_key(n)
}
pub fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut out = [0; N];
    SystemRandom::new()
        .fill(&mut out)
        .map_err(|_| "无法生成安全随机数。")?;
    Ok(out)
}
pub struct Seal<W: Write> {
    out: W,
    key: LessSafeKey,
    counter: u64,
}
impl<W: Write> Seal<W> {
    pub fn new(mut out: W, master: &[u8]) -> io::Result<Self> {
        let secret = Zeroizing::new(random::<32>().map_err(|_| err())?);
        let n = random::<12>().map_err(|_| err())?;
        let mut wrapped = secret.to_vec();
        key(master)?
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(n),
                Aad::from(MAGIC),
                &mut wrapped,
            )
            .map_err(|_| err())?;
        out.write_all(MAGIC)?;
        out.write_all(&n)?;
        out.write_all(&wrapped)?;
        Ok(Self {
            out,
            key: key(secret.as_ref())?,
            counter: 0,
        })
    }
    fn frame(&mut self, plain: &[u8]) -> io::Result<()> {
        let length = ((plain.len() + 16) as u32).to_be_bytes();
        let mut buffer = plain.to_vec();
        self.key
            .seal_in_place_append_tag(nonce(self.counter), Aad::from(length), &mut buffer)
            .map_err(|_| err())?;
        self.counter = self.counter.checked_add(1).ok_or_else(err)?;
        self.out.write_all(&length)?;
        self.out.write_all(&buffer)
    }
    pub fn finish(mut self) -> io::Result<W> {
        self.frame(&[])?;
        self.out.flush()?;
        Ok(self.out)
    }
}
impl<W: Write> Write for Seal<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for block in bytes.chunks(BLOCK) {
            self.frame(block)?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}
pub struct Open<R: Read> {
    input: R,
    key: LessSafeKey,
    counter: u64,
    buffer: Zeroizing<Vec<u8>>,
    offset: usize,
    ended: bool,
}
impl<R: Read> Open<R> {
    pub fn new(mut input: R, master: &[u8]) -> io::Result<Self> {
        let mut magic = [0; 8];
        input.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(err());
        }
        let mut n = [0; 12];
        input.read_exact(&mut n)?;
        let mut wrapped = Zeroizing::new(vec![0; 48]);
        input.read_exact(&mut wrapped)?;
        let k = key(master)?;
        let plain = k
            .open_in_place(
                Nonce::assume_unique_for_key(n),
                Aad::from(MAGIC),
                &mut wrapped,
            )
            .map_err(|_| err())?;
        Ok(Self {
            input,
            key: key(plain)?,
            counter: 0,
            buffer: Zeroizing::new(Vec::new()),
            offset: 0,
            ended: false,
        })
    }
}
impl<R: Read> Read for Open<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.offset == self.buffer.len() {
            if self.ended {
                return Ok(0);
            }
            let mut size = [0; 4];
            self.input.read_exact(&mut size)?;
            let length = u32::from_be_bytes(size) as usize;
            if !(16..=BLOCK + 16).contains(&length) {
                return Err(err());
            }
            self.buffer.clear();
            self.buffer.resize(length, 0);
            self.input.read_exact(&mut self.buffer)?;
            let plain = self
                .key
                .open_in_place(nonce(self.counter), Aad::from(size), &mut self.buffer)
                .map_err(|_| err())?;
            let len = plain.len();
            self.buffer.truncate(len);
            self.offset = 0;
            self.counter = self.counter.checked_add(1).ok_or_else(err)?;
            if len == 0 {
                let mut tail = [0];
                if self.input.read(&mut tail)? != 0 {
                    return Err(err());
                }
                self.ended = true;
                return Ok(0);
            }
        }
        let length = out.len().min(self.buffer.len() - self.offset);
        out[..length].copy_from_slice(&self.buffer[self.offset..self.offset + length]);
        self.offset += length;
        Ok(length)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_stream_rejects_damage_and_truncation() {
        let master = random::<32>().unwrap();
        let data = vec![77; BLOCK * 3 + 11];
        let mut seal = Seal::new(Vec::new(), &master).unwrap();
        seal.write_all(&data).unwrap();
        let saved = seal.finish().unwrap();
        let mut opened = Vec::new();
        Open::new(&saved[..], &master)
            .unwrap()
            .read_to_end(&mut opened)
            .unwrap();
        assert_eq!(opened, data);
        for position in [0, 30, 80, saved.len() - 1] {
            let mut broken = saved.clone();
            broken[position] ^= 1;
            let result =
                Open::new(&broken[..], &master).and_then(|mut r| r.read_to_end(&mut Vec::new()));
            assert!(result.is_err());
        }
        assert!(Open::new(&saved[..saved.len() - 16], &master)
            .and_then(|mut r| r.read_to_end(&mut Vec::new()))
            .is_err());
        let mut extra = saved.clone();
        extra.push(0);
        assert!(Open::new(&extra[..], &master)
            .and_then(|mut r| r.read_to_end(&mut Vec::new()))
            .is_err());
    }
}
