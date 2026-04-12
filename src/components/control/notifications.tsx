import { useEffect, useState } from "react";
import * as commands from "../../lib/commands";

export default function ControlNotifications({ title = "Since you were last here..." }: { title?: string }) {
    const [entries, setEntries] = useState<commands.ControlLogEntry[]>([]);
    const [loading, setLoading] = useState(true);

    async function load() {
        setLoading(true);
        try {
            const res = await commands.getControlLog();
            setEntries(res || []);
        } catch (err) {
            console.error("Failed to load control log", err);
            setEntries([]);
        }
        setLoading(false);
    }

    useEffect(() => {
        load();
    }, []);

    async function markAsRead() {
        try {
            await commands.clearControlLog();
            setEntries([]);
        } catch (err) {
            console.error("Failed to clear control log", err);
        }
    }

    return (
        <div className="control-panel card">
            <h4>{title}</h4>
            {loading ? (
                <div>Loading...</div>
            ) : (
                <ul className="control-log-list">
                    {entries.length === 0 ? (
                        <li className="muted">No updates</li>
                    ) : entries.map((e, i) => (
                        <li key={i}>
                            <div style={{ fontSize: 12, color: '#666' }}>{new Date(e.timestamp).toLocaleString()}</div>
                            <div>{e.description}</div>
                        </li>
                    ))}
                </ul>
            )}

            <div style={{ marginTop: 8 }}>
                <button type="button" onClick={markAsRead}>Mark as read</button>
            </div>
        </div>
    );
}
