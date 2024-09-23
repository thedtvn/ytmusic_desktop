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
        url: ytplayerapi.getVideoUrl(),
        album_art: get_video_image()
    };
}

ytplayerapi.addEventListener("onStateChange", async (event) => {
    console.log("onStateChange", event);
    if (event == 1) {
        let data = {
            is_playing: true,
            is_distroyed: false,
            video_data: get_video_info()
        }
        await tauri_api.invoke("update_state", {data: data});
    } else if (event == 2) {
        let data = {
            is_playing: false,
            is_distroyed: false,
            video_data: get_video_info()
        }
        await tauri_api.invoke("update_state", {data: data})
    } else if (event == -1) {
        let data = {
            is_playing: false,
            is_distroyed: true
        }
        await tauri_api.invoke("update_state", {data: data});
    }
    console.log("onStateChange", event, window.ytplayerapi.getVideoData());
});

