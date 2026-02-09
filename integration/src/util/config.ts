import * as fs from "fs";
import * as path from "path";
import type { WwatcherConfig } from "./types.js";

/** Resolve the wwatcher config directory (mirrors src/config.rs config_path()) */
export function getConfigDir(): string {
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  if (process.platform === "win32") {
    return path.join(process.env.APPDATA || path.join(homeDir, "AppData", "Roaming"), "wwatcher");
  }
  return path.join(homeDir, ".config", "wwatcher");
}

export function getConfigPath(): string {
  return path.join(getConfigDir(), "config.json");
}

export function getHistoryPath(): string {
  return path.join(getConfigDir(), "alert_history.jsonl");
}

/** Load wwatcher config.json. Returns default if missing. */
export function loadWwatcherConfig(): WwatcherConfig {
  const configPath = getConfigPath();
  if (!fs.existsSync(configPath)) {
    return {};
  }
  try {
    const raw = fs.readFileSync(configPath, "utf-8");
    return JSON.parse(raw) as WwatcherConfig;
  } catch {
    return {};
  }
}
