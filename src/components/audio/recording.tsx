// builtin

// external
import { useState } from "react";

// internal
import * as commands from "../../lib/commands";
import type { RecordingState } from "../../lib/audio/types";
import "./recording.css";


export default function Recording() {
    const [state, setState] = useState<RecordingState>("idle");

    async function start() {
        try {
            setState("recording");
            await commands.startAudioRecording();
        } catch (e) {
            console.error(e);
            setState("idle");
        }
    }

    async function stop() {
        try {
            setState("processing");
            await commands.stopAudioRecording();
        } catch (e) {
            console.error(e);
        } finally {
            setState("idle");
        }
    }

    return (
        <div className="recording">
            <h3>Recording</h3>
            <div className="recording-state">State: {state}</div>
            <div className="recording-controls">
                <button type="button" onClick={start}>Record Audio</button>
                <button type="button" onClick={stop}>Stop Recording</button>
            </div>
        </div>
    );
}

