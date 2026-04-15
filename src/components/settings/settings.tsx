import React, { useEffect, useState } from "react";
import { getSettings, saveSettings, reloadSettings, Settings } from "../../lib/commands";
import "./settings.css";

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
        <div className="card settings-card">
            <h3>Settings</h3>
            <div className="settings-field">
                <label>Summarization model</label>
                <select value={settings.summarization_model} onChange={(e) => setSettings({ ...settings, summarization_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div className="settings-field">
                <label>Writer model</label>
                <select value={settings.writer_model} onChange={(e) => setSettings({ ...settings, writer_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div className="settings-field">
                <label>Control model</label>
                <select value={settings.control_model} onChange={(e) => setSettings({ ...settings, control_model: e.target.value })}>
                    {MODELS.map((m) => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                </select>
            </div>

            <div className="settings-controls">
                <button onClick={onSave} className="primary">Save</button>
                {status ? <span className="settings-status">{status}</span> : null}
            </div>
        </div>
    );
}
