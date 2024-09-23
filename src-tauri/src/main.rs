// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tauri::{
    async_runtime::RwLock, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
};

use std::time::SystemTime;

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

fn create_discord_rpc() -> Arc<RwLock<discord_rpc_client::Client>> {
    let mut drpc = discord_rpc_client::Client::new(1049275932239728672);
    drpc.start();
    Arc::new(RwLock::new(drpc))
}

fn system_tray_event(app_handle: tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            app_handle.get_window("main").unwrap().show().unwrap();
            app_handle.get_window("main").unwrap().set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                std::process::exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct VideoData {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub album_art: String,
    pub current_duration: f64,
    pub duration: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct PlayerState {
    pub is_playing: bool,
    pub is_distroyed: bool,
    pub video_data: Option<VideoData>,
}

#[tauri::command(async)]
async fn update_state(app_handle: tauri::AppHandle, data: PlayerState) {
    let drpc_cl = app_handle.state::<Arc<RwLock<discord_rpc_client::Client>>>();
    let drpc = drpc_cl.inner().clone();
    if data.is_distroyed {
        drpc.write_owned()
            .await
            .set_activity(|a| a.details("idle not playing"))
            .unwrap();
        return;
    } else {
        let video_data = data.video_data.unwrap();
        drpc.write_owned()
            .await
            .set_activity(|a| {
                let b = a.details(&format!("{} - {}", video_data.title, video_data.artist))
                    .assets(|ass| {
                        ass.large_image(&video_data.album_art).small_image(if data.is_playing { "play" } else { "pause" })
                    }).timestamps(|ts| {
                        let start = get_sys_time_in_secs() - video_data.current_duration as u64;
                        let end = start + video_data.duration as u64;
                        if data.is_playing {
                            ts.start(start).end(end)
                        } else {
                            ts
                        } 
                    });
                if data.is_playing {
                    b
                } else {
                    b.state("music paused")
                }
            })
            .unwrap();
        return;
    }
}

fn main() {
    tauri::Builder::default()
        .manage(create_discord_rpc())
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
