// builtin

// external

// internal
import type { VolumeIndexEntry } from "../../lib/volumes/types";
import "./volumes.css";


export default function VolumeItem({ entry, onOpen }: { entry: VolumeIndexEntry; onOpen?: (id: string) => void }) {
    return (
        <li className="volume-item">
            <div><strong>{entry.title}</strong></div>
            <div className="volume-meta">{entry.updated_at}</div>
            <div className="volume-actions">
                <button type="button" onClick={() => onOpen?.(entry.id)}>Open</button>
            </div>
        </li>
    );
}
