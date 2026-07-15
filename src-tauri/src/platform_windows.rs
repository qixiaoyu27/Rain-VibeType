use std::{ffi::c_void, mem::size_of, path::Path, ptr, thread, time::Duration};
use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    Media::Audio::{
        eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    },
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GlobalFree, HANDLE, RECT},
    Globalization::GetUserDefaultUILanguage,
    Graphics::Gdi::{
        CopyEnhMetaFileW, DeleteEnhMetaFile, GetMonitorInfoW, MonitorFromWindow, MONITORINFO,
        MONITOR_DEFAULTTONEAREST,
    },
    Storage::FileSystem::GetDiskFreeSpaceExW,
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
            GetClipboardSequenceNumber, OpenClipboard, SetClipboardData,
        },
        Diagnostics::Debug::MessageBeep,
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        },
        WindowsAndMessaging::{
            GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, MessageBoxW,
            SetWindowPos, ShowWindow, HWND_TOPMOST, IDYES, MB_ICONQUESTION, MB_YESNO,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE,
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

pub struct SystemAudioDucker {
    original_volume: f32,
}

impl SystemAudioDucker {
    pub fn activate() -> Result<Self, String> {
        let original_volume = with_default_output_volume(|endpoint| unsafe {
            endpoint.GetMasterVolumeLevelScalar()
        })?;
        with_default_output_volume(|endpoint| unsafe {
            endpoint.SetMasterVolumeLevelScalar(ducked_volume(original_volume), ptr::null())
        })?;
        Ok(Self { original_volume })
    }
}

impl Drop for SystemAudioDucker {
    fn drop(&mut self) {
        // ponytail: restores the current default endpoint; retain its device ID
        // if output-device hot-swapping during one recording becomes important.
        let _ = with_default_output_volume(|endpoint| unsafe {
            endpoint.SetMasterVolumeLevelScalar(self.original_volume, ptr::null())
        });
    }
}

fn ducked_volume(original: f32) -> f32 {
    (original * 0.2).clamp(0.0, 1.0)
}

fn with_default_output_volume<T>(
    operation: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
) -> Result<T, String> {
    unsafe {
        let com_status = CoInitializeEx(None, COINIT_MULTITHREADED);
        let uninitialize = com_status.is_ok();
        if com_status.is_err() && com_status != RPC_E_CHANGED_MODE {
            return Err(format!("无法初始化 Windows 音频控制：{com_status}"));
        }
        let result = (|| {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            operation(&endpoint)
        })()
        .map_err(|error| format!("无法控制系统播放音量：{error}"));
        if uninitialize {
            CoUninitialize();
        }
        result
    }
}

struct ClipboardSnapshot {
    entries: Vec<ClipboardEntry>,
}

struct ClipboardEntry {
    format: u32,
    handle: *mut c_void,
    enhanced_metafile: bool,
}

impl Drop for ClipboardEntry {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            if self.enhanced_metafile {
                DeleteEnhMetaFile(self.handle);
            } else {
                GlobalFree(self.handle);
            }
        }
    }
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
            if thread_id == 0 || process_id == 0 {
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
    let original = restore_clipboard.then(capture_clipboard).transpose()?;
    copy_text(text)?;
    let written_sequence = unsafe { GetClipboardSequenceNumber() };
    send_paste()?;
    thread::sleep(Duration::from_millis(300));
    if should_restore_clipboard(restore_clipboard, written_sequence, unsafe {
        GetClipboardSequenceNumber()
    }) {
        restore_clipboard_snapshot(original.expect("clipboard snapshot"))?;
    }
    Ok(())
}

fn should_restore_clipboard(requested: bool, written_sequence: u32, current_sequence: u32) -> bool {
    requested && written_sequence == current_sequence
}

