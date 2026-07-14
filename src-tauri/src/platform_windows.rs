use std::{mem::size_of, path::Path, ptr, thread, time::Duration};
use windows::Win32::System::Ole::{
    OleGetClipboard, OleInitialize, OleSetClipboard, OleUninitialize,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GlobalFree, HANDLE, RECT},
    Globalization::GetUserDefaultUILanguage,
    Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    Storage::FileSystem::GetDiskFreeSpaceExW,
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
            SetClipboardData,
        },
        Diagnostics::Debug::MessageBeep,
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::{
        Input::KeyboardAndMouse::{
            IsWindowEnabled, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        },
        WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowRect, GetWindowThreadProcessId,
            MessageBoxW, SetWindowPos, ShowWindow, GUITHREADINFO, HWND_TOPMOST, IDYES,
            MB_ICONQUESTION, MB_YESNO, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
            SW_HIDE,
        },
    },
};

pub fn free_disk_space(path: &Path) -> Option<u64> {
    use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt};
    let path = OsStr::new(path)
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let mut available = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    (ok != 0).then_some(available)
}

pub fn system_prefers_english() -> bool {
    let language = unsafe { GetUserDefaultUILanguage() };
    language & 0x03ff != 0x0004
}

pub fn show_without_activation(window: isize) -> Result<(), String> {
    let shown = unsafe {
        SetWindowPos(
            window as *mut _,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    };
    if shown == 0 {
        Err(format!(
            "无法显示状态悬浮窗：{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

pub fn hide_window(window: isize) {
    unsafe {
        ShowWindow(window as *mut _, SW_HIDE);
    }
}

pub struct KillOnDropJob(isize);

impl KillOnDropJob {
    pub fn attach(process: HANDLE) -> Result<Self, String> {
        unsafe {
            let job = CreateJobObjectW(ptr::null(), ptr::null());
            if job.is_null() {
                return Err(format!(
                    "无法创建 Worker 作业：{}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
                || AssignProcessToJobObject(job, process) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(format!("无法保护 Worker 生命周期：{error}"));
            }
            Ok(Self(job as isize))
        }
    }
}

impl Drop for KillOnDropJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0 as HANDLE);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputTarget {
    foreground: isize,
    process_id: u32,
}

impl InputTarget {
    pub fn capture() -> Option<Self> {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_null() {
                return None;
            }
            let mut process_id = 0;
            let thread_id = GetWindowThreadProcessId(foreground, &mut process_id);
            if thread_id == 0 {
                return None;
            }
            let mut info = GUITHREADINFO::default();
            info.cbSize = size_of::<GUITHREADINFO>() as u32;
            if GetGUIThreadInfo(thread_id, &mut info) == 0
                || info.hwndFocus.is_null()
                || IsWindowEnabled(info.hwndFocus) == 0
            {
                return None;
            }
            Some(Self {
                foreground: foreground as isize,
                process_id,
            })
        }
    }

    pub fn is_still_active(self) -> bool {
        input_target_compatible(Some(self), foreground_process_id())
    }

    pub fn is_fullscreen(self) -> bool {
        unsafe {
            let monitor = MonitorFromWindow(self.foreground as *mut _, MONITOR_DEFAULTTONEAREST);
            if monitor.is_null() {
                return false;
            }
            let mut monitor_info = MONITORINFO::default();
            monitor_info.cbSize = size_of::<MONITORINFO>() as u32;
            let mut window = RECT::default();
            GetMonitorInfoW(monitor, &mut monitor_info) != 0
                && GetWindowRect(self.foreground as *mut _, &mut window) != 0
                && window.left <= monitor_info.rcMonitor.left
                && window.top <= monitor_info.rcMonitor.top
                && window.right >= monitor_info.rcMonitor.right
                && window.bottom >= monitor_info.rcMonitor.bottom
        }
    }

    pub fn work_area(self) -> Option<(i32, i32, i32, i32)> {
        unsafe {
            let monitor = MonitorFromWindow(self.foreground as *mut _, MONITOR_DEFAULTTONEAREST);
            if monitor.is_null() {
                return None;
            }
            let mut info = MONITORINFO::default();
            info.cbSize = size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return None;
            }
            let area = info.rcWork;
            Some((
                area.left,
                area.top,
                area.right - area.left,
                area.bottom - area.top,
            ))
        }
    }
}

pub fn type_text(text: &str) -> Result<(), String> {
    let units = text.encode_utf16().collect::<Vec<_>>();
    let mut inputs = Vec::with_capacity(units.len() * 2);
    for unit in units {
        let (virtual_key, scan, flags) = if unit == b'\n' as u16 {
            (0x0d, 0, 0)
        } else if unit == b'\r' as u16 {
            continue;
        } else {
            (0, unit, KEYEVENTF_UNICODE)
        };
        inputs.push(key_input(virtual_key, scan, flags));
        inputs.push(key_input(virtual_key, scan, flags | KEYEVENTF_KEYUP));
    }
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err("系统拒绝文字注入，可能是目标应用以管理员权限运行".into());
    }
    Ok(())
}

pub fn paste_text(text: &str, restore_clipboard: bool) -> Result<(), String> {
    let initialized = unsafe { OleInitialize(None) }.is_ok();
    if restore_clipboard && !initialized {
        return Err("CLIPBOARD_RESTORE_FAILED：无法初始化剪贴板事务".into());
    }
    let original = if restore_clipboard {
        Some(unsafe { OleGetClipboard() }.map_err(|error| {
            if initialized {
                unsafe { OleUninitialize() };
            }
            format!("CLIPBOARD_RESTORE_FAILED：无法保存原剪贴板：{error}")
        })?)
    } else {
        None
    };

    let result = (|| {
        copy_text(text)?;
        let written_sequence = unsafe { GetClipboardSequenceNumber() };
        send_paste()?;
        thread::sleep(Duration::from_millis(80));
        if should_restore_clipboard(restore_clipboard, written_sequence, unsafe {
            GetClipboardSequenceNumber()
        }) {
            unsafe { OleSetClipboard(original.as_ref().expect("clipboard snapshot")) }
                .map_err(|error| format!("CLIPBOARD_RESTORE_FAILED：{error}"))?;
        }
        Ok(())
    })();
    drop(original);
    if initialized {
        unsafe { OleUninitialize() };
    }
    result
}

fn should_restore_clipboard(requested: bool, written_sequence: u32, current_sequence: u32) -> bool {
    requested && written_sequence == current_sequence
}

fn foreground_process_id() -> Option<u32> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() {
        return None;
    }
    let mut process_id = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(foreground, &mut process_id) };
    (thread_id != 0 && process_id != 0).then_some(process_id)
}

fn input_target_compatible(captured: Option<InputTarget>, current_process_id: Option<u32>) -> bool {
    matches!((captured, current_process_id), (Some(captured), Some(current))
        if captured.process_id == current)
}

fn send_paste() -> Result<(), String> {
    let inputs = [
        key_input(0x11, 0, 0),
        key_input(0x56, 0, 0),
        key_input(0x56, 0, KEYEVENTF_KEYUP),
        key_input(0x11, 0, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err("INJECTION_FAILED：系统拒绝粘贴操作".into())
    }
}

pub fn play_sound(kind: &str) {
    let sound = match kind {
        "error" => 0x0000_0010,
        "stop" => 0x0000_0040,
        _ => 0x0000_0000,
    };
    unsafe {
        MessageBeep(sound);
    }
}

pub fn confirm_exit(english: bool) -> bool {
    let message = if english {
        "Recording or recognition is active. Exit Rain and cancel the current task?\0"
    } else {
        "正在录音或识别。确定退出 Rain 并取消当前任务吗？\0"
    }
    .encode_utf16()
    .collect::<Vec<_>>();
    let title = "Rain氛围输入法\0".encode_utf16().collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONQUESTION,
        ) == IDYES
    }
}

