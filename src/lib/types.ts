export type PortMap = { host: number; container: number };
export type EnvStatus = "running" | "stopped" | "broken";
export type EnvView = { name: string; image: string; ports: PortMap[]; status: EnvStatus };

export type FamilyKind = "distro" | "language";
export type FamilyVersions = { key: string; displayName: string; versions: string[]; kind: FamilyKind };
export type PackagePreset = "none" | "minimal" | "full" | "essentials";

/** A language to install on top of the distro base image. */
export type LangSpec = { key: string; version: string };

/** A per-environment shell script; onStart ones run at every container start. */
export type ScriptEntry = { name: string; content: string; onStart: boolean };
