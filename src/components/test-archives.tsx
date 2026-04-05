import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import rehypeHighlight from "rehype-highlight";
import { invoke } from "@tauri-apps/api/core";

type VolumeIndexEntry = { id: string; title: string; updated_at: string; snippet?: string };
type VolumeMeta = { id: string; title: string; description?: string; created_at: string; updated_at: string; tags: string[]; version: number };
type Volume = { meta: VolumeMeta; content: string; attachments: string[] };

export default function TestArchives() {
    const [list, setList] = useState<VolumeIndexEntry[]>([]);
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const [volume, setVolume] = useState<Volume | null>(null);

    const [title, setTitle] = useState("");
    const [content, setContent] = useState("");
    const [description, setDescription] = useState("");

    async function refreshList() {
        try {
            const res = await invoke<VolumeIndexEntry[]>("list_volumes");
            setList(res || []);
        } catch (e) {
            console.error("list error", e);
        }
    }

    async function handleCreate() {
        try {
            const req = { title, content, description, tags: [] };
            const v = await invoke<Volume>("create_volume", { req });
            setVolume(v);
            setSelectedId(v.meta.id);
            await refreshList();
        } catch (e) {
            console.error("create error", e);
        }
    }

    async function handleRead(id?: string) {
        const useId = id ?? selectedId;
        if (!useId) return;
        try {
            const v = await invoke<Volume>("read_volume", { id: useId });
            setVolume(v);
            setTitle(v.meta.title);
            setContent(v.content);
            setDescription(v.meta.description || "");
            setSelectedId(useId);
        } catch (e) {
            console.error("read error", e);
        }
    }

    async function handleEdit() {
        if (!selectedId) return;
        try {
            const req = { title, content, description, tags: [] };
            const v = await invoke<Volume>("edit_volume", { id: selectedId, req });
            setVolume(v);
            await refreshList();
        } catch (e) {
            console.error("edit error", e);
        }
    }

    async function handleDelete() {
        if (!selectedId) return;
        try {
            await invoke<void>("delete_volume", { id: selectedId });
            setVolume(null);
            setSelectedId(null);
            await refreshList();
        } catch (e) {
            console.error("delete error", e);
        }
    }

    // biome-ignore lint/correctness/useExhaustiveDependencies: Do once on startup
    useEffect(() => {
        refreshList();
    }, []);

    return (
        <div style={{ padding: 12, border: "1px solid #ddd", borderRadius: 6 }}>
            <h2>Test Archives</h2>

            <div style={{ display: "flex", gap: 12 }}>
                <div style={{ flex: 1 }}>
                    <h3>Create / Edit</h3>
                    <input placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} style={{ width: "100%" }} />
                    <textarea placeholder="Content" value={content} onChange={(e) => setContent(e.target.value)} rows={8} style={{ width: "100%", marginTop: 8 }} />
                    <input placeholder="Description" value={description} onChange={(e) => setDescription(e.target.value)} style={{ width: "100%", marginTop: 8 }} />
                    <div style={{ marginTop: 8 }}>
                        <button type="button" onClick={handleCreate}>Create</button>
                        <button type="button" onClick={handleEdit} disabled={!selectedId} style={{ marginLeft: 8 }}>Save</button>
                    </div>
                </div>

                <div style={{ width: 320 }}>
                    <h3>Volumes</h3>
                    <button type="button" onClick={refreshList}>Refresh</button>
                    <ul>
                        {list.map((it) => (
                            <li key={it.id} style={{ marginTop: 8 }}>
                                <div>
                                    <strong>{it.title}</strong>
                                </div>
                                <div style={{ fontSize: 12, color: "#666" }}>{it.updated_at}</div>
                                <div style={{ marginTop: 4 }}>
                                    <button type="button" onClick={() => handleRead(it.id)}>Open</button>
                                    <button type="button" onClick={async () => { await handleRead(it.id); }} style={{ marginLeft: 6 }}>Load</button>
                                </div>
                            </li>
                        ))}
                    </ul>
                </div>
            </div>

            <div style={{ marginTop: 12 }}>
                <h3>Selected</h3>
                {volume ? (
                    <div>
                        <div><strong>{volume.meta.title}</strong> ({volume.meta.id})</div>
                        <div style={{ fontSize: 12, color: "#666" }}>{volume.meta.updated_at}</div>
                        <div style={{ background: "#fff", padding: 12, border: "1px solid #eee", borderRadius: 6 }}>
                            <ReactMarkdown
                                remarkPlugins={[remarkGfm]}
                                rehypePlugins={[rehypeSanitize, rehypeHighlight]}
                            >
                                {volume.content}
                            </ReactMarkdown>
                        </div>
                        <div style={{ marginTop: 8 }}>
                            <button type="button" onClick={handleDelete}>Delete</button>
                        </div>
                    </div>
                ) : (
                    <div>No volume selected</div>
                )}
            </div>
        </div>
    );
}
