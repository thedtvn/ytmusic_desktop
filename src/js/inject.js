// This is list event youtube call when you change video or pause ...
[
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
    "onVideoProgress",
    "onLoadProgress",
    "onShowControls",
    "onHideControls",
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
let last_send = null;

async function update_state(event_id) {
    if (event_id == undefined) {
        event_id = ytplayerapi.getPlayerState()
    }
    let data = null;
    if (event_id == 1) {
        data = {
            is_playing: true,
            is_distroyed: false,
            video_data: get_video_info()
        }
    } else if (event_id == 2) {
        data = {
            is_playing: false,
            is_distroyed: false,
            video_data: get_video_info()
        }
    } else if (event_id == -1 || event_id == 5) {
        data = {
            is_playing: false,
            is_distroyed: true
        }
    } else return;
    console.log("update_state", last_send == data, last_send, data);
    if (JSON.stringify(last_send) == JSON.stringify(data) && event_id != undefined) return
    await tauri_api.invoke("update_state", { data: data });
    last_send = data;
    last_update = Date.now();
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
    let skip_time = event - last_duration;
    console.log("skip_time", skip_time);
    if (skip_time > 1 || skip_time < -1 || skip_time == 0) {
        update_state()
    }
    last_duration = event;
});
