import { useState, useEffect } from "react";
import type { CreateVolumeRequest, Volume } from "../../lib/volumes/types";
import * as commands from "../../lib/commands";
import "./volumes.css";

export default function VolumeEditor({
    volumeId,
    onSaved,
}: {
    volumeId?: string | null;
    onSaved?: (v: Volume) => void;
}) {
    const [title, setTitle] = useState("");
    const [content, setContent] = useState("");
    const [description, setDescription] = useState("");

    useEffect(() => {
        if (!volumeId) return;
        let mounted = true;
        (async () => {
            try {
                const v = await commands.readVolume(volumeId);
                if (!mounted) return;
                setTitle(v.meta.title);
                setContent(v.content);
                setDescription(v.meta.description || "");
            } catch (e) {
                console.error(e);
            }
        })();
        return () => { mounted = false; };
    }, [volumeId]);

    async function handleCreate() {
        const req: CreateVolumeRequest = { title, content, description, tags: [] };
        const v = await commands.createVolume(req);
        onSaved?.(v);
    }

    async function handleSave() {
        if (!volumeId) return;
        const req: CreateVolumeRequest = { title, content, description, tags: [] };
        const v = await commands.editVolume(volumeId, req);
        onSaved?.(v);
    }

    return (
        <div className="volume-editor">
            <h3>{volumeId ? "Edit Volume" : "Create Volume"}</h3>
            <input className="input" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
            <textarea className="textarea" placeholder="Content" value={content} onChange={(e) => setContent(e.target.value)} rows={8} />
            <input className="input" placeholder="Description" value={description} onChange={(e) => setDescription(e.target.value)} />
            <div className="actions">
                {!volumeId ? (
                    <button type="button" onClick={handleCreate}>Create</button>
                ) : (
                    <button type="button" onClick={handleSave}>Save</button>
                )}
            </div>
        </div>
    );
}
