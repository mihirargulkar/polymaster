import * as fs from "fs";
import { watch } from "chokidar";
import type { WhalertAlert } from "../util/types.js";
import type { AlertStore } from "./alert-store.js";

/**
 * Watches the wwatcher alert_history.jsonl file for new lines.
 * Uses chokidar for filesystem events + offset tracking to only read new bytes.
 */
export class FileWatcher {
  private offset: number = 0;
  private watcher: ReturnType<typeof watch> | null = null;

  constructor(
    private filePath: string,
    private store: AlertStore,
    private onNewAlert?: (alert: WhalertAlert) => void
  ) {}

  /** Start watching. Sets offset to current file end so we only get new alerts. */
  start(): void {
    // Set offset to current end of file
    if (fs.existsSync(this.filePath)) {
      const stat = fs.statSync(this.filePath);
      this.offset = stat.size;
    }

    this.watcher = watch(this.filePath, {
      persistent: true,
      awaitWriteFinish: { stabilityThreshold: 100, pollInterval: 50 },
    });

    this.watcher.on("change", () => {
      this.readNewLines();
    });

    // Handle file creation if it doesn't exist yet
    this.watcher.on("add", () => {
      if (this.offset === 0) {
        this.readNewLines();
      }
    });
  }

  stop(): void {
    if (this.watcher) {
      this.watcher.close();
      this.watcher = null;
    }
  }

  private readNewLines(): void {
    if (!fs.existsSync(this.filePath)) return;

    const stat = fs.statSync(this.filePath);
    if (stat.size <= this.offset) return;

    const fd = fs.openSync(this.filePath, "r");
    const buffer = Buffer.alloc(stat.size - this.offset);
    fs.readSync(fd, buffer, 0, buffer.length, this.offset);
    fs.closeSync(fd);

    this.offset = stat.size;

    const newContent = buffer.toString("utf-8");
    for (const line of newContent.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const alert = JSON.parse(trimmed) as WhalertAlert;
        this.store.addAlert(alert);
        this.onNewAlert?.(alert);
      } catch {
        // Skip malformed lines
      }
    }
  }
}
