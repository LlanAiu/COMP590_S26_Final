// builtin

// external
import { useState, useEffect } from "react";

// internal
import Recording from "./components/audio/recording";
import AllVolumes from "./components/volumes/all-volumes";
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

    function handleCreateNew() {
        setOpenVolumeId(null);
        setMode("create");
    }

    return (
        <div>
            <h1>It's Beautiful</h1>

            <div className="app-layout">
                <div className="app-main">
                    <Recording />
                </div>
                <div className="app-sidebar">
                    <div style={{ marginBottom: 12 }}>
                        <button type="button" onClick={handleCreateNew}>Create Volume</button>
                    </div>
                    <AllVolumes onOpen={handleOpen} />

                    {mode === "view" && openVolumeId ? (
                        <div className="sidebar-section">
                            <VolumeDetail id={openVolumeId} />
                            <div className="sidebar-actions">
                                <button type="button" onClick={() => setMode("edit")}>Edit</button>
                                <button type="button" onClick={() => { setOpenVolumeId(null); setMode("list"); }} className="btn-close">Close</button>
                            </div>
                        </div>
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
                </div>
            </div>
        </div>
    );
}

