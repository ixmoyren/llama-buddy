use std::env::home_dir;
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::{ptr, slice};

use crate::BaseDirs;
use windows_sys::core::GUID;
use windows_sys::core::PWSTR;
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::Globalization::lstrlenW;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell;
use windows_sys::Win32::UI::Shell::KF_FLAG_DONT_VERIFY;

pub fn base_dirs() -> Option<BaseDirs> {
    let home = home_dir()?;
    let data = from_guid(Shell::FOLDERID_RoamingAppData, || {
        home.join("AppData/Roaming")
    });
    let data_local = from_guid(Shell::FOLDERID_LocalAppData, || home.join("AppData/Local"));
    let cache = data_local.join("Temp");
    let config = data.clone();
    let config_local = data_local.clone();
    let executable = None;
    let preference = None;
    let runtime = None;
    let state = None;
    Some(BaseDirs {
        home,
        cache,
        config,
        config_local,
        data,
        data_local,
        executable,
        preference,
        runtime,
        state,
    })
}

fn from_guid(folder_id: GUID, f: impl FnOnce() -> PathBuf) -> PathBuf {
    unsafe {
        let mut path_ptr: PWSTR = ptr::null_mut();
        let result = Shell::SHGetKnownFolderPath(
            &folder_id,
            KF_FLAG_DONT_VERIFY as u32,
            ptr::null_mut(),
            &mut path_ptr,
        );
        if result == S_OK {
            let len = lstrlenW(path_ptr) as usize;
            let path = slice::from_raw_parts(path_ptr, len);
            let os_str: OsString = OsStrExt::from_wide(path);
            CoTaskMemFree(path_ptr as *const c_void);
            PathBuf::from(os_str)
        } else {
            CoTaskMemFree(path_ptr as *const c_void);
            f()
        }
    }
}
