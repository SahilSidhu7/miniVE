export type PortMap = { host: number; container: number };
export type EnvStatus = "running" | "stopped" | "broken";
export type EnvView = { name: string; image: string; ports: PortMap[]; status: EnvStatus };

export const RUNTIMES = [
  { label: "Python 3.12", image: "python:3.12" },
  { label: "Python 3.11", image: "python:3.11" },
  { label: "Python 3.10", image: "python:3.10" },
  { label: "Node 22", image: "node:22" },
  { label: "Node 20", image: "node:20" },
  { label: "Node 18", image: "node:18" },
  { label: "Ubuntu 24.04 (blank)", image: "ubuntu:24.04" },
] as const;

export function runtimeLabel(image: string): string {
  return RUNTIMES.find((r) => r.image === image)?.label ?? image;
}
