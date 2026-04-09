// builtin

// external
import { useEffect, useState } from "react";

// internal
import type { VolumeIndexEntryFull } from "../../lib/volumes/types";
import * as commands from "../../lib/commands";
import VolumeTree from "./volume-tree";
import "./volumes.css";


export default function AllVolumes({ onOpen }: { onOpen?: (id: string) => void }) {
    const [list, setList] = useState<VolumeIndexEntryFull[]>([]);

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
            <VolumeTree list={list} onRefresh={refresh} onOpen={onOpen} />
        </div>
    );
}

