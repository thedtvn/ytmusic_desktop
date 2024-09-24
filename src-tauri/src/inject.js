let events = [
    "onPlaybackStartExternal",
    "onStateChange",
    "onReady",
    "onVideoDataChange",
    "onPlaylistNext",
    "onPlaylistPrevious",
    "onDetailedError",
    "onAutoplayBlocked",
    "onVolumeChange",
    "onCaptionsModuleAvailable",
    "innertubeCommand",

    // Use to get live time about video progress
    // "onVideoProgress",
    // "onLoadProgress",

    // Show / Hide Control
    // "onShowControls",
    // "onHideControls",

    // ads event
    "onAdStart",
    "onAdEnd",
    "onAdMetadataAvailable",
    "onAdStateChange",
];

let yt_control_bar = document.querySelector("ytmusic-app-layout>ytmusic-player-bar");
window.ytplayerapi = yt_control_bar.playerApi;

function get_video_image() {
    return window.ytplayerapi.getPlayerResponse().videoDetails.thumbnail.thumbnails[0].url;
}

function get_video_info() {
    let info = ytplayerapi.getVideoData();
    return {
        title: info.title,
        artist: info.author,
        duration: ytplayerapi.getDuration(),
        current_duration: ytplayerapi.getCurrentTime(),
        url: `https://music.youtube.com/watch?v=${info.video_id}`,
        album_art: get_video_image()
    };
}

let last_update = 0;
let last_duration = ytplayerapi.getCurrentTime();

async function update_state(event_id) {
    last_update = Date.now();
    if (event_id == undefined) {
        event_id = ytplayerapi.getPlayerState()
    }
    if (event_id == 1) {
        let data = {
            is_playing: true,
            is_distroyed: false,
            video_data: get_video_info()
        }
        await tauri_api.invoke("update_state", { data: data });
    } else if (event_id == 2) {
        let data = {
            is_playing: false,
            is_distroyed: false,
            video_data: get_video_info()
        }
        await tauri_api.invoke("update_state", { data: data })
    } else if (event_id == -1) {
        let data = {
            is_playing: false,
            is_distroyed: true
        }
        await tauri_api.invoke("update_state", { data: data });
    }
}

ytplayerapi.addEventListener("onStateChange", async (event) => {
    last_duration = ytplayerapi.getCurrentTime();
    update_state(event.state);
});

setInterval(() => {
    if (Date.now() - last_update < 15000) return;
    update_state();
}, 1000);

ytplayerapi.addEventListener("onVideoProgress", async (event) => {
    if (event - last_duration > 5) { 
        update_state(event.state) 
    }
    last_duration = event;
});
