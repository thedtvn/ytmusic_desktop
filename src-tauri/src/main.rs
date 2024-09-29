// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};
use tokio::sync::Mutex;

use std::{sync::Arc, time::SystemTime};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

fn get_sys_time_in_secs() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

fn create_tray() -> SystemTray {
    let previous = CustomMenuItem::new("previous".to_string(), "Previous Song");
    let play_pause = CustomMenuItem::new("play_pause".to_string(), "Play");
    let next = CustomMenuItem::new("next".to_string(), "Next Song");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new().add_item(previous).add_item(play_pause).add_item(next).add_native_item(SystemTrayMenuItem::Separator).add_item(quit);
    SystemTray::new().with_menu(tray_menu)
}

fn update_status(dipc_client: Arc<Mutex<DiscordIpcClient>>, data: PlayerState) {
    println!("Update state: {:?}", data);
    let status_activity = activity::Activity::new();
    if data.is_distroyed {
        let _ = dipc_client.blocking_lock().set_activity(status_activity.details("idle not playing"));
    } else {
        let video_data = data.video_data.unwrap();
        let acess = activity::Assets::new();
        let time_stam = activity::Timestamps::new();
        let start = get_sys_time_in_secs() - video_data.current_duration as u64;
        let end = start + video_data.duration as u64;
        let _ = dipc_client.blocking_lock().set_activity(
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
        );
    }
}

fn system_tray_event(app_handle: tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            let main = app_handle.get_window("main").unwrap();
            main.unminimize().unwrap();
            main.show().unwrap();
            main.set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                let dipc_client = app_handle.state::<Arc<Mutex<DiscordIpcClient>>>().inner();
                dipc_client.blocking_lock().close().unwrap();
                std::process::exit(0);
            },
            "play_pause" => {
                app_handle.emit_to("main", "control_player", "play_pause").unwrap();
            },
            "previous" => {
                app_handle.emit_to("main", "control_player", "previous").unwrap();
            },
            "next" => {
                app_handle.emit_to("main", "control_player", "next").unwrap();
            },
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
fn update_state(app: tauri::AppHandle, data: PlayerState) {
    let dipc_client = app.state::<Arc<Mutex<DiscordIpcClient>>>().inner();

    app.tray_handle().get_item("play_pause").set_enabled(!data.is_distroyed).unwrap();
    app.tray_handle().get_item("previous").set_enabled(!data.is_distroyed).unwrap();
    app.tray_handle().get_item("next").set_enabled(!data.is_distroyed).unwrap();

    app.tray_handle().get_item("play_pause").set_title(if data.is_playing { "Pause" } else { "Play" }).unwrap();

    update_status(dipc_client.clone(), data.clone());
}

fn main() {
    let drpc_client = Arc::new(Mutex::new(DiscordIpcClient::new("1049275932239728672").unwrap()));
    let drpc_client_th = drpc_client.clone();
    std::thread::spawn(move || {
        drpc_client_th.blocking_lock().connect().unwrap();
        println!("Connected to discord rpc");
    });
    tauri::Builder::default()
        .manage(drpc_client)
        .system_tray(create_tray())
        .plugin(tauri_plugin_single_instance::init(|app_handle, _, _| {
            let main = app_handle.get_window("main").unwrap();
            main.unminimize().unwrap();
            main.show().unwrap();
            main.set_focus().unwrap();
        }))
        .on_system_tray_event(|a, e| system_tray_event(a.clone(), e))
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                event.window().minimize().unwrap();
                event.window().hide().unwrap();
            }
            _ => {}
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_page_load(|window, _page_load_payload| {
            let js_script = "
            if (window.trustedTypes && window.trustedTypes.createPolicy) { // Feature testing
                    window.trustedTypes.createPolicy('default', {
                        createHTML: (string) => DOMPurify.sanitize(string, {RETURN_TRUSTED_TYPE: true}),
                        createScriptURL: string => string, // warning: this is unsafe!
                        createScript: string => string, // warning: this is unsafe!
                    });
                }
                window.resolveLoad = null;
                let done_load_script = new Promise((resolve) => {
                    window.resolveLoad = resolve;
                });
                let script = document.createElement(\"script\");
                script.type = \"module\";
                script.src = \"https://tauri.localhost/js/load_lib.js\";
                document.head.appendChild(script);
                addEventListener(\"load\", async () => {
                    await done_load_script;
                    try {
                        let script = document.createElement(\"script\");
                        script.type = \"module\";
                        script.src = \"https://tauri.localhost/js/inject.js\";
                        document.head.appendChild(script);
                    } catch (error) {
                        console.error(error);
                    }
                })
            ";
            window.eval(&js_script).unwrap();
        })
        .invoke_handler(tauri::generate_handler![update_state])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
