import type React from "react";
import * as commands from "../../lib/commands";
import type { VolumeIndexEntryFull } from "../../lib/volumes/types";

type Node = VolumeIndexEntryFull & { children: Node[] };

function buildTree(list: VolumeIndexEntryFull[]): Node[] {
    const map = new Map<string, Node>();
    const roots: Node[] = [];

    for (const it of list) {
        map.set(it.id, { ...it, children: [] });
    }

    for (const it of list) {
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

    return roots;
}

export default function VolumeTree({ list, onRefresh, onOpen }: { list: VolumeIndexEntryFull[]; onRefresh: () => void; onOpen?: (id: string) => void }) {
    const roots = buildTree(list || []);

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
        return (
            <li key={n.id} className="volume-item" style={{ marginLeft: depth * 12 }}
                draggable
                onDragStart={(e) => { e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setData("text/plain", n.id); console.log(`[dragstart] id=${n.id}`); }}
                onDragEnter={(e) => { e.preventDefault(); e.dataTransfer.dropEffect = "move"; console.log(`[dragenter] target=${n.id}`); }}
                onDragOver={(e) => { allowDrop(e); console.log(`[dragover] target=${n.id}`); }}
                onDragLeave={(e) => { console.log(`[dragleave] target=${n.id}`); }}
                onDrop={(e) => handleDrop(e, n.id)}
            >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <div>
                        <strong style={{ cursor: "pointer" }} onClick={() => onOpen?.(n.id)}>{n.title}</strong>
                        <div className="volume-meta">{n.updated_at}</div>
                    </div>
                    <div className="volume-actions">
                        <button type="button" onClick={async () => { await commands.flattenVolume(n.id); onRefresh(); }}>Flatten</button>
                    </div>
                </div>
                {n.children.length > 0 ? (
                    <ul>
                        {n.children.map((c) => renderNode(c, depth + 1))}
                    </ul>
                ) : null}
            </li>
        );
    }

    return (
        <div className="volumes">
            <h3>Volume Tree</h3>
            <ul className="volume-list" onDragOver={allowDrop} onDrop={(e) => { e.preventDefault(); }}>
                {roots.map((r) => renderNode(r))}
            </ul>
            <div style={{ marginTop: 8 }}><small>Drag an item onto another to nest it. Use "Flatten" to move to top-level.</small></div>
        </div>
    );
}
