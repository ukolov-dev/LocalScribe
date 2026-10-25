#[cfg(not(target_os = "macos"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
#[cfg(not(target_os = "macos"))]
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri::{LogicalPosition, LogicalSize, Manager};

pub fn show_widget(app: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.show()?;
    if let Some(window) = app.get_webview_window("main") {
        window.unminimize()?;
        window.set_visible_on_all_workspaces(true)?;
        anchor_widget(app, &window)?;
        window.show()?;
        window.set_focus()?;
        #[cfg(target_os = "macos")]
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            let native = window.ns_window()? as *mut objc2_app_kit::NSWindow;
            // Accessory apps need explicit activation when opened from the status bar.
            #[allow(deprecated)]
            unsafe {
                objc2_app_kit::NSApplication::sharedApplication(mtm)
                    .activateIgnoringOtherApps(true);
                (*native).makeKeyAndOrderFront(None);
                (*native).orderFrontRegardless();
            }
        }
    }
    Ok(())
}

fn anchor_widget(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(not(target_os = "macos"))]
    let Some(tray) = app.tray_by_id("localscribe") else {
        return Ok(());
    };
    let current_scale = window.scale_factor()?;
    #[cfg(target_os = "macos")]
    let geometry = {
        // Read AppKit points directly: tray-icon 0.21 mixes Retina pixels with points on macOS.
        let frame = native_click::frame();
        match (frame, app.primary_monitor()?) {
            (Some((x, y, width, height)), Some(primary)) => {
                let primary_height = primary
                    .size()
                    .to_logical::<f64>(primary.scale_factor())
                    .height;
                Some((
                    LogicalPosition::new(x, primary_height - y - height),
                    LogicalSize::new(width, height),
                ))
            }
            _ => None,
        }
    };
    #[cfg(not(target_os = "macos"))]
    let geometry = tray.rect()?.map(|rect| {
        (
            rect.position.to_logical(current_scale),
            rect.size.to_logical(current_scale),
        )
    });
    let Some((icon_position, icon_size)) = geometry else {
        return Ok(());
    };
    let Some(monitor) = app.monitor_from_point(icon_position.x, icon_position.y)? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    window.set_position(widget_position(
        icon_position,
        icon_size,
        window.outer_size()?.to_logical(current_scale),
        area.position.to_logical(scale),
        area.size.to_logical(scale),
    ))?;
    Ok(())
}

fn widget_position(
    icon: LogicalPosition<f64>,
    icon_size: LogicalSize<f64>,
    window_size: LogicalSize<f64>,
    work_origin: LogicalPosition<f64>,
    work_size: LogicalSize<f64>,
) -> LogicalPosition<f64> {
    let x = icon.x + (icon_size.width - window_size.width) / 2.0;
    let y = icon.y + icon_size.height + 6.0;
    LogicalPosition::new(
        x.clamp(
            work_origin.x,
            (work_origin.x + work_size.width - window_size.width).max(work_origin.x),
        ),
        y.clamp(
            work_origin.y,
            (work_origin.y + work_size.height - window_size.height).max(work_origin.y),
        ),
    )
}

fn toggle_widget(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible()? && !window.is_minimized()? {
            window.hide()?;
        } else {
            show_widget(app)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let app = app.handle().clone();
    // Create the status item after AppKit finishes its launch callback.
    std::thread::spawn(move || {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Err(error) = native_click::setup(&handle) {
                eprintln!("Cannot create LocalScribe tray: {error}");
            }
        });
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(
        app,
        "toggle-widget",
        "Show / Hide LocalScribe",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit-localscribe",
        "Quit LocalScribe",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
    let mut tray = TrayIconBuilder::with_id("localscribe")
        .tooltip("LocalScribe")
        .menu(&menu)
        .title("LocalScribe")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle-widget" => {
                if let Err(error) = toggle_widget(app) {
                    eprintln!("Cannot toggle LocalScribe: {error}");
                }
            }
            "quit-localscribe" => app.exit(0),
            _ => {}
        });
    {
        tray = tray.on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = toggle_widget(tray.app_handle()) {
                    eprintln!("Cannot toggle LocalScribe: {error}");
                }
            }
        });
    }
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_stays_below_the_icon_and_inside_the_screen() {
        let position = |icon_x, origin_x, width| {
            widget_position(
                LogicalPosition::new(icon_x, 0.0),
                LogicalSize::new(24.0, 24.0),
                LogicalSize::new(440.0, 600.0),
                LogicalPosition::new(origin_x, 24.0),
                LogicalSize::new(width, 800.0),
            )
        };
        assert_eq!(
            position(700.0, 0.0, 1440.0),
            LogicalPosition::new(492.0, 30.0)
        );
        assert_eq!(
            position(1420.0, 0.0, 1440.0),
            LogicalPosition::new(1000.0, 30.0)
        );
        assert_eq!(
            position(-1420.0, -1440.0, 1440.0),
            LogicalPosition::new(-1440.0, 30.0)
        );
        assert_eq!(position(10.0, 0.0, 360.0), LogicalPosition::new(0.0, 30.0));
    }

    #[test]
    fn physical_retina_geometry_converts_to_the_same_anchor() {
        let scale = 2.0;
        let result = widget_position(
            tauri::PhysicalPosition::new(1400, 0).to_logical(scale),
            tauri::PhysicalSize::new(48, 48).to_logical(scale),
            tauri::PhysicalSize::new(880, 1200).to_logical(scale),
            tauri::PhysicalPosition::new(0, 48).to_logical(scale),
            tauri::PhysicalSize::new(2880, 1600).to_logical(scale),
        );
        assert_eq!(result, LogicalPosition::new(492.0, 30.0));
    }
}

