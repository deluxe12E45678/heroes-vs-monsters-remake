//! Windows-only: after the primary window exists, show it with SW_SHOWNOACTIVATE /
//! SetWindowPos(SWP_NOACTIVATE) so creating the HWND does not steal focus or minimize
//! an exclusive-fullscreen game (e.g. League of Legends). Own design.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapper};
use raw_window_handle::RawWindowHandle;

#[derive(Resource, Default)]
struct SoftShown(bool);

pub fn plugin(app: &mut App) {
    app.init_resource::<SoftShown>()
        .add_systems(Update, soft_show_noactivate);
}

fn soft_show_noactivate(
    mut done: ResMut<SoftShown>,
    q: Query<&RawHandleWrapper, With<PrimaryWindow>>,
) {
    if done.0 { return; }
    let Ok(raw) = q.single() else { return };
    let RawWindowHandle::Win32(h) = raw.get_window_handle() else { return };
    let hwnd = h.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    // SAFETY: HWND from Bevy/winit's live primary window. Only ShowWindow / SetWindowPos
    // with NOACTIVATE — never SetForegroundWindow / BringWindowToTop.
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SW_SHOWNOACTIVATE,
        };
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetWindowPos(hwnd, 0 as _, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
    done.0 = true;
}
