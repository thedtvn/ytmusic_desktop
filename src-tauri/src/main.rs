// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use tauri::{
    async_runtime::{self},
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
};
use tokio::sync::mpsc::UnboundedSender;

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

fn create_discord_rpc() -> UnboundedSender<PlayerState> {
    let mut drpc = discord_rpc_client::Client::new(1049275932239728672);
    let drpc_event_cl = drpc.clone();
    drpc.on_ready(|_| {
        println!("Discord RPC Ready");
    });
    drpc.on_error(move |_| {
        let mut drpc_cl = drpc_event_cl.clone();
        drpc_cl.start();
    });
    drpc.start();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PlayerState>();
    async_runtime::spawn(async move {
        while let Some(data) = rx.recv().await {
            println!("Update state: {:?}", data);
            let mut drpc = drpc.clone();
            let _ = std::thread::spawn(move || {
                if data.is_distroyed {
                    drpc.set_activity(|a| a.details("idle not playing")).unwrap();
                } else {
                    let video_data = data.video_data.unwrap();
                    drpc.set_activity(|a| {
                        a
                            .instance(true)
                            .details(if data.is_playing { "Playing" } else { "Paused" })
                            .state(&format!("{} - {}", video_data.title, video_data.artist))
                            .assets(|ass| {
                                ass.large_image(&video_data.album_art)
                                    .small_image(if data.is_playing { "play" } else { "pause" })
                            })
                            .timestamps(|ts| {
                                let start = get_sys_time_in_secs() - video_data.current_duration as u64;
                                let end = start + video_data.duration as u64;
                                if data.is_playing {
                                    ts.start(start).end(end)
                                } else {
                                    ts
                                }
                            }).buttons(|x| x.add_button("Music Link", &video_data.url))
                    })
                    .unwrap();
                }
            });
        }
    });
    tx
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

#[tauri::command]
fn update_state(app_handle: tauri::AppHandle, data: PlayerState) {
    let sender = app_handle.state::<UnboundedSender<PlayerState>>();
    let _ = sender.send(data.clone());
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(create_discord_rpc());
            Ok(())
        })
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
