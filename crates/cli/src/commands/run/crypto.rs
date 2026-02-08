use anyhow::Result;
use blake2::digest::consts::U32;
use blake2::{Blake2b, Blake2s256};
use clg_ir::TrapCode;
use crypto_common::BlockSizeUser;
use ed25519_dalek::{Signature as EdSignature, VerifyingKey as EdVerifyingKey};
use k256::ecdsa::signature::Verifier as SecpVerifier;
use k256::ecdsa::{Signature as SecpSignature, VerifyingKey as SecpVerifyingKey};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use wasmtime as wt;

use super::runtime_error::set_runtime_error;
use super::wasm_state::{get_caller_global_i32, get_caller_memory, set_caller_global_i32};

pub(super) fn add_crypto_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
    linker.func_wrap(
        "clearlang_crypto",
        "crypto_hash",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, alg: i32, data: i32| -> Result<i32> {
            catch_crypto_panic(&mut caller, |caller| crypto_hash_stub(caller, alg, data))
        },
    )?;
    linker.func_wrap(
        "clearlang_crypto",
        "crypto_hmac",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>,
         alg: i32,
         key: i32,
         data: i32|
         -> Result<i32> {
            catch_crypto_panic(&mut caller, |caller| {
                crypto_hmac_stub(caller, alg, key, data)
            })
        },
    )?;
    linker.func_wrap(
        "clearlang_crypto",
        "crypto_verify",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>,
         alg: i32,
         msg: i32,
         sig: i32,
         pk: i32|
         -> Result<i32> {
            catch_crypto_panic(&mut caller, |caller| {
                crypto_verify_stub(caller, alg, msg, sig, pk)
            })
        },
    )?;
    Ok(())
}

