// [INPUT]: 标准库文件系统、当前用户身份与 Windows ACL API。
// [OUTPUT]: 私有目录/文件、链接与所有权检查、同步原子写入。
// [POS]: 身份保险库的磁盘边界；权限在秘密写入之前收紧，不修改用户系统策略。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use std::{fs, io::Write, path::Path};
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
        fs::create_dir(path).map_err(|_| "无法创建身份目录。")?;
    }
    protect(path)
}
pub fn create(path: &Path) -> Result<fs::File, String> {
    parents(path.parent().ok_or("身份路径无效。")?)?;
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| "无法创建身份暂存文件。")?;
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
