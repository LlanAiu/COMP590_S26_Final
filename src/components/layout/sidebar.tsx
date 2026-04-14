import React from "react";
import { BookOpen, Settings as SettingsIcon } from "lucide-react";

type Props = {
    page: "volumes" | "settings";
    setPage: (p: "volumes" | "settings") => void;
};

export default function Sidebar({ page, setPage }: Props) {
    return (
        <div style={{ width: 36, zIndex: 20, display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 12, borderRight: "1px solid var(--border)" }}>
            <button aria-label="volumes" title="Volumes" onClick={() => setPage("volumes")} style={{ background: "none", border: "none", padding: 6, cursor: "pointer", opacity: page === "volumes" ? 1 : 0.6 }}>
                <BookOpen size={20} />
            </button>
            <div style={{ height: 8 }} />
            <button aria-label="settings" title="Settings" onClick={() => setPage("settings")} style={{ background: "none", border: "none", padding: 6, cursor: "pointer", opacity: page === "settings" ? 1 : 0.6 }}>
                <SettingsIcon size={20} />
            </button>
        </div>
    );
}
