// [INPUT]: 标准库文件系统、当前用户身份与 Windows ACL API。
// [OUTPUT]: 私有目录/文件、链接与所有权检查、同步原子写入。
// [POS]: 身份保险库的磁盘边界；Windows 创建时明确用户所有者/DACL，既有文件仍拒绝所有者变化。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use std::{fs, io::Write, path::Path};
#[cfg(windows)]
fn windows_create<T>(
    path: &Path,
    create: impl FnOnce(
        *const u16,
        *const windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    ) -> Result<T, String>,
) -> Result<T, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };
    let sid = crate::windows::current_sid().ok_or("无法读取当前用户 SID。")?;
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: 0x1f01ff,
        grfAccessMode: SET_ACCESS,
        grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_USER,
            ptstrName: sid.as_ptr() as *mut u16,
        },
    };
    let mut acl = std::ptr::null_mut();
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        if SetEntriesInAclW(1, &entry, std::ptr::null(), &mut acl) != 0 {
            return Err("无法保护身份文件权限。".into());
        }
        let mut descriptor: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let pointer = (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast();
        let result = if InitializeSecurityDescriptor(pointer, 1) == 0
            || SetSecurityDescriptorOwner(pointer, sid.as_ptr() as *mut _, 0) == 0
            || SetSecurityDescriptorDacl(pointer, 1, acl, 0) == 0
            || SetSecurityDescriptorControl(pointer, SE_DACL_PROTECTED, SE_DACL_PROTECTED) == 0
        {
            Err("无法保护身份文件权限。".into())
        } else {
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: pointer,
                bInheritHandle: 0,
            };
            create(wide.as_ptr(), &attributes)
        };
        LocalFree(acl.cast());
        result
    }
}
pub fn safe(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "身份文件无法读取。")?;
    if crate::cleaner::linked(&meta) || (!meta.is_file() && !meta.is_dir()) {
        return Err("身份路径包含链接或特殊文件。".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        extern "C" {
            fn getuid() -> u32;
        }
        if meta.uid() != unsafe { getuid() } {
            return Err("身份文件不属于当前用户。".into());
        }
    }
    #[cfg(windows)]
    if !crate::windows::owned(path) {
        return Err("身份文件不属于当前用户。".into());
    }
    Ok(())
}
pub fn parents(path: &Path) -> Result<(), String> {
    let mut cursor = Some(path);
    while let Some(p) = cursor {
        if p.exists() || fs::symlink_metadata(p).is_ok() {
            let m = fs::symlink_metadata(p).map_err(|_| "身份路径无法检查。")?;
            if crate::cleaner::linked(&m) || !m.is_dir() {
                return Err("身份路径包含链接或特殊文件。".into());
            }
        }
        cursor = p.parent();
    }
    Ok(())
}
pub fn protect(path: &Path) -> Result<(), String> {
    safe(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| "无法保护身份文件权限。")?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::{
                Authorization::*, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
                SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            },
        };
        let sid = crate::windows::current_sid().ok_or("无法读取当前用户 SID。")?;
        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: 0x1f01ff,
            grfAccessMode: SET_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: std::ptr::null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_USER,
                ptstrName: sid.as_ptr() as *mut u16,
            },
        };
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut acl = std::ptr::null_mut();
        unsafe {
            if SetEntriesInAclW(1, &entry, std::ptr::null(), &mut acl) != 0 {
                return Err("无法保护身份文件权限。".into());
            }
            let status = SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null(),
            );
            LocalFree(acl.cast());
            if status != 0 {
                return Err("无法保护身份文件权限。".into());
            }
        }
    }
    Ok(())
}
pub fn directory(path: &Path) -> Result<(), String> {
    parents(path)?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                directory(parent)?;
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(path)
                .map_err(|_| "无法创建身份目录。")?;
        }
        #[cfg(windows)]
        windows_create(path, |wide, attributes| {
            if unsafe {
                windows_sys::Win32::Storage::FileSystem::CreateDirectoryW(wide, attributes)
            } == 0
            {
                Err("无法创建身份目录。".into())
            } else {
                Ok(())
            }
        })?;
    }
    protect(path)
}
pub fn create(path: &Path) -> Result<fs::File, String> {
    parents(path.parent().ok_or("身份路径无效。")?)?;
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| "无法创建身份暂存文件。")?
    };
    #[cfg(windows)]
    let file = windows_create(path, |wide, attributes| {
        use std::os::windows::io::FromRawHandle;
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::*,
        };
        let handle = unsafe {
            CreateFileW(
                wide,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ,
                attributes,
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err("无法创建身份暂存文件。".into())
        } else {
            Ok(unsafe { fs::File::from_raw_handle(handle) })
        }
    })?;
    if let Err(e) = protect(path) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    Ok(file)
}
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        safe(path)?;
    }
    let temp = path.with_file_name(format!(".baga-{}", super::new_id()?));
    let result = (|| {
        let mut f = create(&temp)?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| "无法保存身份文件。")?;
        drop(f);
        fs::rename(&temp, path).map_err(|_| "无法提交身份文件。")?;
        sync_parent(path);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(p) = path.parent() {
        if let Ok(f) = fs::File::open(p) {
            let _ = f.sync_all();
        }
    }
    let _ = path;
}
