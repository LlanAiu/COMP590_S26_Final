// builtin

// external

// internal


export type VolumeIndexEntry = { id: string; title: string; updated_at: string; snippet?: string };

export type VolumeMeta = {
    id: string;
    title: string;
    description?: string;
    created_at: string;
    updated_at: string;
    tags: string[];
    version: number;
};

export type Volume = { meta: VolumeMeta; content: string; attachments: string[] };

export type CreateVolumeRequest = { title: string; content: string; description?: string; tags?: string[] };
export type EditVolumeRequest = CreateVolumeRequest;
