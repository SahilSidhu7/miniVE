import { invoke } from "@tauri-apps/api/core";
import type { FamilyVersions } from "./types";

let families: FamilyVersions[] = [];
let pinned: string[] = [];

export async function loadCatalog(): Promise<FamilyVersions[]> {
  families = await invoke<FamilyVersions[]>("list_runtime_catalog");
  return families;
}

export async function loadPinned(): Promise<string[]> {
  pinned = await invoke<string[]>("list_pinned_versions");
  return pinned;
}

export function isPinned(image: string): boolean {
  return pinned.includes(image);
}

export async function pinVersion(image: string): Promise<void> {
  await invoke("pin_version", { version: image });
  pinned = [...pinned, image];
}

export async function unpinVersion(image: string): Promise<void> {
  await invoke("unpin_version", { version: image });
  pinned = pinned.filter((v) => v !== image);
}

export function runtimeLabel(image: string): string {
  const [repo, tag] = image.split(":");
  const family = families.find((f) => f.key === repo);
  return family ? `${family.displayName} ${tag}` : image;
}

/** Flat {label, image} list for the wizard dropdown, pinned versions sorted first. */
export function catalogEntries(): { label: string; image: string }[] {
  const entries: { label: string; image: string }[] = [];
  for (const f of families) {
    for (const v of f.versions) {
      entries.push({ label: `${f.displayName} ${v}`, image: `${f.key}:${v}` });
    }
  }
  entries.sort((a, b) => Number(isPinned(b.image)) - Number(isPinned(a.image)));
  return entries;
}
