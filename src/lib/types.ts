export type PortMap = { host: number; container: number };
export type EnvStatus = "running" | "stopped" | "broken";
export type EnvView = { name: string; image: string; ports: PortMap[]; status: EnvStatus };

export type FamilyVersions = { key: string; displayName: string; versions: string[] };
export type PackagePreset = "none" | "minimal" | "full";
