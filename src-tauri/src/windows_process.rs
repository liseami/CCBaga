// [INPUT]: Win32 Toolhelp/进程句柄、当前用户 SID、精确 CLI 入口与独立浏览器资料路径。
// [OUTPUT]: Claude 进程识别、创建时间/所有权复核后的结束，以及按完整资料参数正常关闭浏览器。
// [POS]: Windows 进程边界；参数仅用于原生判定，不返回 UI；不对其他用户或名称相似的程序执行终止。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{current_sid, input, process_sid, response};
use crate::cleaner::ClaudeProcess;
use serde_json::json;
use std::{ffi::c_char, path::Path};
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree, FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM},
    System::{
        Diagnostics::{
            Debug::ReadProcessMemory,
            ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
        },
        Threading::{
            GetProcessTimes, IsWow64Process, OpenProcess, QueryFullProcessImageNameW,
            TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            PROCESS_VM_READ,
        },
    },
    UI::{
        Shell::CommandLineToArgvW,
        WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE},
    },
};
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Handle {
    fn open(pid: u32, terminate: bool) -> Option<Self> {
        let flags = PROCESS_QUERY_LIMITED_INFORMATION
            | PROCESS_VM_READ
            | if terminate { PROCESS_TERMINATE } else { 0 };
        let handle = unsafe { OpenProcess(flags, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let handle = Self(handle);
        if unsafe { process_sid(handle.0) }.as_ref() != current_sid() || current_sid().is_none() {
            return None;
        }
        Some(handle)
    }
    fn image(&self) -> Option<String> {
        let mut buffer = vec![0u16; 32768];
        let mut size = buffer.len() as u32;
        if unsafe { QueryFullProcessImageNameW(self.0, 0, buffer.as_mut_ptr(), &mut size) } == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }
    fn packaged_claude(&self) -> bool {
        let mut buffer = [0u16; 128];
        let mut size = buffer.len() as u32;
        let status = unsafe {
            windows_sys::Win32::Storage::Packaging::Appx::GetPackageFamilyName(
                self.0,
                &mut size,
                buffer.as_mut_ptr(),
            )
        };
        status == 0
            && String::from_utf16_lossy(&buffer[..size.saturating_sub(1) as usize])
                == "Claude_pzs8sxrjxfjjc"
    }
    fn created(&self) -> Option<u64> {
        let [mut created, mut exited, mut kernel, mut user] = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        if unsafe { GetProcessTimes(self.0, &mut created, &mut exited, &mut kernel, &mut user) }
            == 0
        {
            return None;
        }
        Some((created.dwHighDateTime as u64) << 32 | created.dwLowDateTime as u64)
    }
}
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct BasicInfo {
    status: i32,
    peb: usize,
    affinity: usize,
    priority: i32,
    pid: usize,
    parent: usize,
}
#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Unicode {
    length: u16,
    maximum: u16,
    buffer: usize,
}
#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        handle: HANDLE,
        class: u32,
        info: *mut std::ffi::c_void,
        size: u32,
        written: *mut u32,
    ) -> i32;
}
fn read<T: Default + Copy>(handle: HANDLE, address: usize) -> Option<T> {
    let mut value = T::default();
    let mut written = 0;
    let size = std::mem::size_of::<T>();
    if address == 0
        || unsafe {
            ReadProcessMemory(
                handle,
                address as _,
                (&mut value as *mut T).cast(),
                size,
                &mut written,
            )
        } == 0
        || written != size
    {
        return None;
    }
    Some(value)
}
fn arguments(handle: &Handle) -> Option<Vec<String>> {
    unsafe {
        let mut wow = 0;
        if IsWow64Process(handle.0, &mut wow) == 0 || wow != 0 {
            return None;
        }
        let mut basic = BasicInfo::default();
        let mut written = 0;
        if NtQueryInformationProcess(
            handle.0,
            0,
            (&mut basic as *mut BasicInfo).cast(),
            std::mem::size_of::<BasicInfo>() as u32,
            &mut written,
        ) != 0
        {
            return None;
        }
        let params: usize = read(handle.0, basic.peb.checked_add(0x20)?)?;
        let command: Unicode = read(handle.0, params.checked_add(0x70)?)?;
        if command.buffer == 0
            || command.length == 0
            || command.length > command.maximum
            || command.length % 2 != 0
        {
            return None;
        }
        let mut buffer = vec![0u16; command.length as usize / 2 + 1];
        let mut read_bytes = 0usize;
        if ReadProcessMemory(
            handle.0,
            command.buffer as _,
            buffer.as_mut_ptr().cast(),
            command.length as usize,
            &mut read_bytes,
        ) == 0
            || read_bytes != command.length as usize
        {
            return None;
        }
        // ----- 按 Windows 参数语法解析；完整参数判定，禁止子串命中 -----
        let mut count = 0;
        let args = CommandLineToArgvW(buffer.as_ptr(), &mut count);
        if args.is_null() {
            return None;
        }
        let values = std::slice::from_raw_parts(args, count as usize)
            .iter()
            .map(|p| {
                let mut size = 0;
                while *p.add(size) != 0 {
                    size += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(*p, size))
            })
            .collect();
        LocalFree(args.cast());
        Some(values)
    }
}
fn normalized(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}
fn official_cli(args: &[String]) -> bool {
    let script = if args
        .first()
        .is_some_and(|a| normalized(a).ends_with("\\bun.exe"))
        && args.get(1).is_some_and(|a| a == "run")
    {
        2
    } else {
        1
    };
    args.get(script).is_some_and(|arg| {
        Path::new(arg).is_absolute()
            && normalized(arg).ends_with("\\node_modules\\@anthropic-ai\\claude-code\\cli.js")
    })
}
fn claude_kind(image: &str, args: &[String]) -> Option<&'static str> {
    let path = normalized(image);
    let name = path.rsplit('\\').next()?;
    if ["node.exe", "bun.exe"].contains(&name) && official_cli(args) {
        return Some("Claude Code");
    }
    let home = normalized(&std::env::var("USERPROFILE").ok()?);
    if path == format!("{home}\\.local\\bin\\claude.exe")
        || path == format!("{home}\\.bun\\bin\\claude.exe")
        || (Path::new(&path).parent()
            == Some(Path::new(&format!(
                "{home}\\.local\\share\\claude\\versions"
            )))
            && version_name(name))
    {
        return Some("Claude Code");
    }
    if name != "claude.exe" {
        return None;
    }
    let local = normalized(&std::env::var("LOCALAPPDATA").ok()?);
    if path == format!("{local}\\microsoft\\winget\\packages\\anthropic.claudecode_microsoft.winget.source_8wekyb3d8bbwe\\claude.exe") { return Some("Claude Code"); }
    if path.starts_with(&format!("{local}\\anthropicclaude\\"))
        || path.starts_with(&format!("{local}\\programs\\claude\\"))
    {
        return Some("Claude Desktop");
    }
    None
}
fn version_name(name: &str) -> bool {
    let name = name.strip_suffix(".exe").unwrap_or(name);
    let parts: Vec<_> = name.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}