#[cfg(target_os = "macos")]
mod native_click {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSEvent, NSEventMask, NSEventType, NSImage, NSMenu, NSMenuItem, NSScreen,
        NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
    };
    use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
    use std::cell::RefCell;

    struct ClickState {
        app: tauri::AppHandle,
    }
    struct StatusItem {
        item: Retained<NSStatusItem>,
        menu: Retained<NSMenu>,
        anchor: Option<(f64, f64, f64, f64)>,
        _target: Retained<ClickTarget>,
    }
    thread_local! {
        static STATUS: RefCell<Option<StatusItem>> = const { RefCell::new(None) };
    }
    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = ClickState]
        struct ClickTarget;
        unsafe impl NSObjectProtocol for ClickTarget {}
        impl ClickTarget {
            #[unsafe(method(trayClicked:))]
            fn click(&self, _sender: Option<&AnyObject>) {
                let mtm = MainThreadMarker::from(self);
                let point = NSEvent::mouseLocation();
                let screen = NSScreen::screens(mtm).iter().find_map(|screen| {
                    let f = screen.frame();
                    (point.x >= f.origin.x && point.x < f.origin.x + f.size.width
                        && point.y >= f.origin.y && point.y <= f.origin.y + f.size.height).then_some(f)
                });
                if let Some(screen) = screen {
                    STATUS.with(|slot| {
                        if let Some(status) = slot.borrow_mut().as_mut() {
                            status.anchor = Some((point.x - 12.0, screen.origin.y + screen.size.height - 24.0, 24.0, 24.0));
                        }
                    });
                }
                let right_click = NSApplication::sharedApplication(mtm).currentEvent()
                    .is_some_and(|event| event.r#type() == NSEventType::RightMouseUp);
                if right_click {
                    STATUS.with(|slot| {
                        if let Some(status) = slot.borrow().as_ref() {
                            #[allow(deprecated)]
                            status.item.popUpStatusItemMenu(&status.menu);
                        }
                    });
                } else { self.toggle(sel!(togglePanel:), None); }
            }
            #[unsafe(method(togglePanel:))]
            fn toggle(&self, _sender: Option<&AnyObject>) {
                if let Err(error) = super::toggle_widget(&self.ivars().app) {
                    eprintln!("Cannot toggle LocalScribe: {error}");
                }
            }
            #[unsafe(method(quitApp:))]
            fn quit(&self, _sender: Option<&AnyObject>) { self.ivars().app.exit(0); }
        }
    );
    pub fn frame() -> Option<(f64, f64, f64, f64)> {
        STATUS.with(|slot| slot.borrow().as_ref()?.anchor)
    }

    pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
        let mtm = MainThreadMarker::new().expect("tray setup runs on main thread");
        let target = ClickTarget::alloc(mtm).set_ivars(ClickState { app: app.clone() });
        let target: Retained<ClickTarget> = unsafe { msg_send![super(target), init] };
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let button = item.button(mtm).expect("status button");
        button.setTitle(&NSString::from_str("LocalScribe"));
        button.setToolTip(Some(&NSString::from_str("LocalScribe")));
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("waveform"),
            Some(&NSString::from_str("LocalScribe")),
        );
        button.setImage(image.as_deref());
        unsafe {
            button.setTarget(Some(&target));
            button.setAction(Some(sel!(trayClicked:)));
        }
        button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        let menu = NSMenu::new(mtm);
        for (title, action) in [
            ("Показать / скрыть LocalScribe", sel!(togglePanel:)),
            ("Выйти", sel!(quitApp:)),
        ] {
            let entry = NSMenuItem::new(mtm);
            entry.setTitle(&NSString::from_str(title));
            unsafe {
                entry.setTarget(Some(&target));
                entry.setAction(Some(action));
            }
            menu.addItem(&entry);
        }
        STATUS.with(|slot| {
            *slot.borrow_mut() = Some(StatusItem {
                item,
                menu,
                anchor: None,
                _target: target,
            })
        });
        Ok(())
    }
}
