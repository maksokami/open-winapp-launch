#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
struct Placement {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[cfg(any(windows, test))]
fn centered(work_area: [i32; 4], dpi: u32, logical_width: f32, logical_height: f32) -> Placement {
    let [left, top, right, bottom] = work_area;
    let scale = dpi as f32 / 96.0;
    let width = (logical_width * scale).round() as i32;
    let height = (logical_height * scale).round() as i32;
    Placement {
        x: left + (right - left - width) / 2,
        y: top + (bottom - top - height) / 2,
        width,
        height,
    }
}

#[cfg(windows)]
pub fn update(window: &slint::Window, width: f32, height: f32) -> anyhow::Result<()> {
    use anyhow::{Context, bail};
    use raw_window_handle::HasWindowHandle;
    use windows::Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint},
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::{GetWindowRect, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos},
        },
    };

    let native = window.window_handle();
    let handle = native
        .window_handle()
        .context("could not obtain window handle")?;
    let raw_window_handle::RawWindowHandle::Win32(handle) = handle.as_raw() else {
        bail!("launcher is not using a Win32 window");
    };
    let hwnd = HWND(handle.hwnd.get() as *mut _);
    unsafe {
        // The desktop origin always belongs to the primary display.
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err(std::io::Error::last_os_error())
                .context("could not read primary display work area");
        }
        let mut dpi_x = 0;
        let mut dpi_y = 0;
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y)
            .context("could not read primary display DPI")?;
        if dpi_x == 0 {
            bail!("primary display reported zero DPI");
        }
        let work = info.rcWork;
        let target = centered(
            [work.left, work.top, work.right, work.bottom],
            dpi_x,
            width,
            height,
        );
        let mut current = RECT::default();
        GetWindowRect(hwnd, &mut current).context("could not read launcher bounds")?;
        if current.left != target.x
            || current.top != target.y
            || current.right - current.left != target.width
            || current.bottom - current.top != target.height
        {
            SetWindowPos(
                hwnd,
                None,
                target.x,
                target.y,
                target.width,
                target.height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .context("could not resize and center launcher")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_in_work_area_excluding_taskbar() {
        assert_eq!(
            centered([0, 0, 1920, 1040], 96, 704.0, 457.0),
            Placement {
                x: 608,
                y: 291,
                width: 704,
                height: 457
            }
        );
    }

    #[test]
    fn docking_and_undocking_do_not_accumulate_scaling() {
        let laptop = centered([0, 0, 1920, 1040], 120, 704.0, 457.0);
        let tv = centered([0, 0, 3840, 2080], 192, 704.0, 457.0);
        assert_eq!((tv.width, tv.height), (1408, 914));
        assert_eq!(centered([0, 0, 1920, 1040], 120, 704.0, 457.0), laptop);
        assert_eq!((laptop.width, laptop.height), (880, 571));
    }

    #[test]
    fn accounts_for_work_area_origin_and_error_banner() {
        assert_eq!(
            centered([80, 40, 1920, 1080], 144, 704.0, 495.0),
            Placement {
                x: 472,
                y: 188,
                width: 1056,
                height: 743
            }
        );
    }
}
