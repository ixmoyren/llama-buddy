use std::{
    env::home_dir,
    ffi::{OsString, c_void},
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    ptr, slice,
};

use crate::dir::{BaseDirs, UserDirs};
use windows_sys::{
    Win32::{
        Foundation::S_OK,
        Globalization::lstrlenW,
        System::Com::CoTaskMemFree,
        UI::{Shell, Shell::KF_FLAG_DONT_VERIFY},
    },
    core::{GUID, PWSTR},
};

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

pub fn user_dirs() -> Option<UserDirs> {
    let home = home_dir()?;
    let audio = know_guid(Shell::FOLDERID_Music);
    let desktop = know_guid(Shell::FOLDERID_Desktop);
    let document = know_guid(Shell::FOLDERID_Documents);
    let download = know_guid(Shell::FOLDERID_Downloads);
    let picture = know_guid(Shell::FOLDERID_Pictures);
    let public = know_guid(Shell::FOLDERID_Public);
    let video = know_guid(Shell::FOLDERID_Videos);
    let template = know_guid(Shell::FOLDERID_Templates);
    Some(UserDirs {
        home,
        audio,
        desktop,
        document,
        download,
        font: None,
        picture,
        public,
        template,
        video,
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

fn know_guid(folder_id: GUID) -> Option<PathBuf> {
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
            Some(PathBuf::from(os_str))
        } else {
            CoTaskMemFree(path_ptr as *const c_void);
            None
        }
    }
}
