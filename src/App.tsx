// builtin

// external
import { useState, useEffect } from "react";

// internal
import Recording from "./components/audio/recording";
import AllVolumes from "./components/volumes/all-volumes";
import ControlNotifications from "./components/control/notifications";
import VolumeDetail from "./components/volumes/volume-detail";
import VolumeEditor from "./components/volumes/volume-editor";
import "./App.css";


export default function App() {
    const [openVolumeId, setOpenVolumeId] = useState<string | null>(null);
    const [mode, setMode] = useState<"list" | "view" | "edit" | "create">("list");

    useEffect(() => {
        const log = (e: DragEvent) => {
            try {
                console.debug(`[global ${e.type}] target=${(e.target as HTMLElement)?.className || (e.target as HTMLElement)?.id || e.target}`);
            } catch {
                console.debug(`[global ${e.type}]`);
            }
        };
        window.addEventListener("dragenter", log, true);
        window.addEventListener("dragover", log, true);
        window.addEventListener("dragleave", log, true);
        window.addEventListener("drop", log, true);
        return () => {
            window.removeEventListener("dragenter", log, true);
            window.removeEventListener("dragover", log, true);
            window.removeEventListener("dragleave", log, true);
            window.removeEventListener("drop", log, true);
        };
    }, []);

    function handleOpen(id: string) {
        setOpenVolumeId(id);
        setMode("view");
    }

    function handleEdit(id: string) {
        setOpenVolumeId(id);
        setMode("edit");
    }

    function handleCreateNew() {
        setOpenVolumeId(null);
        setMode("create");
    }

    return (
        <div>
            <header className="app-header">
                <h1>Welcome Back!</h1>
                <div className="header-controls">
                    <button type="button" onClick={handleCreateNew} className="primary">Create</button>
                    <Recording compact />
                </div>
            </header>

            <div className="app-layout">
                <aside className="app-left">
                    <AllVolumes onOpen={handleOpen} onEdit={handleEdit} />
                </aside>

                <main className="app-main">
                    {mode === "view" && openVolumeId ? (
                        <VolumeDetail id={openVolumeId} />
                    ) : null}

                    {mode === "edit" ? (
                        <div style={{ marginTop: 12 }}>
                            <VolumeEditor volumeId={openVolumeId ?? undefined} onSaved={(v) => { setOpenVolumeId(v.meta.id); setMode("view"); }} />
                        </div>
                    ) : null}

                    {mode === "create" ? (
                        <div style={{ marginTop: 12 }}>
                            <VolumeEditor onSaved={(v) => { setOpenVolumeId(v.meta.id); setMode("view"); }} />
                        </div>
                    ) : null}

                    {mode === "list" ? (
                        <div className="markdown-placeholder card">
                            <h3>Welcome</h3>
                            <p className="muted">Select a volume on the left to view or edit. The right column shows recent agent actions.</p>
                        </div>
                    ) : null}
                </main>

                <aside className="app-right">
                    <ControlNotifications />
                </aside>
            </div>
        </div>
    );
}

