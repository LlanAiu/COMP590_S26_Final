// builtin

// external
import { invoke } from "@tauri-apps/api/core";

// internal
import "./App.css";


export default function App() {

    function handleStartRecord() {
        invoke("start_audio_recording", {})
    }

    return (
        <div>
            <h1>It's Beautiful</h1>
            <button type="button" onClick={handleStartRecord}>Record Audio</button>
        </div>
    );
}

