import React, { useEffect, useState } from "react";
import "../volumes/volumes.css";
import * as commands from "../../lib/commands";

export default function KeypointsPanel({ volumeId }: { volumeId?: string | null }) {
    const [keypoints, setKeypoints] = useState<string[] | null>(null);

    useEffect(() => {
        let mounted = true;
        async function load() {
            if (!volumeId) {
                setKeypoints(null);
                return;
            }
            try {
                const vol = await commands.readVolume(volumeId);
                if (!mounted) return;
                setKeypoints(vol.meta.keypoints || null);
            } catch (e) {
                console.error("Failed to load volume for keypoints", e);
                if (mounted) setKeypoints(null);
            }
        }
        load();
        return () => {
            mounted = false;
        };
    }, [volumeId]);

    return (
        <div className="control-panel card">
            <h4>Keypoints & Actions</h4>
            {volumeId ? (
                keypoints && keypoints.length > 0 ? (
                    <ul>
                        {keypoints.map((k, i) => (
                            <li key={i}>{k}</li>
                        ))}
                    </ul>
                ) : (
                    <div className="muted">No keypoints yet for this volume.</div>
                )
            ) : (
                <div className="muted">No volume selected.</div>
            )}
        </div>
    );
}
