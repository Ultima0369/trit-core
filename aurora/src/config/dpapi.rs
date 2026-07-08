//! Windows DPAPI encryption/decryption bindings.
//!
//! Each FFI function carries its own `#[allow(unsafe_code)]` — the narrowest
//! possible scope. No other module in the crate uses `unsafe`.

use crate::percept::ConfigError;

/// Encrypt data using Windows DPAPI (user-level, CRYPTPROTECT_UI_FORBIDDEN).
#[allow(unsafe_code)]
pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>, ConfigError> {
    use std::mem;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Security::Cryptography;

    let data_in = Cryptography::CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut data_out: Cryptography::CRYPT_INTEGER_BLOB = unsafe { mem::zeroed() };

    let ok = unsafe {
        Cryptography::CryptProtectData(
            &data_in,
            windows_sys::w!("aurora-config"),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0x1, // CRYPTPROTECT_UI_FORBIDDEN
            &mut data_out,
        )
    };

    if ok == 0 {
        let err = unsafe { Foundation::GetLastError() };
        return Err(ConfigError::Dpapi(format!(
            "CryptProtectData failed: {err}"
        )));
    }

    // Sanity-cap the OS-reported length before slicing — a pathological return
    // value must not cause from_raw_parts to read gigabytes.
    const MAX_DPAPI_BLOB: usize = 16 * 1024 * 1024;
    if data_out.cbData as usize > MAX_DPAPI_BLOB {
        unsafe {
            windows_sys::Win32::Foundation::LocalFree(data_out.pbData as *mut std::ffi::c_void)
        };
        return Err(ConfigError::Dpapi(format!(
            "CryptProtectData returned oversized blob: {} bytes",
            data_out.cbData
        )));
    }

    let result =
        unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec() };
    unsafe { windows_sys::Win32::Foundation::LocalFree(data_out.pbData as *mut std::ffi::c_void) };
    Ok(result)
}

/// Decrypt data using Windows DPAPI (user-level, CRYPTPROTECT_UI_FORBIDDEN).
#[allow(unsafe_code)]
pub fn decrypt(cipher: &[u8]) -> Result<Vec<u8>, ConfigError> {
    use std::mem;
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Security::Cryptography;

    let data_in = Cryptography::CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut data_out: Cryptography::CRYPT_INTEGER_BLOB = unsafe { mem::zeroed() };

    let ok = unsafe {
        Cryptography::CryptUnprotectData(
            &data_in,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0x1,
            &mut data_out,
        )
    };

    if ok == 0 {
        let err = unsafe { Foundation::GetLastError() };
        return Err(ConfigError::Dpapi(format!(
            "CryptUnprotectData failed: {err}. Config may belong to different user/machine."
        )));
    }

    const MAX_DPAPI_BLOB: usize = 16 * 1024 * 1024;
    if data_out.cbData as usize > MAX_DPAPI_BLOB {
        unsafe {
            windows_sys::Win32::Foundation::LocalFree(data_out.pbData as *mut std::ffi::c_void)
        };
        return Err(ConfigError::Dpapi(format!(
            "CryptUnprotectData returned oversized blob: {} bytes",
            data_out.cbData
        )));
    }

    let result =
        unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec() };
    unsafe { windows_sys::Win32::Foundation::LocalFree(data_out.pbData as *mut std::ffi::c_void) };
    Ok(result)
}
