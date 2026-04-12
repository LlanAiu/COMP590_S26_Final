// builtin

// external
import { invoke } from "@tauri-apps/api/core";

// internal
import type { CreateVolumeRequest, EditVolumeRequest, Volume, VolumeIndexEntryFull } from "./volumes/types";


export async function startAudioRecording(): Promise<void> {
    return invoke("start_audio_recording");
}

export async function stopAudioRecording(): Promise<void> {
    return invoke("stop_audio_recording");
}

export async function listVolumes(): Promise<VolumeIndexEntryFull[]> {
    return invoke("list_volumes");
}

export async function createVolume(req: CreateVolumeRequest): Promise<Volume> {
    return invoke("create_volume", { req });
}

export async function readVolume(id: string): Promise<Volume> {
    return invoke("read_volume", { id });
}

export async function editVolume(id: string, req: EditVolumeRequest): Promise<Volume> {
    return invoke("edit_volume", { id, req });
}

export async function deleteVolume(id: string): Promise<void> {
    return invoke("delete_volume", { id });
}

export async function nestVolume(parentId: string, childId: string): Promise<Volume> {
    return invoke("nest_volume", { parentId: parentId, childId: childId });
}

export async function flattenVolume(id: string): Promise<Volume> {
    return invoke("flatten_volume", { id });
}

export async function mergeVolumes(aId: string, bId: string, req: CreateVolumeRequest): Promise<Volume> {
    return invoke("merge_volumes", { aId, bId, req });
}

export async function splitVolume(id: string, first: CreateVolumeRequest, second: CreateVolumeRequest): Promise<Volume[]> {
    return invoke("split_volume", { id, first, second });
}

export type ControlLogEntry = { timestamp: string; description: string };

export async function getControlLog(): Promise<ControlLogEntry[]> {
    return invoke("get_control_log");
}

export async function clearControlLog(): Promise<void> {
    return invoke("clear_control_log");
}

