use anyhow::{anyhow, Context, Result};
use blake2::digest::consts::U32;
use blake2::{Blake2b, Blake2s256};
use clg_ir::TrapCode;
use crypto_common::BlockSizeUser;
use ed25519_dalek::{Signature as EdSignature, VerifyingKey as EdVerifyingKey};
use k256::ecdsa::signature::Verifier as SecpVerifier;
use k256::ecdsa::{Signature as SecpSignature, VerifyingKey as SecpVerifyingKey};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::path::{Path, PathBuf};
use wasmtime as wt;
use wasmtime_wasi::sync::WasiCtxBuilder;

use super::helpers::{make_single_json_error, CommandError};
use crate::logging::{Logger, StageTimings};

pub fn run(file: PathBuf, invoke: String, json_errors: bool, logger: Logger) -> Result<()> {
    let mut timings = StageTimings::new();
    let engine = wt::Engine::new(&wt::Config::new())?;
    let module = {
        let _stage = timings.start(logger, "load_module");
        wt::Module::from_file(&engine, &file)
            .with_context(|| format!("loading {}", file.display()))?
    };
    let wasi = WasiCtxBuilder::new()
        .inherit_stdout()
        .inherit_stderr()
        .build();
    let mut store = wt::Store::new(&engine, wasi);
    let instance = {
        let _stage = timings.start(logger, "instantiate");
        let mut linker = wt::Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |cx| cx).context("linking WASI")?;
        add_env_stubs(&mut linker).context("linking env stubs")?;
        add_crypto_stubs(&mut linker).context("linking crypto stubs")?;
        linker
            .instantiate(&mut store, &module)
            .context("instantiating module")?
    };
    let func = instance
        .get_typed_func::<(), i32>(&mut store, &invoke)
        .with_context(|| format!("export `{}` not found or wrong type", invoke))?;

    match func.call(&mut store, ()) {
        Ok(result) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                if json_errors {
                    let json = make_single_json_error(
                        diag.code,
                        "runtime",
                        diag.message.clone(),
                        Path::new(&file),
                        diag.start,
                        diag.end,
                        diag.detail.clone(),
                    );
                    return Err(CommandError::json(json).into());
                } else {
                    let span = if diag.start != 0 || diag.end != 0 {
                        format!(" at {}..{}", diag.start, diag.end)
                    } else {
                        String::new()
                    };
                    return Err(anyhow!("{}{} (code {})", diag.message, span, diag.code));
                }
            }
            println!("{}", result);
            logger.summary(&timings);
            Ok(())
        }
        Err(trap) => {
            if let Some(diag) = extract_runtime_error(&instance, &mut store) {
                if json_errors {
                    let json = make_single_json_error(
                        diag.code,
                        "runtime",
                        diag.message.clone(),
                        Path::new(&file),
                        diag.start,
                        diag.end,
                        diag.detail.clone(),
                    );
                    return Err(CommandError::json(json).into());
                } else {
                    let span = if diag.start != 0 || diag.end != 0 {
                        format!(" at {}..{}", diag.start, diag.end)
                    } else {
                        String::new()
                    };
                    return Err(anyhow!("{}{} (code {})", diag.message, span, diag.code));
                }
            }
            if let Some(diag) = extract_wasmtime_limit_error(&trap) {
                if json_errors {
                    let json = make_single_json_error(
                        diag.code,
                        "runtime",
                        diag.message.clone(),
                        Path::new(&file),
                        0,
                        0,
                        None,
                    );
                    return Err(CommandError::json(json).into());
                } else {
                    return Err(anyhow!("{}", diag.message));
                }
            }
            Err(trap)
        }
    }
}

fn add_env_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
    linker.func_wrap(
        "clearlang_env",
        "env_time",
        |_caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>| -> i32 { 0 },
    )?;
    linker.func_wrap(
        "clearlang_env",
        "env_random",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, len: i32| -> Result<i32> {
            if len < 0 {
                set_runtime_error(&mut caller, TrapCode::InvalidUtf8);
                return Ok(0);
            }
            let len_u32 = len as u32;
            let heap_ptr = get_caller_global_i32(&mut caller, "__clg_heap_ptr")? as u32;
            let size = 4u32.saturating_add(len_u32);
            let next = (heap_ptr + size + 3) & !3;

            let memory = get_caller_memory(&mut caller)?;
            let mem_size = memory.data_size(&caller);
            if next as usize > mem_size {
                set_runtime_error(&mut caller, TrapCode::AllocatorOom);
                return Ok(0);
            }

            let data = memory.data_mut(&mut caller);
            let start = heap_ptr as usize;
            data[start..start + 4].copy_from_slice(&len_u32.to_le_bytes());
            if len_u32 > 0 {
                data[start + 4..start + 4 + len_u32 as usize].fill(0);
            }

            set_caller_global_i32(&mut caller, "__clg_heap_ptr", next as i32)?;
            Ok(heap_ptr as i32)
        },
    )?;
    Ok(())
}

