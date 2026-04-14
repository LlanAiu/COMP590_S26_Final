import React from "react";

type Props = {
    page: "volumes" | "settings";
    setPage: (p: "volumes" | "settings") => void;
};

export default function Sidebar({ page, setPage }: Props) {
    return (
        <div style={{ width: 56, display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 8, borderRight: "1px solid var(--border)" }}>
            <button aria-label="volumes" title="Volumes" onClick={() => setPage("volumes")} style={{ background: "none", border: "none", padding: 8, cursor: "pointer", fontSize: 20, opacity: page === "volumes" ? 1 : 0.6 }}>
                📚
            </button>
            <div style={{ height: 8 }} />
            <button aria-label="settings" title="Settings" onClick={() => setPage("settings")} style={{ background: "none", border: "none", padding: 8, cursor: "pointer", fontSize: 20, opacity: page === "settings" ? 1 : 0.6 }}>
                ⚙️
            </button>
        </div>
    );
}
