#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg(target_os = "windows")]
mod wallpaper {
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::s;

    unsafe extern "system" fn find_worker_w(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let shell = FindWindowExA(hwnd, HWND::default(), s!("SHELLDLL_DefView"), None);
        if shell != HWND::default() {
            let worker = FindWindowExA(HWND::default(), hwnd, s!("WorkerW"), None);
            *(lparam.0 as *mut HWND) = worker;
        }
        TRUE
    }

    pub fn attach(hwnd: HWND) {
        unsafe {
            let progman = FindWindowA(s!("Progman"), None);
            SendMessageA(progman, 0x052C, WPARAM(0), LPARAM(0));
            let mut worker_w = HWND::default();
            let _ = EnumWindows(
                Some(find_worker_w),
                LPARAM(&mut worker_w as *mut HWND as isize),
            );
            if worker_w != HWND::default() {
                SetParent(hwnd, worker_w);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod wallpaper {
    use std::process::Command;

    pub fn attach(title: &str) {
        std::thread::sleep(std::time::Duration::from_millis(800));

        Command::new("wmctrl").args(["-r", title, "-t", "-1"]).spawn().ok();
        Command::new("wmctrl").args(["-r", title, "-b", "add,below"]).spawn().ok();
        Command::new("wmctrl").args(["-r", title, "-b", "add,skip_taskbar,skip_pager"]).spawn().ok();

        if let Ok(out) = Command::new("xdotool").args(["search", "--name", title]).output() {
            let wid = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(wid) = wid.lines().next() {
                Command::new("xprop")
                    .args(["-id", wid.trim(), "-f", "_NET_WM_WINDOW_TYPE", "32a",
                           "-set", "_NET_WM_WINDOW_TYPE", "_NET_WM_WINDOW_TYPE_DESKTOP"])
                    .spawn().ok();
            }
        }
    }
}

#[tauri::command]
fn run_wallpaper(app: tauri::AppHandle, intensity: String) {
    let home = app.get_webview_window("home").unwrap();
    let wall = app.get_webview_window("wallpaper").unwrap();

    let url = format!("wallpaper.html?intensity={}", intensity);
    let _ = wall.eval(&format!("window.location.href='{}'", url));

    wall.show().unwrap();
    home.hide().unwrap();

    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows::Win32::Foundation::HWND;
        if let Ok(handle) = wall.window_handle() {
            if let RawWindowHandle::Win32(h) = handle.as_ref() {
                let hwnd = HWND(isize::from(h.hwnd) as *mut _);
                wallpaper::attach(hwnd);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let title = wall.title().unwrap_or("Crystal Snow Wallpaper".into());
        std::thread::spawn(move || { wallpaper::attach(&title); });
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_wallpaper])
        .setup(|app| {
            let settings = MenuItem::with_id(app, "settings", "Settings",    true, None::<&str>)?;
            let none     = MenuItem::with_id(app, "none",     "✕  None",     true, None::<&str>)?;
            let quit     = MenuItem::with_id(app, "quit",     "Quit",        true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&settings, &none, &quit])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Crystal Snow")
                .on_menu_event(move |app, event| {
                    let home = app.get_webview_window("home").unwrap();
                    let wall = app.get_webview_window("wallpaper").unwrap();
                    match event.id().as_ref() {
                        "settings" => { home.show().unwrap(); home.set_focus().unwrap(); }
                        "none"     => { wall.hide().unwrap(); }
                        "quit"     => { app.exit(0); }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running app");
}