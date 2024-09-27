// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
};
use tokio::sync::Mutex;

use std::{sync::Arc, time::SystemTime};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

use lazy_static::lazy_static;

lazy_static! {
    static ref DRPC_CLIENT: Arc<Mutex<DiscordIpcClient>> = Arc::new(Mutex::new(DiscordIpcClient::new("1049275932239728672").unwrap()));
}

fn get_sys_time_in_secs() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

fn create_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new().add_item(quit);
    SystemTray::new().with_menu(tray_menu)
}

fn update_status(data: PlayerState) {
    println!("Update state: {:?}", data);
    let status_activity = activity::Activity::new();
    if data.is_distroyed {
        DRPC_CLIENT.blocking_lock().set_activity(status_activity.details("idle not playing")).unwrap();
    } else {
        let video_data = data.video_data.unwrap();
        let acess = activity::Assets::new();
        let time_stam = activity::Timestamps::new();
        let start = get_sys_time_in_secs() - video_data.current_duration as u64;
        let end = start + video_data.duration as u64;
        DRPC_CLIENT.blocking_lock().set_activity(
            status_activity.details("idle not playing")
                .details(if data.is_playing { "Playing" } else { "Paused" })
                .state(&format!("{} - {}", video_data.title, video_data.artist))
                .assets(
                    acess.large_image(&video_data.album_art)
                        .small_image(if data.is_playing { "play" } else { "pause" })
                )
                .timestamps(
                    if data.is_playing {
                        time_stam.start(start as i64).end(end as i64)
                    } else {
                        time_stam
                    }
                ).buttons(vec![activity::Button::new("Play on YouTube Music", &video_data.url)]),
        )
        .unwrap();
    }
}

fn system_tray_event(app_handle: tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            app_handle.get_window("main").unwrap().show().unwrap();
            app_handle.get_window("main").unwrap().set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                DRPC_CLIENT.blocking_lock().close().unwrap();
                std::process::exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct VideoData {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub album_art: String,
    pub current_duration: f64,
    pub duration: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct PlayerState {
    pub is_playing: bool,
    pub is_distroyed: bool,
    pub video_data: Option<VideoData>,
}

#[tauri::command]
fn update_state(data: PlayerState) {
    update_status(data.clone());
}

fn main() {
    std::thread::spawn(move || {
        let _ = DRPC_CLIENT.blocking_lock().connect();
    });
    tauri::Builder::default()
        .system_tray(create_tray())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            app.get_window("main").unwrap().show().unwrap();
            app.get_window("main").unwrap().set_focus().unwrap();
        }))
        .on_system_tray_event(|a, e| system_tray_event(a.clone(), e))
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                event.window().hide().unwrap();
            }
            _ => {}
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_page_load(|window, _page_load_payload| {
            let inject_code = include_str!("inject.js");
            let load_lib = include_str!("load_lib.js");
            let js_script = format!(
                "
                window.resolveLoad = null;
                let done_load_script = new Promise((resolve) => {{
                    window.resolveLoad = resolve;
                }});
                let script = document.createElement(\"script\");
                script.type = \"module\";
                script.innerHTML = `{}`;
                document.head.appendChild(script);
                addEventListener(\"load\", async () => {{
                    await done_load_script;
                    try {{
                        {}
                    }} catch (error) {{
                        console.error(error);
                    }}
                }})
            ",
                load_lib, inject_code
            );
            window.eval(&js_script).unwrap();
        })
        .invoke_handler(tauri::generate_handler![update_state])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
