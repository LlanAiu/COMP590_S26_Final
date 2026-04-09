// builtin

// external
import { useEffect, useState } from "react";

// internal
import type { VolumeIndexEntry } from "../../lib/volumes/types";
import * as commands from "../../lib/commands";
import "./volumes.css";


export default function AllVolumes({ onOpen }: { onOpen?: (id: string) => void }) {
    const [list, setList] = useState<VolumeIndexEntry[]>([]);

    async function refresh() {
        try {
            const res = await commands.listVolumes();
            setList(res || []);
        } catch (e) {
            console.error("list volumes", e);
        }
    }

    // biome-ignore lint/correctness/useExhaustiveDependencies: Run only once at startup
    useEffect(() => {
        refresh();
    }, []);

    return (
        <div className="volumes">
            <h3>Volumes</h3>
            <button type="button" onClick={refresh}>Refresh</button>
            <ul className="volume-list">
                {list.map((it) => (
                    <li key={it.id} className="volume-item">
                        <div><strong>{it.title}</strong></div>
                        <div className="volume-meta">{it.updated_at}</div>
                        <div className="volume-actions">
                            <button type="button" onClick={() => onOpen?.(it.id)}>Open</button>
                        </div>
                    </li>
                ))}
            </ul>
        </div>
    );
}

