import { useState, useEffect } from "react";
import type { CreateVolumeRequest, Volume, VolumeIndexEntryFull } from "../../lib/volumes/types";
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
    const [parent, setParent] = useState<string | null>(null);
    const [allVolumes, setAllVolumes] = useState<VolumeIndexEntryFull[]>([]);

    function buildTree(list: VolumeIndexEntryFull[]): VolumeIndexEntryFull[] {
        // return as flat list in depth-first order, with same items (we'll render indentation separately)
        const map = new Map<string, (VolumeIndexEntryFull & { children: string[] })>();
        const roots: (VolumeIndexEntryFull & { children: string[] })[] = [];
        for (const it of list) map.set(it.id, { ...it, children: [] });
        for (const it of list) {
            const node = map.get(it.id)!;
            if (it.parent) {
                const p = map.get(it.parent);
                if (p) p.children.push(it.id);
                else roots.push(node);
            } else {
                roots.push(node);
            }
        }
        const out: VolumeIndexEntryFull[] = [];
        function walk(n: (VolumeIndexEntryFull & { children: string[] }), depth = 0) {
            out.push({ ...n, /* parent preserved */ });
            for (const cid of n.children) {
                const c = map.get(cid)!;
                walk(c, depth + 1);
            }
        }
        for (const r of roots) walk(r, 0);
        return out;
    }

    function collectDescendants(list: VolumeIndexEntryFull[], rootId: string): Set<string> {
        const map = new Map<string, string[]>();
        for (const it of list) {
            if (!map.has(it.parent ?? "")) map.set(it.parent ?? "", []);
            if (it.parent) {
                map.get(it.parent)!.push(it.id);
            }
        }
        const out = new Set<string>();
        const stack = [rootId];
        while (stack.length) {
            const cur = stack.pop()!;
            const children = map.get(cur) || [];
            for (const c of children) {
                if (!out.has(c)) {
                    out.add(c);
                    stack.push(c);
                }
            }
        }
        return out;
    }

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
                setParent(v.meta.parent ?? null);
            } catch (e) {
                console.error(e);
            }
        })();
        return () => { mounted = false; };
    }, [volumeId]);

    useEffect(() => {
        let mounted = true;
        (async () => {
            try {
                const list = await commands.listVolumes();
                if (!mounted) return;
                setAllVolumes(list || []);
            } catch (e) {
                console.error("failed to load volumes for parent select", e);
            }
        })();
        return () => { mounted = false; };
    }, []);

    async function handleCreate() {
        const req: CreateVolumeRequest = { title, content, description, tags: [] };
        const v = await commands.createVolume(req);
        if (parent) {
            try {
                await commands.nestVolume(parent, v.meta.id);
            } catch (err) {
                console.error("failed to set parent on create", err);
            }
        }
        onSaved?.(v);
    }

    async function handleSave() {
        if (!volumeId) return;
        const req: CreateVolumeRequest = { title, content, description, tags: [] };
        const v = await commands.editVolume(volumeId, req);
        // adjust parent if changed
        try {
            const currentParent = v.meta.parent ?? null;
            if (parent && parent !== currentParent) {
                // make selected parent the parent of this volume
                await commands.nestVolume(parent, v.meta.id);
            } else if (!parent && currentParent) {
                // remove parent
                await commands.flattenVolume(v.meta.id);
            }
        } catch (err) {
            console.error("failed to adjust parent on save", err);
        }
        onSaved?.(v);
    }

    return (
        <div className="volume-editor">
            <h3>{volumeId ? "Edit Volume" : "Create Volume"}</h3>
            <input className="input" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
            <label style={{ display: 'block', marginTop: 8 }}>Parent</label>
            <select className="input" value={parent ?? ""} onChange={(e) => setParent(e.target.value || null)}>
                <option value="">(none)</option>
                {(() => {
                    const flat = buildTree(allVolumes);
                    const descendants = volumeId ? collectDescendants(allVolumes, volumeId) : new Set<string>();
                    return flat.map((it) => (
                        <option key={it.id} value={it.id} disabled={it.id === volumeId || descendants.has(it.id)}>{it.title}</option>
                    ));
                })()}
            </select>
            <textarea className="textarea" placeholder="Content" value={content} onChange={(e) => setContent(e.target.value)} rows={16} />
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
