use eframe::egui;
use egui::{Context, Vec2, ViewportBuilder};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use log::{debug, info};
use std::{
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
use tracing_subscriber::{self, fmt, EnvFilter};
use windows::{
    core::{s, Result},
    Win32::{
        Foundation::{HWND, RECT},
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

pub fn setup_logger(level: &str) -> Result<()> {
    let formatter = fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(true)
        .with_thread_names(false);
    let filter = EnvFilter::builder()
        .from_env()
        .unwrap()
        .add_directive(format!("railers={}", level.to_lowercase()).parse().unwrap());
    tracing_subscriber::fmt()
        .event_format(formatter)
        .with_env_filter(filter)
        .init();
    Ok(())
}

static USER_HIDDEN: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();

fn main() -> eframe::Result<()> {
    setup_logger("debug").unwrap();
    USER_HIDDEN.set(Arc::new(Mutex::new(false))).unwrap();
    let manager = GlobalHotKeyManager::new().unwrap();
    // construct the hotkey
    let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyN);

    // register it
    manager.register(hotkey).unwrap();
    debug!("Hotkey registered");
    thread::spawn(move || loop {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Released {
                continue;
            }
            unsafe {
                let hsr_hwnd = FindWindowA(s!("UnityWndClass"), s!("Honkai: Star Rail"));
                let fg_hwnd = GetForegroundWindow();
                let hwnd = OUR_HWND.get().unwrap().clone();
                debug!("FG HWND: {:?}", fg_hwnd);
                debug!("HSR HWND: {:?}", hsr_hwnd);
                debug!("OUR HWND: {:?}", hwnd);
                if hsr_hwnd.0 == 0 || (fg_hwnd != hsr_hwnd && fg_hwnd != hwnd) {
                    continue;
                }
                debug!("Hotkey pressed: {:?}", event);
                let mut a = USER_HIDDEN.get().unwrap().lock().unwrap();
                *a = !*a;
            }
        }
        thread::sleep(Duration::from_millis(10));
    });
    info!("Initializing Railers GUI...");
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = ViewportBuilder::default()
        .with_title("Railers")
        .with_inner_size(Vec2::new(1280.0, 720.0))
        .with_always_on_top()
        .with_transparent(true)
        .with_decorations(false);
    eframe::run_native(
        "Railers",
        native_options,
        Box::new(|cc| Box::new(RailersEgui::new(cc))),
    )
}

#[derive(Default)]
struct RailersEgui {
    map_story_keys: bool,
    hsr_hwnd: Option<HWND>,
    trace_thread: bool,
    user_hidden: bool,
}

impl RailersEgui {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

fn utils_window(ctx: &Context, railers_egui: &mut RailersEgui) {
    egui::Window::new("Utils").show(ctx, |ui| {
        ui.label("These are utilities which enhance your gameplay experience.");
        let map_story_keys_checkbox = ui.checkbox(
            &mut railers_egui.map_story_keys,
            "Map story keys (F & Enter to Space)",
        );
        if map_story_keys_checkbox.changed() {
            debug!("Map story keys: {}", railers_egui.map_story_keys);
            ui.ctx().request_repaint();
        }
    });
}

fn debug_window(ctx: &Context, railers_egui: &mut RailersEgui) {
    egui::Window::new("Debug").show(ctx, |ui| {
        ui.label("Used internally for debugging purposes.");
        ui.label(format!("HSR HWND: {:?}", railers_egui.hsr_hwnd));
        let test_btn = ui.button("Resize window to 1600x900");
        if test_btn.clicked() {
            debug!("Test button clicked");
            unsafe {
                let hwnd = OUR_HWND.get().unwrap().clone();
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 1600, 900, SWP_NOMOVE).unwrap();
            };
        }
        let test_2_btn = ui.button("Resize window to HSR");
        if test_2_btn.clicked() {
            debug!("Test 2 button clicked");
            unsafe {
                let hwnd = OUR_HWND.get().unwrap().clone();
                let mut hsr_rect: RECT = Default::default();
                GetWindowRect(railers_egui.hsr_hwnd.unwrap(), &mut hsr_rect).unwrap();
                // debug!("HSR Rectangle: {:?}", hsr_rect);
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    // Idk why
                    hsr_rect.left + 8,
                    // 32 because of the title bar
                    hsr_rect.top + 30,
                    hsr_rect.right - hsr_rect.left - 16,
                    hsr_rect.bottom - hsr_rect.top - 38,
                    SWP_ASYNCWINDOWPOS,
                )
                .unwrap();
            };
        }
    });
}

static mut OUR_HWND: OnceLock<HWND> = OnceLock::new();

impl eframe::App for RailersEgui {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Get the window size
        unsafe {
            if OUR_HWND.get().is_none() {
                let active_hwnd = GetActiveWindow();
                if active_hwnd.0 == 0 {
                    return;
                }
                OUR_HWND.set(active_hwnd).unwrap();
            }
            if !self.trace_thread {
                thread::spawn(|| {
                    let mut hidden = false;
                    loop {
                        let hsr_hwnd = FindWindowA(s!("UnityWndClass"), s!("Honkai: Star Rail"));
                        let fg_hwnd = GetForegroundWindow();
                        let hwnd = OUR_HWND.get().unwrap().clone();
                        if (hsr_hwnd.0 == 0 || (fg_hwnd != hsr_hwnd && fg_hwnd != hwnd))
                            || USER_HIDDEN.get().unwrap().lock().unwrap().clone()
                        {
                            if hidden {
                                continue;
                            }
                            ShowWindow(OUR_HWND.get().unwrap().clone(), SW_HIDE);
                            hidden = true;
                            continue;
                        }
                        let mut hsr_rect: RECT = Default::default();
                        GetWindowRect(hsr_hwnd, &mut hsr_rect).unwrap();
                        // debug!("HSR Rectangle: {:?}", hsr_rect);
                        SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            // Idk why
                            hsr_rect.left + 8,
                            // 32 because of the title bar
                            hsr_rect.top + 30,
                            hsr_rect.right - hsr_rect.left - 16,
                            hsr_rect.bottom - hsr_rect.top - 38,
                            SWP_ASYNCWINDOWPOS,
                        )
                        .unwrap();
                        if hidden {
                            ShowWindow(hwnd, SW_SHOW);
                            hidden = false;
                        }
                        thread::sleep(Duration::from_nanos(1));
                    }
                });
            }
            self.trace_thread = true;
            if self.hsr_hwnd.is_none() || GetWindow(self.hsr_hwnd.unwrap(), GW_CHILD).0 == 0 {
                let hsr_hwnd = FindWindowA(s!("UnityWndClass"), s!("Honkai: Star Rail"));
                if hsr_hwnd.0 != 0 {
                    self.hsr_hwnd = Some(hsr_hwnd);
                } else {
                    debug!("HSR HWND not found");
                    self.hsr_hwnd = None;
                }
            }
        }
        utils_window(ctx, self);
        debug_window(ctx, self);
    }
}
