// builtin

// external
import { invoke } from "@tauri-apps/api/core";

// internal
import type { CreateVolumeRequest, EditVolumeRequest, Volume, VolumeIndexEntry } from "./volumes/types";


export async function startAudioRecording(): Promise<void> {
    return invoke("start_audio_recording");
}

export async function stopAudioRecording(): Promise<void> {
    return invoke("stop_audio_recording");
}

export async function listVolumes(): Promise<VolumeIndexEntry[]> {
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

