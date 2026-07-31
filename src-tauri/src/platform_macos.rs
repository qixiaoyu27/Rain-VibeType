use std::{ffi::c_void, path::Path};

use tauri::WebviewWindow;

unsafe extern "C" {
    fn rain_accessibility_trusted(prompt: bool) -> bool;
    fn rain_frontmost_pid() -> i32;
    fn rain_target_is_fullscreen(process_id: i32) -> bool;
    fn rain_target_work_area(
        process_id: i32,
        left: *mut i32,
        top: *mut i32,
        width: *mut i32,
        height: *mut i32,
    ) -> bool;
    fn rain_copy_text(bytes: *const u8, length: usize) -> i32;
    fn rain_paste_text(
        bytes: *const u8,
        length: usize,
        process_id: i32,
        restore_clipboard: bool,
    ) -> i32;
    fn rain_type_text(bytes: *const u8, length: usize, process_id: i32) -> i32;
    fn rain_show_window(window: *mut c_void, interactive: bool) -> bool;
    fn rain_hide_window(window: *mut c_void);
    fn rain_play_sound(kind: *const u8, length: usize);
    fn rain_confirm_exit(english: bool) -> bool;
    fn rain_system_prefers_english() -> bool;
    fn rain_free_disk_space(bytes: *const u8, length: usize, output: *mut u64) -> bool;
    fn rain_duck_system_audio() -> *mut c_void;
    fn rain_restore_system_audio(token: *mut c_void);
    fn rain_kill_process_group(process_group: i32);
}

pub fn ensure_accessibility_permission() -> Result<(), String> {
    if unsafe { rain_accessibility_trusted(false) } || unsafe { rain_accessibility_trusted(true) } {
        Ok(())
    } else {
        Err("请在“系统设置 → 隐私与安全性 → 辅助功能”中允许雨音输入法，然后重新按快捷键。识别文字不会被发送到其他应用。".into())
    }
}

pub fn free_disk_space(path: &Path) -> Option<u64> {
    let path = path.as_os_str().as_encoded_bytes();
    let mut output = 0;
    unsafe { rain_free_disk_space(path.as_ptr(), path.len(), &mut output) }.then_some(output)
}

pub fn system_prefers_english() -> bool {
    unsafe { rain_system_prefers_english() }
}

pub fn show_without_activation(window: &WebviewWindow, interactive: bool) -> Result<(), String> {
    let window = window
        .ns_window()
        .map_err(|error| format!("无法读取状态悬浮窗句柄：{error}"))?;
    if unsafe { rain_show_window(window, interactive) } {
        Ok(())
    } else {
        Err("无法显示状态悬浮窗".into())
    }
}

pub fn hide_window(window: &WebviewWindow) {
    if let Ok(window) = window.ns_window() {
        unsafe { rain_hide_window(window) };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputTarget {
    process_id: i32,
}

impl InputTarget {
    pub fn capture() -> Option<Self> {
        let process_id = unsafe { rain_frontmost_pid() };
        (process_id > 0).then_some(Self { process_id })
    }

    pub fn is_still_active(self) -> bool {
        input_target_compatible(self.process_id, unsafe { rain_frontmost_pid() })
    }

    pub fn is_fullscreen(self) -> bool {
        unsafe { rain_target_is_fullscreen(self.process_id) }
    }

    pub fn work_area(self) -> Option<(i32, i32, i32, i32)> {
        let (mut left, mut top, mut width, mut height) = (0, 0, 0, 0);
        unsafe {
            rain_target_work_area(
                self.process_id,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
            )
        }
        .then_some((left, top, width, height))
    }
}

fn input_target_compatible(captured_process_id: i32, current_process_id: i32) -> bool {
    captured_process_id > 0 && captured_process_id == current_process_id
}

pub struct SystemAudioDucker {
    token: usize,
}

impl SystemAudioDucker {
    pub fn activate() -> Result<Self, String> {
        let token = unsafe { rain_duck_system_audio() } as usize;
        if token == 0 {
            Err("当前输出设备不允许安全调整系统播放音量；已取消本次录音，原音量未改变".into())
        } else {
            Ok(Self { token })
        }
    }
}

impl Drop for SystemAudioDucker {
    fn drop(&mut self) {
        unsafe { rain_restore_system_audio(self.token as *mut c_void) };
    }
}

pub struct KillOnDropJob(i32);

impl KillOnDropJob {
    pub fn attach(process_id: u32) -> Result<Self, String> {
        let process_group =
            i32::try_from(process_id).map_err(|_| "无法保护 Worker 生命周期：进程 ID 超出范围")?;
        Ok(Self(process_group))
    }
}

impl Drop for KillOnDropJob {
    fn drop(&mut self) {
        unsafe { rain_kill_process_group(self.0) };
    }
}

pub fn copy_text(text: &str) -> Result<(), String> {
    match unsafe { rain_copy_text(text.as_ptr(), text.len()) } {
        0 => Ok(()),
        _ => Err("无法更新剪贴板".into()),
    }
}

pub fn paste_text(target: InputTarget, text: &str, restore_clipboard: bool) -> Result<(), String> {
    match unsafe {
        rain_paste_text(
            text.as_ptr(),
            text.len(),
            target.process_id,
            restore_clipboard,
        )
    } {
        0 => Ok(()),
        1 => Err("INPUT_TARGET_CHANGED：原输入应用已切换，未执行粘贴".into()),
        2 => Err("INJECTION_FAILED：macOS 辅助功能权限不可用".into()),
        3 => Err("CLIPBOARD_RESTORE_FAILED：无法完整保存原剪贴板，未执行粘贴".into()),
        4 => Err("INJECTION_FAILED：无法把识别文字写入剪贴板".into()),
        5 => Err("INJECTION_FAILED：macOS 拒绝发送 Command+V".into()),
        6 => Err("CLIPBOARD_RESTORE_FAILED：原剪贴板未能完整恢复".into()),
        _ => Err("INJECTION_FAILED：文字粘贴失败".into()),
    }
}

pub fn type_text(target: InputTarget, text: &str) -> Result<(), String> {
    match unsafe { rain_type_text(text.as_ptr(), text.len(), target.process_id) } {
        0 => Ok(()),
        1 => Err("INPUT_TARGET_CHANGED：原输入应用已切换，未执行文字注入".into()),
        2 => Err("INJECTION_FAILED：macOS 辅助功能权限不可用".into()),
        _ => Err("INJECTION_FAILED：macOS 拒绝模拟文字输入".into()),
    }
}

pub fn play_sound(kind: &str) {
    unsafe { rain_play_sound(kind.as_ptr(), kind.len()) };
}

pub fn confirm_exit(english: bool) -> bool {
    unsafe { rain_confirm_exit(english) }
}

pub fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开目录：{error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn input_target_requires_the_same_frontmost_process() {
        assert!(super::input_target_compatible(3, 3));
        assert!(!super::input_target_compatible(3, 5));
        assert!(!super::input_target_compatible(0, 0));
    }
}