fn catch_crypto_panic<T, F>(caller: &mut wt::Caller<'_, T>, f: F) -> Result<i32>
where
    F: FnOnce(&mut wt::Caller<'_, T>) -> Result<i32>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(caller)));
    match result {
        Ok(res) => res,
        Err(_) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            Ok(0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CryptoError {
    Unsupported,
    InvalidLength,
    Malformed,
}

impl CryptoError {
    fn trap_code(self) -> TrapCode {
        match self {
            CryptoError::Unsupported => TrapCode::CryptoUnsupported,
            CryptoError::InvalidLength => TrapCode::CryptoInvalidLength,
            CryptoError::Malformed => TrapCode::CryptoMalformed,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ReadBufferError {
    InvalidLayout,
    InvalidUtf8,
}

fn read_bytes<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Result<Vec<u8>, ReadBufferError> {
    if ptr < 0 || (ptr & 3) != 0 {
        return Err(ReadBufferError::InvalidLayout);
    }
    let memory = get_caller_memory(caller).map_err(|_| ReadBufferError::InvalidLayout)?;
    let data = memory.data(caller);
    let start = ptr as usize;
    if start + 4 > data.len() {
        return Err(ReadBufferError::InvalidLayout);
    }
    let header = data
        .get(start..start + 4)
        .ok_or(ReadBufferError::InvalidLayout)?;
    let len = u32::from_le_bytes(
        header
            .try_into()
            .map_err(|_| ReadBufferError::InvalidLayout)?,
    ) as usize;
    let end = start + 4 + len;
    if end > data.len() {
        return Err(ReadBufferError::InvalidLayout);
    }
    Ok(data[start + 4..end].to_vec())
}

fn read_string<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Result<String, ReadBufferError> {
    let bytes = read_bytes(caller, ptr)?;
    String::from_utf8(bytes).map_err(|_| ReadBufferError::InvalidUtf8)
}

fn read_bytes_param<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Option<Vec<u8>> {
    match read_bytes(caller, ptr) {
        Ok(value) => Some(value),
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            None
        }
    }
}

fn read_string_param<T>(caller: &mut wt::Caller<'_, T>, ptr: i32) -> Option<String> {
    match read_string(caller, ptr) {
        Ok(value) => Some(value),
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            None
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            None
        }
    }
}

fn write_bytes<T>(caller: &mut wt::Caller<'_, T>, bytes: &[u8]) -> Result<i32> {
    let len_u32 = bytes.len() as u32;
    let heap_ptr = get_caller_global_i32(caller, "__clg_heap_ptr")? as u32;
    let size = 4u32.saturating_add(len_u32);
    let next = (heap_ptr + size + 3) & !3;

    let memory = get_caller_memory(caller)?;
    let mem_size = memory.data_size(&caller);
    if next as usize > mem_size {
        return Err(anyhow::anyhow!("crypto output out of memory"));
    }

    let data = memory.data_mut(&mut *caller);
    let start = heap_ptr as usize;
    data[start..start + 4].copy_from_slice(&len_u32.to_le_bytes());
    if len_u32 > 0 {
        data[start + 4..start + 4 + len_u32 as usize].copy_from_slice(bytes);
    }

    set_caller_global_i32(caller, "__clg_heap_ptr", next as i32)?;
    Ok(heap_ptr as i32)
}

type Blake2b256 = Blake2b<U32>;

fn digest_bytes<D: Digest>(data: &[u8]) -> Vec<u8> {
    let mut hasher = D::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

fn hmac_bytes<D>(key: &[u8], data: &[u8]) -> Vec<u8>
where
    D: Digest + BlockSizeUser + Clone + Default,
{
    let block_size = D::block_size();
    let mut key_block = vec![0u8; block_size];
    if key.len() > block_size {
        let hashed = digest_bytes::<D>(key);
        key_block[..hashed.len()].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = vec![0x36u8; block_size];
    let mut opad = vec![0x5cu8; block_size];
    for i in 0..block_size {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = D::new();
    inner.update(&ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = D::new();
    outer.update(&opad);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn crypto_hash_bytes(alg: &str, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    match alg {
        "sha256" => Ok(digest_bytes::<Sha256>(data)),
        "keccak256" => Ok(digest_bytes::<Keccak256>(data)),
        "blake2b256" => Ok(digest_bytes::<Blake2b256>(data)),
        "blake2s256" => Ok(digest_bytes::<Blake2s256>(data)),
        _ => Err(CryptoError::Unsupported),
    }
}

fn crypto_hmac_bytes(alg: &str, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    match alg {
        "sha256" => Ok(hmac_bytes::<Sha256>(key, data)),
        "keccak256" => Ok(hmac_bytes::<Keccak256>(key, data)),
        "blake2b256" => Ok(hmac_bytes::<Blake2b256>(key, data)),
        "blake2s256" => Ok(hmac_bytes::<Blake2s256>(key, data)),
        _ => Err(CryptoError::Unsupported),
    }
}

fn crypto_verify_bytes(alg: &str, msg: &[u8], sig: &[u8], pk: &[u8]) -> Result<bool, CryptoError> {
    match alg {
        "ed25519" => {
            if pk.len() != 32 || sig.len() != 64 {
                return Err(CryptoError::InvalidLength);
            }
            let pk_bytes: &[u8; 32] = pk.try_into().map_err(|_| CryptoError::InvalidLength)?;
            let vk = EdVerifyingKey::from_bytes(pk_bytes).map_err(|_| CryptoError::Malformed)?;
            let sig = EdSignature::try_from(sig).map_err(|_| CryptoError::Malformed)?;
            Ok(vk.verify_strict(msg, &sig).is_ok())
        }
        "secp256k1" => {
            if !matches!(pk.len(), 33 | 65) || sig.len() != 64 {
                return Err(CryptoError::InvalidLength);
            }
            let vk = SecpVerifyingKey::from_sec1_bytes(pk).map_err(|_| CryptoError::Malformed)?;
            let sig = SecpSignature::from_slice(sig).map_err(|_| CryptoError::Malformed)?;
            Ok(vk.verify(msg, &sig).is_ok())
        }
        _ => Err(CryptoError::Unsupported),
    }
}

fn write_crypto_output<T>(caller: &mut wt::Caller<'_, T>, bytes: &[u8]) -> Result<i32> {
    match write_bytes(caller, bytes) {
        Ok(ptr) => Ok(ptr),
        Err(_) => {
            set_runtime_error(caller, TrapCode::AllocatorOom);
            Ok(0)
        }
    }
}

fn crypto_hash_stub<T>(caller: &mut wt::Caller<'_, T>, alg_ptr: i32, data_ptr: i32) -> Result<i32> {
    let Some(alg) = read_string_param(caller, alg_ptr) else {
        return Ok(0);
    };
    let Some(data) = read_bytes_param(caller, data_ptr) else {
        return Ok(0);
    };

    match crypto_hash_bytes(alg.as_str(), &data) {
        Ok(digest) => write_crypto_output(caller, &digest),
        Err(err) => {
            set_runtime_error(caller, err.trap_code());
            Ok(0)
        }
    }
}

fn crypto_hmac_stub<T>(
    caller: &mut wt::Caller<'_, T>,
    alg_ptr: i32,
    key_ptr: i32,
    data_ptr: i32,
) -> Result<i32> {
    let Some(alg) = read_string_param(caller, alg_ptr) else {
        return Ok(0);
    };
    let Some(key) = read_bytes_param(caller, key_ptr) else {
        return Ok(0);
    };
    let Some(data) = read_bytes_param(caller, data_ptr) else {
        return Ok(0);
    };

    match crypto_hmac_bytes(alg.as_str(), &key, &data) {
        Ok(digest) => write_crypto_output(caller, &digest),
        Err(err) => {
            set_runtime_error(caller, err.trap_code());
            Ok(0)
        }
    }
}

fn crypto_verify_stub<T>(
    caller: &mut wt::Caller<'_, T>,
    alg_ptr: i32,
    msg_ptr: i32,
    sig_ptr: i32,
    pk_ptr: i32,
) -> Result<i32> {
    let Some(alg) = read_string_param(caller, alg_ptr) else {
        return Ok(0);
    };
    let Some(msg) = read_bytes_param(caller, msg_ptr) else {
        return Ok(0);
    };
    let Some(sig) = read_bytes_param(caller, sig_ptr) else {
        return Ok(0);
    };
    let Some(pk) = read_bytes_param(caller, pk_ptr) else {
        return Ok(0);
    };

    match crypto_verify_bytes(alg.as_str(), &msg, &sig, &pk) {
        Ok(ok) => Ok(if ok { 1 } else { 0 }),
        Err(err) => {
            set_runtime_error(caller, err.trap_code());
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_hash_sha256_vector() {
        let digest = crypto_hash_bytes("sha256", b"abc").expect("hash ok");
        let expected =
            hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
                .expect("hex");
        assert_eq!(digest, expected);
    }

    #[test]
    fn crypto_hmac_sha256_vector() {
        let digest = crypto_hmac_bytes(
            "sha256",
            b"key",
            b"The quick brown fox jumps over the lazy dog",
        )
        .expect("hmac ok");
        let expected =
            hex::decode("f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8")
                .expect("hex");
        assert_eq!(digest, expected);
    }

    #[test]
    fn crypto_verify_ed25519_vector_true() {
        let pk = hex::decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .expect("pk");
        let sig = hex::decode("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
            .expect("sig");
        let ok = crypto_verify_bytes("ed25519", b"", &sig, &pk).expect("verify ok");
        assert!(ok, "expected ed25519 vector to verify");
    }

    #[test]
    fn crypto_verify_ed25519_wrong_sig_false() {
        let pk = hex::decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .expect("pk");
        let mut sig = hex::decode("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
            .expect("sig");
        sig[0] ^= 0x01;
        let ok = crypto_verify_bytes("ed25519", b"", &sig, &pk).expect("verify ok");
        assert!(!ok, "expected wrong signature to fail");
    }

    #[test]
    fn crypto_verify_invalid_length_reports_error() {
        let err = crypto_verify_bytes("ed25519", b"", &[0u8; 63], &[0u8; 32])
            .expect_err("invalid length");
        assert_eq!(err, CryptoError::InvalidLength);
    }

    #[test]
    fn crypto_verify_malformed_secp256k1_reports_error() {
        let err = crypto_verify_bytes("secp256k1", b"msg", &[0u8; 64], &[0u8; 33])
            .expect_err("malformed input");
        assert_eq!(err, CryptoError::Malformed);
    }
}
