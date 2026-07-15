import { writable } from "svelte/store";

/** A terminal session handed back from a popped-out window, waiting for the
 *  matching env's Workspace to re-adopt it as a tab. */
export type Popin = { env: string; session: number };

export const popins = writable<Popin[]>([]);