fn add_crypto_stubs(linker: &mut wt::Linker<wasmtime_wasi::WasiCtx>) -> Result<()> {
    linker.func_wrap(
        "clearlang_crypto",
        "crypto_hash",
        |mut caller: wt::Caller<'_, wasmtime_wasi::WasiCtx>, alg: i32, data: i32| -> Result<i32> {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crypto_hash_stub(&mut caller, alg, data)
            }));
            match result {
                Ok(res) => res,
                Err(_) => {
                    set_runtime_error(&mut caller, TrapCode::CryptoMalformed);
                    Ok(0)
                }
            }
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
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crypto_hmac_stub(&mut caller, alg, key, data)
            }));
            match result {
                Ok(res) => res,
                Err(_) => {
                    set_runtime_error(&mut caller, TrapCode::CryptoMalformed);
                    Ok(0)
                }
            }
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
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crypto_verify_stub(&mut caller, alg, msg, sig, pk)
            }));
            match result {
                Ok(res) => res,
                Err(_) => {
                    set_runtime_error(&mut caller, TrapCode::CryptoMalformed);
                    Ok(0)
                }
            }
        },
    )?;
    Ok(())
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
    let len = u32::from_le_bytes(header.try_into().map_err(|_| ReadBufferError::InvalidLayout)?)
        as usize;
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

