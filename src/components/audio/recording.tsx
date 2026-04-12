// builtin

// external
import { useState } from "react";

// internal
import * as commands from "../../lib/commands";
import type { RecordingState } from "../../lib/audio/types";
import "./recording.css";


export default function Recording({ compact }: { compact?: boolean } = {}) {
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

    if (compact) {
        const isRecording = state === "recording";
        return (
            <div className="recording recording-compact" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div className="indicator" style={{ width: 12, height: 12, borderRadius: 999, background: isRecording ? 'linear-gradient(90deg, #ff6b6b, #ffb86b)' : '#444' }} />
                <button type="button" className={isRecording ? 'primary' : ''} onClick={() => (isRecording ? stop() : start())}>{isRecording ? 'Stop' : 'Record'}</button>
            </div>
        );
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

