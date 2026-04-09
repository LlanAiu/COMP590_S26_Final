// builtin

// external

// internal
import type { VolumeIndexEntry } from "./types";


export function formatUpdatedAt(iso?: string) {
    if (!iso) return "";
    try {
        const d = new Date(iso);
        return d.toLocaleString();
    } catch {
        return iso;
    }
}

export function makeSnippet(content: string, max = 120) {
    if (!content) return "";
    const s = content.replace(/\s+/g, " ").trim();
    return s.length > max ? `${s.slice(0, max - 1)}...` : s;
}

export function indexEntryFromVolume(v: { meta: { id: string; title: string; updated_at: string }; content?: string }): VolumeIndexEntry {
    return { id: v.meta.id, title: v.meta.title, updated_at: v.meta.updated_at, snippet: makeSnippet(v.content || "") };
}
