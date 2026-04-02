// builtin

// external
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

// internal
import "./App.css";


export default function App() {

    const [query, setQuery] = useState<string>("");
    const [response, setResponse] = useState<string>("");

    function handleStartRecord() {
        invoke("start_audio_recording", {});
    }

    function handleStopRecord() {
        invoke("stop_audio_recording", {});
    }

    function handleQueryChange(e: React.ChangeEvent<HTMLInputElement>) {
        setQuery(_ => e.target.value);
    }

    function handleSendMessage() {
        invoke("send_message", { message: query }).then((res) => setResponse(res as string));
    }

    return (
        <div>
            <h1>It's Beautiful</h1>

            <div>
                <button type="button" onClick={handleStartRecord}>Record Audio</button>
                <button type="button" onClick={handleStopRecord}>Stop Recording</button>
            </div>

            <div>
                <input
                    type="text"
                    onChange={handleQueryChange}
                    value={query}
                />
                <button type="button" onClick={handleSendMessage}>Send Message</button>

                {
                    response && <div>{response}</div>
                }
            </div>
        </div>
    );
}

