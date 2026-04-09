// builtin

// external
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeSanitize from "rehype-sanitize";
import rehypeHighlight from "rehype-highlight";
import { useEffect, useState } from "react";

// internal
import type { Volume } from "../../lib/volumes/types";
import "./volumes.css";
import { readVolume } from "../../lib/commands";

export default function VolumeDetail({ id }: { id: string }) {
    const [volume, setVolume] = useState<Volume | null>(null);
    const [loading, setLoading] = useState(false);

    // biome-ignore lint/correctness/useExhaustiveDependencies: Not right
    useEffect(() => {
        let mounted = true;
        if (!id) return;
        // If we already have the volume and ids match, don't re-fetch
        if (volume && volume.meta.id === id) return;
        (async () => {
            try {
                setLoading(true);
                const v = await readVolume(id);
                if (!mounted) return;
                setVolume(v);
            } catch (e) {
                console.error("failed to load volume", e);
            } finally {
                if (mounted) setLoading(false);
            }
        })();
        return () => { mounted = false; };
    }, [id]);

    if (loading) return <div>Loading...</div>;
    if (!volume) return <div>No volume selected</div>;

    return (
        <div>
            <div><strong>{volume.meta.title}</strong> ({volume.meta.id})</div>
            <div className="volume-meta">{volume.meta.updated_at}</div>
            <div className="volume-detail-body">
                <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize, rehypeHighlight]}>
                    {volume.content}
                </ReactMarkdown>
            </div>
        </div>
    );
}
