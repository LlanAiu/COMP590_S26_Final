// builtin

// external
import { invoke } from "@tauri-apps/api/core";

// internal
import "./App.css";
import TestArchives from "./components/test-archives";


export default function App() {
    function handleStartRecord() {
        invoke("start_audio_recording", {});
    }

    function handleStopRecord() {
        invoke("stop_audio_recording", {});
    }

    return (
        <div>
            <h1>It's Beautiful</h1>

            <div>
                <button type="button" onClick={handleStartRecord}>Record Audio</button>
                <button type="button" onClick={handleStopRecord}>Stop Recording</button>
            </div>

            <div style={{ marginTop: 24 }}>
                <TestArchives />
            </div>
        </div>
    );
}

