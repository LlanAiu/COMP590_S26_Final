import type React from "react";
import * as commands from "../../lib/commands";
import type { VolumeIndexEntryFull } from "../../lib/volumes/types";

type Node = VolumeIndexEntryFull & { children: Node[] };

function buildTree(list: VolumeIndexEntryFull[]): Node[] {
    const map = new Map<string, Node>();
    const roots: Node[] = [];

    // Sort input list by title (case-insensitive) for deterministic ordering
    const sorted = [...list].sort((a, b) => (a.title || '').localeCompare(b.title || '', undefined, { sensitivity: 'base' }));

    for (const it of sorted) {
        map.set(it.id, { ...it, children: [] });
    }

    for (const it of sorted) {
        const node = map.get(it.id);
        if (it.parent) {
            const p = map.get(it.parent);
            if (p && node) {
                p.children.push(node);
                continue;
            }
        }
        if (node) {
            roots.push(node);
        }
    }

    // Sort children of each node recursively
    function sortChildren(n: Node) {
        n.children.sort((x, y) => (x.title || '').localeCompare(y.title || '', undefined, { sensitivity: 'base' }));
        for (const c of n.children) sortChildren(c);
    }

    for (const node of map.values()) {
        if (node.children && node.children.length) sortChildren(node);
    }

    // Ensure roots are sorted as well
    roots.sort((x, y) => (x.title || '').localeCompare(y.title || '', undefined, { sensitivity: 'base' }));

    return roots;
}

import { useState } from "react";

export default function VolumeTree({ list, onRefresh, onOpen, onEdit, mode }: { list: VolumeIndexEntryFull[]; onRefresh: () => void; onOpen?: (id: string) => void; onEdit?: (id: string) => void; mode?: "list" | "view" | "edit" | "create" }) {
    const roots = buildTree(list || []);
    const [expanded, setExpanded] = useState<Record<string, boolean>>({});
    const [selected, setSelected] = useState<string | null>(null);

    function toggleExpand(id: string) {
        setExpanded((s) => ({ ...s, [id]: !s[id] }));
    }

    async function handleDrop(e: React.DragEvent, targetId: string) {
        e.preventDefault();
        e.stopPropagation();
        const childId = e.dataTransfer.getData("text/plain");
        console.log(`[drop] target=${targetId} data='${childId}'`);
        if (!childId) {
            console.log("[drop] no child id in dataTransfer");
            return;
        }
        if (childId === targetId) {
            console.log("[drop] child === target, ignoring");
            return;
        }
        try {
            await commands.nestVolume(targetId, childId);
            console.log(`[drop] nestVolume succeeded for parent=${targetId} child=${childId}`);
        } catch (err) {
            console.error("nest failed", err);
        }
        onRefresh();
    }

    function allowDrop(e: React.DragEvent) {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
    }

    function renderNode(n: Node, depth = 0) {
        const isOpen = !!expanded[n.id];
        const isSelected = selected === n.id;
        return (
            <li key={n.id} className={"volume-item" + (isSelected ? ' selected' : '')} style={{ marginLeft: 0, flexDirection: 'column', alignItems: 'stretch' }}
                draggable
                onDragStart={(e) => { e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setData("text/plain", n.id); console.log(`[dragstart] id=${n.id}`); }}
                onDragEnter={(e) => { e.preventDefault(); e.dataTransfer.dropEffect = "move"; console.log(`[dragenter] target=${n.id}`); }}
                onDragOver={(e) => { allowDrop(e); console.log(`[dragover] target=${n.id}`); }}
                onDragLeave={(e) => { console.log(`[dragleave] target=${n.id}`); }}
                onDrop={(e) => handleDrop(e, n.id)}
            >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                        {n.children.length > 0 ? (
                            <button type="button" className="expand-toggle" onClick={(e) => { e.stopPropagation(); toggleExpand(n.id); }}>{isOpen ? '▾' : '▸'}</button>
                        ) : <span style={{ width: 18 }} />}
                        <div style={{ cursor: 'pointer' }} onClick={(e) => { e.stopPropagation(); setSelected(n.id); onOpen?.(n.id); }}>
                            <strong>{n.title}</strong>
                        </div>
                    </div>

                </div>

                {n.children.length > 0 && isOpen ? (
                    <ul className="volume-children">
                        {n.children.map((c) => renderNode(c, depth + 1))}
                    </ul>
                ) : null}
            </li>
        );
    }

    return (
        <div className="volumes">
            <ul className="volume-list" onDragOver={allowDrop} onDrop={(e) => { e.preventDefault(); }}>
                {roots.map((r) => renderNode(r))}
            </ul>

            {selected && mode !== "list" ? (
                <div className="sidebar-section selected-actions" style={{ marginTop: 12, display: 'flex', justifyContent: 'center' }}>
                    <div style={{ display: 'flex', gap: 8, justifyContent: 'center' }}>
                        <button type="button" onClick={() => { setSelected(null); onEdit?.(selected!); }}>Edit</button>
                        <button type="button" onClick={async () => { try { await commands.flattenVolume(selected); onRefresh(); } catch (err) { console.error(err); } }}>Flatten</button>
                        <button type="button" onClick={async () => { if (!confirm('Delete this volume?')) return; try { await commands.deleteVolume(selected); setSelected(null); onRefresh(); } catch (err) { console.error(err); } }}>Delete</button>
                        <button type="button" onClick={() => setSelected(null)} className="btn-close">Close</button>
                    </div>
                </div>
            ) : null}
        </div>
    );
}