fn capture_clipboard() -> Result<ClipboardSnapshot, String> {
    open_clipboard_with_retry().map_err(|_| {
        "CLIPBOARD_RESTORE_FAILED：无法保存原剪贴板，剪贴板正被其他应用占用".to_string()
    })?;
    let result = (|| {
        let mut entries = Vec::new();
        let mut format = 0;
        loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            // GDI objects cannot be copied as movable global memory. CF_DIB
            // preserves bitmap content, while owner-rendered formats are tied
            // to the process that owns the clipboard.
            if matches!(format, 2 | 3 | 9) || (0x80..=0xff).contains(&format) {
                continue;
            }
            let source = unsafe { GetClipboardData(format) };
            if source.is_null() {
                continue;
            }
            if format == 14 {
                let handle = unsafe { CopyEnhMetaFileW(source, ptr::null()) };
                if handle.is_null() {
                    return Err("CLIPBOARD_RESTORE_FAILED：无法复制增强型图元文件".into());
                }
                entries.push(ClipboardEntry {
                    format,
                    handle,
                    enhanced_metafile: true,
                });
                continue;
            }
            let bytes = unsafe { GlobalSize(source) };
            if bytes == 0 {
                continue;
            }
            let copy = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
            if copy.is_null() {
                return Err("CLIPBOARD_RESTORE_FAILED：无法分配剪贴板快照内存".into());
            }
            let source_bytes = unsafe { GlobalLock(source) };
            let destination = unsafe { GlobalLock(copy) };
            if source_bytes.is_null() || destination.is_null() {
                if !source_bytes.is_null() {
                    unsafe { GlobalUnlock(source) };
                }
                if !destination.is_null() {
                    unsafe { GlobalUnlock(copy) };
                }
                unsafe { GlobalFree(copy) };
                return Err("CLIPBOARD_RESTORE_FAILED：无法读取原剪贴板数据".into());
            }
            unsafe {
                ptr::copy_nonoverlapping(source_bytes as *const u8, destination as *mut u8, bytes);
                GlobalUnlock(source);
                GlobalUnlock(copy);
            }
            entries.push(ClipboardEntry {
                format,
                handle: copy,
                enhanced_metafile: false,
            });
        }
        Ok(ClipboardSnapshot { entries })
    })();
    unsafe { CloseClipboard() };
    result
}

fn restore_clipboard_snapshot(mut snapshot: ClipboardSnapshot) -> Result<(), String> {
    open_clipboard_with_retry().map_err(|_| {
        "CLIPBOARD_RESTORE_FAILED：原剪贴板暂时无法恢复，剪贴板正被其他应用占用".to_string()
    })?;
    let result = (|| {
        if unsafe { EmptyClipboard() } == 0 {
            return Err("CLIPBOARD_RESTORE_FAILED：无法清空临时识别文本".into());
        }
        let mut failed = false;
        for entry in &mut snapshot.entries {
            if unsafe { SetClipboardData(entry.format, entry.handle) }.is_null() {
                failed = true;
            } else {
                // SetClipboardData transfers ownership to Windows.
                entry.handle = ptr::null_mut();
            }
        }
        if failed {
            Err("CLIPBOARD_RESTORE_FAILED：部分原剪贴板格式无法恢复".into())
        } else {
            Ok(())
        }
    })();
    unsafe { CloseClipboard() };
    result
}

fn open_clipboard_with_retry() -> Result<(), ()> {
    for attempt in 0..6 {
        if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10 * (attempt + 1)));
    }
    Err(())
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
    let mut inputs = Vec::with_capacity(8);
    // The push-to-talk modifiers may still be physically held when recognition
    // finishes. Release them so Ctrl+V cannot turn into Ctrl+Shift+V/Alt+V.
    for virtual_key in [0x10, 0x12, 0x5b, 0x5c] {
        if unsafe { GetAsyncKeyState(virtual_key) } < 0 {
            inputs.push(key_input(virtual_key as u16, 0, KEYEVENTF_KEYUP));
        }
    }
    inputs.extend([
        key_input(0x11, 0, 0),
        key_input(0x56, 0, 0),
        key_input(0x56, 0, KEYEVENTF_KEYUP),
        key_input(0x11, 0, KEYEVENTF_KEYUP),
    ]);
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

    if open_clipboard_with_retry().is_err() {
        unsafe { GlobalFree(memory) };
        return Err("剪贴板正被其他应用占用".into());
    }
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
        Ok(())
    } else {
        unsafe { GlobalFree(memory) };
        Err("无法更新剪贴板".into())
    }
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

    #[test]
    fn system_audio_is_reduced_to_twenty_percent() {
        assert!((ducked_volume(0.75) - 0.15).abs() < f32::EPSILON);
    }
}
