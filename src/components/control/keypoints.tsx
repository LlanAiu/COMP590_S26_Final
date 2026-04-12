import React from "react";
import "../volumes/volumes.css";

export default function KeypointsPanel({ volumeId }: { volumeId?: string | null }) {
    return (
        <div className="control-panel card">
            <h4>Keypoints & Actions</h4>
            <div className="muted">No keypoints yet for {volumeId ? 'this volume' : 'selection'}.</div>
        </div>
    );
}