pub fn copy_text(text: &str) -> Result<(), String> {
    let wide = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let bytes = wide.len() * size_of::<u16>();
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
    if memory.is_null() {
        return Err("无法分配剪贴板内存".into());
    }
    unsafe {
        let destination = GlobalLock(memory) as *mut u16;
        if destination.is_null() {
            GlobalFree(memory);
            return Err("无法写入剪贴板内存".into());
        }
        ptr::copy_nonoverlapping(wide.as_ptr(), destination, wide.len());
        GlobalUnlock(memory);
    }

    for attempt in 0..6 {
        if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
            let result = unsafe {
                let emptied = EmptyClipboard();
                let result = if emptied != 0 {
                    SetClipboardData(13, memory) // CF_UNICODETEXT
                } else {
                    ptr::null_mut()
                };
                CloseClipboard();
                result
            };
            if !result.is_null() {
                return Ok(());
            }
            unsafe { GlobalFree(memory) };
            return Err("无法更新剪贴板".into());
        }
        thread::sleep(Duration::from_millis(10 * (attempt + 1)));
    }
    unsafe { GlobalFree(memory) };
    Err("剪贴板正被其他应用占用".into())
}

fn key_input(virtual_key: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_restore_never_overwrites_a_concurrent_user_copy() {
        assert!(should_restore_clipboard(true, 41, 41));
        assert!(!should_restore_clipboard(true, 41, 42));
        assert!(!should_restore_clipboard(false, 41, 41));
    }

    #[test]
    fn input_target_requires_the_same_process() {
        let first = InputTarget {
            foreground: 1,
            process_id: 3,
        };
        assert!(input_target_compatible(Some(first), Some(3)));
        assert!(!input_target_compatible(Some(first), Some(5)));
        assert!(!input_target_compatible(Some(first), None));
    }
}
