import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";

interface ProgressEvent {
  phase: string;
  currentFile: string | null;
  scanned: number;
  matched: number;
  moved: number;
  skippedDuplicates: number;
  errors: number;
  percent: number;
}

export default function App() {
  const [sourcePath, setSourcePath] = useState("");
  const [destPath, setDestPath] = useState("");
  const [suffixInput, setSuffixInput] = useState("");
  const [dryRun, setDryRun] = useState(false);
  const [verbose, setVerbose] = useState(false);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<ProgressEvent>({
    phase: "idle",
    currentFile: null,
    scanned: 0,
    matched: 0,
    moved: 0,
    skippedDuplicates: 0,
    errors: 0,
    percent: 0,
  });
  const [logLines, setLogLines] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const logEndRef = useRef<HTMLDivElement>(null);

  const addLog = useCallback((line: string) => {
    setLogLines((prev) => [...prev.slice(-500), line]);
  }, []);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logLines]);

  useEffect(() => {
    const unlisten = listen<ProgressEvent>("progress", (event) => {
      setProgress(event.payload);
      if (event.payload.phase === "done") {
        setRunning(false);
        addLog(
          `Done. Moved: ${event.payload.moved}, Duplicates skipped: ${event.payload.skippedDuplicates}, Errors: ${event.payload.errors}`
        );
      }
      if (event.payload.currentFile && verbose) {
        addLog(event.payload.currentFile);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [verbose, addLog]);

  const pickSource = async () => {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      recursive: true,
      title: "Select Source Folder",
    });
    if (selected && typeof selected === "string") {
      setSourcePath(selected);
      addLog(`Source: ${selected}`);
    }
  };

  const pickDest = async () => {
    setError(null);
    const selected = await open({
      directory: true,
      multiple: false,
      recursive: true,
      title: "Select Destination Folder",
    });
    if (selected && typeof selected === "string") {
      setDestPath(selected);
      addLog(`Destination: ${selected}`);
    }
  };

  const start = async () => {
    setError(null);
    setLogLines((prev) => [...prev, "Starting…"]);
    setRunning(true);
    try {
      await invoke("start_move", {
        source: sourcePath,
        dest: destPath,
        suffixInput: suffixInput.trim(),
        dryRun: dryRun,
        verbose: verbose,
      });
    } catch (e) {
      setError(String(e));
      setRunning(false);
      addLog(`Error: ${e}`);
    }
  };

  const cancel = async () => {
    try {
      await invoke("cancel_move");
      addLog("Cancel requested.");
    } catch (e) {
      addLog(`Cancel error: ${e}`);
    }
  };

  const [dragOver, setDragOver] = useState(false);
  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const items = e.dataTransfer?.items;
      if (!items || items.length === 0) return;
      const item = items[0];
      if (item.kind !== "file") return;
      const file = item.getAsFile();
      if (file) {
        // Tauri/webview may expose path on the File object for native drops
        const path = (file as File & { path?: string }).path;
        if (path) {
          setSourcePath(path);
          addLog(`Source (from drop): ${path}`);
        }
      }
    },
    [addLog]
  );
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDragOver(true);
  }, []);
  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
  }, []);

  return (
    <div>
      <h1>FrameMover</h1>
      <p style={{ margin: "0 0 1rem 0", color: "var(--text-muted)", fontSize: "0.875rem" }}>
        Move image files whose filename ends with the given suffix numbers. Preserves folder structure and skips duplicates by content hash.
      </p>

      <div className="section">
        <label>Source folder (drag-and-drop or pick)</label>
        <div
          className={`drop-zone ${dragOver ? "drag-over" : ""}`}
          onDrop={handleDrop}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
        >
          <div className="row" style={{ justifyContent: "center" }}>
            <span className="path-display filled" style={{ maxWidth: "100%" }}>
              {sourcePath || "No folder selected"}
            </span>
            <button type="button" className="btn-secondary" onClick={pickSource}>
              Browse…
            </button>
          </div>
        </div>
      </div>

      <div className="section">
        <label>Destination folder</label>
        <div className="row">
          <span className={`path-display ${destPath ? "filled" : ""}`}>
            {destPath || "No folder selected"}
          </span>
          <button type="button" className="btn-secondary" onClick={pickDest}>
            Browse…
          </button>
        </div>
      </div>

      <div className="section">
        <label>Suffix numbers (comma, space, or newline separated)</label>
        <textarea
          placeholder="e.g. 7612, 7608, 7605 7602 7595"
          value={suffixInput}
          onChange={(e) => setSuffixInput(e.target.value)}
        />
      </div>

      <div className="section toggles">
        <label className="toggle-wrap">
          <input
            type="checkbox"
            checked={dryRun}
            onChange={(e) => setDryRun(e.target.checked)}
            disabled={running}
          />
          Dry run (simulate)
        </label>
        <label className="toggle-wrap">
          <input
            type="checkbox"
            checked={verbose}
            onChange={(e) => setVerbose(e.target.checked)}
            disabled={running}
          />
          Verbose log
        </label>
      </div>

      <div className="actions">
        <button
          type="button"
          className="btn-primary"
          onClick={start}
          disabled={running || !sourcePath || !destPath || !suffixInput.trim()}
        >
          Start
        </button>
        <button
          type="button"
          className="btn-secondary"
          onClick={cancel}
          disabled={!running}
        >
          Cancel
        </button>
      </div>

      {error && <p className="error-msg">{error}</p>}

      <div className="progress-section">
        <label>Progress</label>
        <div className="progress-bar-wrap">
          <div
            className="progress-bar"
            style={{ width: `${progress.percent}%` }}
          />
        </div>
        <div className="progress-stats">
          <span><strong>Scanned / Matched:</strong> {progress.scanned} / {progress.matched}</span>
          <span><strong>Moved:</strong> {progress.moved}</span>
          <span><strong>Skipped (duplicates):</strong> {progress.skippedDuplicates}</span>
          <span><strong>Errors:</strong> {progress.errors}</span>
        </div>
        {progress.currentFile && (
          <div className="current-file">{progress.currentFile}</div>
        )}
      </div>

      <div className="section">
        <label>Log</label>
        <div className="log-view">
          {logLines.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
          <div ref={logEndRef} />
        </div>
      </div>
    </div>
  );
}
