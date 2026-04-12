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
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        let mounted = true;
        if (!id) return;
        (async () => {
            try {
                setError(null);
                setLoading(true);
                const v = await readVolume(id);
                if (!mounted) return;
                setVolume(v);
            } catch (e) {
                console.error("failed to load volume", e);
                if (mounted) setError(String(e));
            } finally {
                if (mounted) setLoading(false);
            }
        })();
        return () => { mounted = false; };
    }, [id]);

    if (loading) return <div>Loading...</div>;
    if (!volume) return <div>{error ? `Failed to load volume: ${error}` : "No volume selected"}</div>;

    return (
        <div>
            <div className="volume-header">
                <div className="volume-title">{volume.meta.title}</div>
                <div className="volume-detail-meta">{new Date(volume.meta.updated_at).toLocaleString()}</div>
            </div>
            <div className="volume-detail-body">
                <ReactMarkdown key={volume.meta.version} remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize, rehypeHighlight]}>
                    {volume.content}
                </ReactMarkdown>
            </div>
        </div>
    );
}
