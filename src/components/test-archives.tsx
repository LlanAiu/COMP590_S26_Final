// builtin

// external
import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import rehypeHighlight from "rehype-highlight";

// internal
import * as commands from "../lib/commands";
import type { VolumeIndexEntry, Volume, CreateVolumeRequest } from "../lib/volumes/types";
import "./test-archives.css";


export default function TestArchives() {
    const [list, setList] = useState<VolumeIndexEntry[]>([]);
    const [selectedId, setSelectedId] = useState<string | null>(null);
    const [volume, setVolume] = useState<Volume | null>(null);

    const [title, setTitle] = useState("");
    const [content, setContent] = useState("");
    const [description, setDescription] = useState("");

    async function refreshList() {
        try {
            const res = await commands.listVolumes();
            setList(res || []);
        } catch (e) {
            console.error("list error", e);
        }
    }

    async function handleCreate() {
        try {
            const req: CreateVolumeRequest = { title, content, description, tags: [] };
            const v = await commands.createVolume(req);
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
            const v = await commands.readVolume(useId);
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
            const req: CreateVolumeRequest = { title, content, description, tags: [] };
            const v = await commands.editVolume(selectedId, req);
            setVolume(v);
            await refreshList();
        } catch (e) {
            console.error("edit error", e);
        }
    }

    async function handleDelete() {
        if (!selectedId) return;
        try {
            await commands.deleteVolume(selectedId);
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
        <div className="test-archives">
            <h2>Test Archives</h2>

            <div className="ta-flex">
                <div className="ta-main">
                    <h3>Create / Edit</h3>
                    <input className="form-input" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
                    <textarea className="form-textarea" placeholder="Content" value={content} onChange={(e) => setContent(e.target.value)} rows={8} />
                    <input className="form-input" placeholder="Description" value={description} onChange={(e) => setDescription(e.target.value)} />
                    <div className="form-row">
                        <button className="btn-spaced" type="button" onClick={handleCreate}>Create</button>
                        <button className="btn-spaced" type="button" onClick={handleEdit} disabled={!selectedId}>Save</button>
                    </div>
                </div>

                <div className="ta-sidebar">
                    <h3>Volumes</h3>
                    <button type="button" onClick={refreshList}>Refresh</button>
                    <ul>
                        {list.map((it) => (
                            <li key={it.id} className="list-item">
                                <div>
                                    <strong>{it.title}</strong>
                                </div>
                                <div className="meta-small">{it.updated_at}</div>
                                <div className="controls">
                                    <button type="button" onClick={() => handleRead(it.id)}>Open</button>
                                    <button className="btn-spaced" type="button" onClick={async () => { await handleRead(it.id); }}>Load</button>
                                </div>
                            </li>
                        ))}
                    </ul>
                </div>
            </div>

            <div className="selected">
                <h3>Selected</h3>
                {volume ? (
                    <div>
                        <div><strong>{volume.meta.title}</strong> ({volume.meta.id})</div>
                        <div className="meta-small">{volume.meta.updated_at}</div>
                        <div className="content-box">
                            <ReactMarkdown
                                remarkPlugins={[remarkGfm]}
                                rehypePlugins={[rehypeSanitize, rehypeHighlight]}
                            >
                                {volume.content}
                            </ReactMarkdown>
                        </div>
                        <div className="form-row">
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
