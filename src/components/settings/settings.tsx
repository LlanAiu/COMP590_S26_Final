import React, { useEffect, useState } from "react";
import { getSettings, saveSettings, reloadSettings, Settings } from "../../lib/commands";

const MODELS = ["gemma3:1b", "gemma3:4b", "gemma4:e2b"];

export default function SettingsPage() {
    const [settings, setSettings] = useState<Settings | null>(null);
    const [status, setStatus] = useState<string | null>(null);

    useEffect(() => {
        getSettings()
            .then((s) => setSettings(s))
            .catch((e) => setStatus("Failed to load settings: " + String(e)));
    }, []);

    if (!settings) {
        return <div style={{ padding: 16 }}>Loading settings...</div>;
    }

    const onSave = async () => {
        setStatus("Saving...");
        try {
            await saveSettings(settings);
            await reloadSettings();
            setStatus("Saved and reloaded.");
        } catch (e) {
            setStatus("Failed to save: " + String(e));
        }
        setTimeout(() => setStatus(null), 3000);
    };

    return (
        <div style={{ padding: 16, maxWidth: 640 }}>
            <h3>Settings</h3>
            <div style={{ marginTop: 8 }}>
                <label style={{ display: "block", marginBottom: 6 }}>Summarization model</label>
                <select value={settings.summarization_model} onChange={(e) => setSettings({ ...settings, summarization_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div style={{ marginTop: 12 }}>
                <label style={{ display: "block", marginBottom: 6 }}>Writer model</label>
                <select value={settings.writer_model} onChange={(e) => setSettings({ ...settings, writer_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div style={{ marginTop: 12 }}>
                <label style={{ display: "block", marginBottom: 6 }}>Control model</label>
                <select value={settings.control_model} onChange={(e) => setSettings({ ...settings, control_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div style={{ marginTop: 16 }}>
                <button onClick={onSave} className="primary">Save</button>
                {status ? <span style={{ marginLeft: 12 }}>{status}</span> : null}
            </div>
        </div>
    );
}