fn processes(terminate: bool) -> Result<Vec<(u32, Handle)>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("无法检查 Windows 进程。".into());
    }
    let snapshot = Handle(snapshot);
    let mut entry = unsafe { std::mem::zeroed::<PROCESSENTRY32W>() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut result = vec![];
    let mut has = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    while has != 0 {
        let name = String::from_utf16_lossy(&entry.szExeFile)
            .trim_end_matches('\0')
            .to_lowercase();
        if (["claude.exe", "node.exe", "bun.exe", "chrome.exe"].contains(&name.as_str())
            || version_name(&name))
            && entry.th32ProcessID > 1
        {
            if let Some(handle) = Handle::open(entry.th32ProcessID, terminate) {
                result.push((entry.th32ProcessID, handle));
            }
        }
        has = unsafe { Process32NextW(snapshot.0, &mut entry) };
    }
    Ok(result)
}
pub fn claude_processes() -> Result<Vec<ClaudeProcess>, String> {
    let mut found = vec![];
    for (pid, handle) in processes(false)? {
        let Some(image) = handle.image() else { continue; };
        let args = arguments(&handle).unwrap_or_default();
        let kind = if handle.packaged_claude() {
            Some("Claude Desktop")
        } else {
            claude_kind(&image, &args)
        };
        if let (Some(kind), Some(created)) = (kind, handle.created()) {
            found.push(ClaudeProcess {
                pid: pid as i32,
                name: kind.into(),
                executable: image,
                created,
            });
        }
    }
    Ok(found)
}
pub fn kill_process(target: &ClaudeProcess) -> Result<(), String> {
    let Some(handle) = Handle::open(target.pid as u32, true) else { return Ok(()); };
    if handle.created() != Some(target.created)
        || handle.image().as_deref() != Some(target.executable.as_str())
    {
        return Ok(());
    }
    if !handle.packaged_claude()
        && claude_kind(&target.executable, &arguments(&handle).unwrap_or_default()).is_none()
    {
        return Ok(());
    }
    if unsafe { TerminateProcess(handle.0, 1) } == 0 {
        return Err("无法终止已核验的 Claude 进程。".into());
    }
    Ok(())
}
fn profile_matches(args: &[String], profile: &str) -> bool {
    args.iter().enumerate().any(|(index, arg)| {
        let value = arg.strip_prefix("--user-data-dir=").or_else(|| {
            if arg == "--user-data-dir" {
                args.get(index + 1).map(String::as_str)
            } else {
                None
            }
        });
        value.is_some_and(|value| normalized(value) == normalized(profile))
    })
}
unsafe extern "system" fn close_window(window: HWND, pid: LPARAM) -> i32 {
    let mut owner = 0;
    GetWindowThreadProcessId(window, &mut owner);
    if owner == pid as u32 {
        PostMessageW(window, WM_CLOSE, 0, 0);
    }
    1
}
fn browser(profile: &str, close: bool) -> Result<bool, String> {
    if !Path::new(profile).is_absolute() {
        return Err("资料路径无效。".into());
    }
    let mut found = false;
    for (pid, handle) in processes(false)? {
        if handle
            .image()
            .is_some_and(|image| normalized(&image).ends_with("\\chrome.exe"))
            && arguments(&handle).is_some_and(|args| profile_matches(&args, profile))
        {
            found = true;
            if close {
                unsafe {
                    EnumWindows(Some(close_window), pid as LPARAM);
                }
            }
        }
    }
    Ok(found)
}
pub unsafe fn rf_browser_running(profile: *const c_char) -> *mut c_char {
    response(browser(input(profile), false).map(|running| json!({"running":running})))
}
pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    response(browser(input(profile), true).map(|_| json!({"ok":true})))
}
// ----- 本次安装子进程树：超时和宿主退出都不会留下后台安装器 -----
pub(crate) struct ChildJob(Handle);
impl ChildJob {
    pub(crate) fn attach(child: &std::process::Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err("无法隔离 Windows 安装子进程。".into());
        }
        let job = Self(Handle(raw));
        let mut limit = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        limit.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&limit as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limit) as u32,
            )
        } == 0
            || unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } == 0
        {
            return Err("无法隔离 Windows 安装子进程。".into());
        }
        Ok(job)
    }
    pub(crate) fn stop(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0 .0, 1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_arguments_reject_substrings_and_unrelated_scripts() {
        assert!(official_cli(&[
            "node.exe".into(),
            "C:\\test\\node_modules\\@anthropic-ai\\claude-code\\cli.js".into()
        ]));
        assert!(!official_cli(&[
            "node.exe".into(),
            "C:\\other.js".into(),
            "C:\\test\\node_modules\\@anthropic-ai\\claude-code\\cli.js".into()
        ]));
        assert!(!profile_matches(
            &["--user-data-dir=C:\\profile-other".into()],
            "C:\\profile"
        ));
        assert!(profile_matches(
            &["--user-data-dir=C:\\空 格\\profile".into()],
            "C:\\空 格\\profile"
        ));
        let home = std::env::var("USERPROFILE").unwrap();
        assert_eq!(
            claude_kind(&format!(r"{home}\.bun\bin\claude.exe"), &[]),
            Some("Claude Code")
        );
        assert!(current_sid().is_some());
        assert!(Handle::open(std::process::id(), false).is_some());
    }
}