fn write_bytes<T>(caller: &mut wt::Caller<'_, T>, bytes: &[u8]) -> Result<i32> {
    let len_u32 = bytes.len() as u32;
    let heap_ptr = get_caller_global_i32(caller, "__clg_heap_ptr")? as u32;
    let size = 4u32.saturating_add(len_u32);
    let next = (heap_ptr + size + 3) & !3;

    let memory = get_caller_memory(caller)?;
    let mem_size = memory.data_size(&caller);
    if next as usize > mem_size {
        return Err(anyhow!("crypto output out of memory"));
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

fn set_runtime_error<T>(caller: &mut wt::Caller<'_, T>, code: TrapCode) {
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_code", code.as_i32());
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_start", 0);
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_end", 0);
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_detail", 0);
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

fn crypto_verify_bytes(
    alg: &str,
    msg: &[u8],
    sig: &[u8],
    pk: &[u8],
) -> Result<bool, CryptoError> {
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

fn crypto_hash_stub<T>(
    caller: &mut wt::Caller<'_, T>,
    alg_ptr: i32,
    data_ptr: i32,
) -> Result<i32> {
    let alg = match read_string(caller, alg_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
    };
    let data = match read_bytes(caller, data_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };
    match crypto_hash_bytes(alg.as_str(), &data) {
        Ok(digest) => match write_bytes(caller, &digest) {
            Ok(ptr) => Ok(ptr),
            Err(_) => {
                set_runtime_error(caller, TrapCode::AllocatorOom);
                Ok(0)
            }
        },
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
    let alg = match read_string(caller, alg_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
    };
    let key = match read_bytes(caller, key_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };
    let data = match read_bytes(caller, data_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };
    match crypto_hmac_bytes(alg.as_str(), &key, &data) {
        Ok(digest) => match write_bytes(caller, &digest) {
            Ok(ptr) => Ok(ptr),
            Err(_) => {
                set_runtime_error(caller, TrapCode::AllocatorOom);
                Ok(0)
            }
        },
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
    let alg = match read_string(caller, alg_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
    };
    let msg = match read_bytes(caller, msg_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };
    let sig = match read_bytes(caller, sig_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };
    let pk = match read_bytes(caller, pk_ptr) {
        Ok(value) => value,
        Err(ReadBufferError::InvalidLayout) => {
            set_runtime_error(caller, TrapCode::InvalidUtf8);
            return Ok(0);
        }
        Err(ReadBufferError::InvalidUtf8) => {
            set_runtime_error(caller, TrapCode::CryptoMalformed);
            return Ok(0);
        }
    };

    match crypto_verify_bytes(alg.as_str(), &msg, &sig, &pk) {
        Ok(ok) => Ok(if ok { 1 } else { 0 }),
        Err(err) => {
            set_runtime_error(caller, err.trap_code());
            Ok(0)
        }
    }
}

fn get_caller_memory<T>(caller: &mut wt::Caller<'_, T>) -> Result<wt::Memory> {
    match caller.get_export("memory") {
        Some(wt::Extern::Memory(mem)) => Ok(mem),
        _ => Err(anyhow!("missing memory export")),
    }
}

fn get_caller_global_i32<T>(caller: &mut wt::Caller<'_, T>, name: &str) -> Result<i32> {
    let global = match caller.get_export(name) {
        Some(wt::Extern::Global(global)) => global,
        _ => return Err(anyhow!("missing global `{}`", name)),
    };
    global
        .get(&mut *caller)
        .i32()
        .ok_or_else(|| anyhow!("global `{}` is not i32", name))
}

fn set_caller_global_i32<T>(caller: &mut wt::Caller<'_, T>, name: &str, value: i32) -> Result<()> {
    let global = match caller.get_export(name) {
        Some(wt::Extern::Global(global)) => global,
        _ => return Err(anyhow!("missing global `{}`", name)),
    };
    global
        .set(&mut *caller, wt::Val::I32(value))
        .context("set global")?;
    Ok(())
}

struct RuntimeErrorDiag {
    code: &'static str,
    message: String,
    start: usize,
    end: usize,
    detail: Option<String>,
}

fn extract_runtime_error<T>(
    instance: &wt::Instance,
    store: &mut wt::Store<T>,
) -> Option<RuntimeErrorDiag> {
    let code = get_global(instance, store, "__clg_runtime_error_code")?;
    if code == 0 {
        return None;
    }

    let mut start = get_global(instance, store, "__clg_runtime_error_start").unwrap_or(0) as usize;
    let mut end = get_global(instance, store, "__clg_runtime_error_end").unwrap_or(0) as usize;
    let detail = get_global(instance, store, "__clg_runtime_error_detail").unwrap_or(0);

    let (code_label, mut message, detail_label, reset_span) = match code {
        1 => {
            let label = match detail {
                1 => "ensure",
                _ => "require",
            };
            (
                "R000",
                format!("contract `{}` guard failed", label),
                Some(label.to_string()),
                false,
            )
        }
        2 => (
            "R001",
            "string allocator ran out of memory".to_string(),
            None,
            false,
        ),
        3 => (
            "R002",
            "runtime rejected invalid input".to_string(),
            None,
            false,
        ),
        4 => {
            let kind = match detail {
                1 => "Result",
                2 => "Enum",
                _ => "Option",
            };
            let tag = start;
            (
                "R003",
                format!("{kind} variant observed invalid tag {}", tag),
                Some(kind.to_string()),
                true,
            )
        }
        5 => ("R004", "runtime limits exceeded".to_string(), None, false),
        6 => ("R005", "unsigned integer overflow".to_string(), None, false),
        7 => (
            "R006",
            "crypto algorithm unsupported or unknown".to_string(),
            None,
            false,
        ),
        8 => (
            "R007",
            "crypto input length is invalid for selected algorithm".to_string(),
            None,
            false,
        ),
        9 => (
            "R008",
            "crypto input is malformed".to_string(),
            None,
            false,
        ),
        10 => (
            "R009",
            "collection bounds error".to_string(),
            None,
            false,
        ),
        11 => (
            "R010",
            "invalid collection handle".to_string(),
            None,
            false,
        ),
        _ => (
            "R999",
            format!("runtime trap with unknown code {}", code),
            None,
            false,
        ),
    };

    if reset_span {
        start = 0;
        end = 0;
    }

    if code_label == "R000" && start == 0 && end == 0 {
        message.push_str(" (no span available)");
    }

    Some(RuntimeErrorDiag {
        code: code_label,
        message,
        start,
        end,
        detail: detail_label,
    })
}
fn get_global<T>(instance: &wt::Instance, store: &mut wt::Store<T>, name: &str) -> Option<i32> {
    let global = instance.get_global(&mut *store, name)?;
    global.get(&mut *store).i32()
}

fn extract_wasmtime_limit_error(trap: &anyhow::Error) -> Option<RuntimeErrorDiag> {
    let trap = trap
        .chain()
        .find_map(|err| err.downcast_ref::<wt::Trap>())?;
    match trap {
        wt::Trap::OutOfFuel | wt::Trap::Interrupt => Some(RuntimeErrorDiag {
            code: "R004",
            message: "runtime limits exceeded".to_string(),
            start: 0,
            end: 0,
            detail: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_hash_sha256_vector() {
        let digest = crypto_hash_bytes("sha256", b"abc").expect("hash ok");
        let expected = hex::decode(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )
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
        let expected = hex::decode(
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8",
        )
        .expect("hex");
        assert_eq!(digest, expected);
    }

    #[test]
    fn crypto_verify_ed25519_vector_true() {
        let pk = hex::decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        )
        .expect("pk");
        let sig = hex::decode("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
            .expect("sig");
        let ok = crypto_verify_bytes("ed25519", b"", &sig, &pk).expect("verify ok");
        assert!(ok, "expected ed25519 vector to verify");
    }

    #[test]
    fn crypto_verify_ed25519_wrong_sig_false() {
        let pk = hex::decode(
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        )
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
