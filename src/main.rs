use eframe::egui;
use egui::{Context, Vec2, ViewportBuilder};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use log::{debug, error, info, trace};
use rdev::{listen, Event, EventType, Key};
use std::{
    ffi::c_void,
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
use tracing_subscriber::{self, fmt, EnvFilter};
use windows::{
    core::{s, Result},
    Win32::{
        Foundation::{BOOL, HWND, RECT, TRUE},
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED},
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
static MAP_STORY_KEYS_ENABLED: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();
static mut OUR_HWND: OnceLock<HWND> = OnceLock::new();

fn get_hsr_hwnd() -> HWND {
    unsafe { FindWindowA(s!("UnityWndClass"), s!("Honkai: Star Rail")) }
}

fn is_hsr_or_overlay_inactive(hsr_hwnd: HWND, our_hwnd: HWND) -> bool {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        trace!("FG HWND: {:?}", fg_hwnd);
        trace!("HSR HWND: {:?}", hsr_hwnd);
        trace!("OUR HWND: {:?}", our_hwnd);
        hsr_hwnd.0 == 0 || (fg_hwnd != hsr_hwnd && fg_hwnd != our_hwnd)
    }
}

fn main() -> eframe::Result<()> {
    setup_logger("debug").unwrap();
    // Initialize global variables
    USER_HIDDEN.set(Arc::new(Mutex::new(false))).unwrap();
    MAP_STORY_KEYS_ENABLED
        .set(Arc::new(Mutex::new(false)))
        .unwrap();
    // Initialize hotkey manager
    let manager = GlobalHotKeyManager::new().unwrap();
    // Toggle GUI
    let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::F10);
    manager.register(hotkey).unwrap();
    info!("Hotkey registered for GUI toggling.");
    thread::spawn(move || {
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                if event.state == HotKeyState::Released {
                    continue;
                } else {
                    // This must be the GUI toggle hotkey
                    unsafe {
                        let hsr_hwnd = get_hsr_hwnd();
                        let hwnd = OUR_HWND.get().unwrap().clone();
                        if is_hsr_or_overlay_inactive(hsr_hwnd, hwnd) {
                            continue;
                        }
                        debug!("Hotkey pressed: {:?}", event);
                        let mut user_hidden = USER_HIDDEN.get().unwrap().lock().unwrap();
                        *user_hidden = !*user_hidden;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    // Map story keys
    thread::spawn(|| {
        fn press_space_if_map_story_key() {
            let map_story_keys_enabled = MAP_STORY_KEYS_ENABLED.get().unwrap().lock().unwrap();
            if !*map_story_keys_enabled {
                return;
            }
            let hsr_hwnd = get_hsr_hwnd();
            let hwnd = unsafe { OUR_HWND.get().unwrap().clone() };
            if is_hsr_or_overlay_inactive(hsr_hwnd, hwnd) {
                return;
            }
            match rdev::simulate(&EventType::KeyPress(Key::Space)) {
                Ok(()) => (),
                Err(_) => {
                    error!("We could not send {:?}", Key::Space);
                }
            }
            thread::sleep(Duration::from_millis(10));
            match rdev::simulate(&EventType::KeyRelease(Key::Space)) {
                Ok(()) => (),
                Err(_) => {
                    error!("We could not send {:?}", Key::Space);
                }
            }
        }
        // This will block.
        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error)
        }

        fn callback(event: Event) {
            // debug!("My callback {:?}", event);
            match event.event_type {
                rdev::EventType::KeyPress(code) => {
                    debug!("Key press: {:?}", code);
                    match code {
                        Key::KeyF => press_space_if_map_story_key(),
                        Key::Return => press_space_if_map_story_key(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    });
    info!("Hotkeys registered for story key mapping.");
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
            let mut map_story_keys_enabled = MAP_STORY_KEYS_ENABLED.get().unwrap().lock().unwrap();
            *map_story_keys_enabled = railers_egui.map_story_keys;
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

impl eframe::App for RailersEgui {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Get the window size
        unsafe {
            if OUR_HWND.get().is_none() {
                let active_hwnd = GetActiveWindow();
                if active_hwnd.0 == 0 {
                    return;
                }
                let mut attrib: BOOL = TRUE;
                let attrib_ptr: *mut c_void = &mut attrib as *mut _ as *mut c_void;
                DwmSetWindowAttribute(
                    active_hwnd,
                    DWMWA_TRANSITIONS_FORCEDISABLED,
                    attrib_ptr,
                    std::mem::size_of::<BOOL>() as u32,
                )
                .unwrap();
                OUR_HWND.set(active_hwnd).unwrap();
            }
            if !self.trace_thread {
                thread::spawn(|| {
                    let mut hidden = false;
                    loop {
                        let hsr_hwnd = get_hsr_hwnd();
                        let hwnd = OUR_HWND.get().unwrap().clone();
                        if is_hsr_or_overlay_inactive(hsr_hwnd, hwnd)
                            || USER_HIDDEN.get().unwrap().lock().unwrap().clone()
                        {
                            if hidden {
                                continue;
                            }
                            hidden = true;
                            ShowWindow(OUR_HWND.get().unwrap().clone(), SW_HIDE);
                            SetForegroundWindow(hsr_hwnd);
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
                            hsr_rect.top + 31,
                            hsr_rect.right - hsr_rect.left - 16,
                            hsr_rect.bottom - hsr_rect.top - 39,
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
                let hsr_hwnd = get_hsr_hwnd();
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
