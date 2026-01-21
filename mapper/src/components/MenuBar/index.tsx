import { useRef } from 'react';
import JSZip from 'jszip';
import { useEditorStore } from '@/state/store';
import { chunkManager } from '@/core/ChunkManager';
import { chunkKey } from '@/core/coords';
import { history } from '@/core/History';
import { storage } from '@/core/Storage';
import styles from './MenuBar.module.css';

export function MenuBar() {
  const {
    chunks,
    showGrid,
    showChunkBounds,
    toggleGrid,
    toggleChunkBounds,
    viewport,
    setViewport,
    getDirtyChunks,
    markAllClean,
    setChunks,
    setWorldBounds,
    isConnected,
  } = useEditorStore();

  const importInputRef = useRef<HTMLInputElement>(null);

  // Store the directory handle for reuse
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const directoryHandleRef = useRef<any>(null);

  const handleSaveAll = async () => {
    const dirtyChunks = getDirtyChunks();
    if (dirtyChunks.length === 0) {
      alert('No changes to save');
      return;
    }

    // Export all dirty chunks from the store (not ChunkManager's cache)
    const exports: { filename: string; content: string }[] = [];
    for (const chunk of dirtyChunks) {
      const json = chunkManager.exportChunkDataToJSON(chunk);
      exports.push({
        filename: `chunk_${chunk.coord.cx}_${chunk.coord.cy}.json`,
        content: json,
      });
    }

    // Download as individual files (in a real app, this would write to the server)
    for (const exp of exports) {
      const blob = new Blob([exp.content], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = exp.filename;
      a.click();
      URL.revokeObjectURL(url);
    }

    markAllClean();
    alert(`Saved ${exports.length} chunk(s)`);
  };

  // Export all chunks to a directory (for testing in live game)
  const handleExportToServer = async () => {
    try {
      // Check if File System Access API is supported
      if ('showDirectoryPicker' in window) {
        await exportToDirectory();
      } else {
        // Fallback: download as ZIP
        await exportAsZip();
      }
    } catch (err) {
      if ((err as Error).name === 'AbortError') {
        // User cancelled
        return;
      }
      console.error('Export failed:', err);
      alert(`Export failed: ${(err as Error).message}`);
    }
  };

  // Export using File System Access API (Chrome/Edge)
  const exportToDirectory = async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let dirHandle: any = directoryHandleRef.current;

    if (!dirHandle) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      dirHandle = await (window as any).showDirectoryPicker({
        mode: 'readwrite',
      });
      directoryHandleRef.current = dirHandle;
    }

    // Verify we still have permission
    const permission = await dirHandle.queryPermission({ mode: 'readwrite' });
    if (permission !== 'granted') {
      const requested = await dirHandle.requestPermission({ mode: 'readwrite' });
      if (requested !== 'granted') {
        alert('Permission denied to write to directory');
        return;
      }
    }

    // Export all chunks from the store (not ChunkManager's cache)
    const allChunks = Array.from(chunks.values());
    let exported = 0;

    for (const chunk of allChunks) {
      const json = chunkManager.exportChunkDataToJSON(chunk);
      const filename = `chunk_${chunk.coord.cx}_${chunk.coord.cy}.json`;
      const fileHandle = await dirHandle.getFileHandle(filename, { create: true });
      const writable = await fileHandle.createWritable();
      await writable.write(json);
      await writable.close();
      exported++;
    }

    markAllClean();
    alert(`Exported ${exported} chunk(s) to ${dirHandle.name}/`);
  };

  // Fallback: export as downloadable ZIP file
  const exportAsZip = async () => {
    const allChunks = Array.from(chunks.values());
    if (allChunks.length === 0) {
      alert('No chunks to export');
      return;
    }

    const zip = new JSZip();
    let exported = 0;

    for (const chunk of allChunks) {
      const json = chunkManager.exportChunkDataToJSON(chunk);
      const filename = `chunk_${chunk.coord.cx}_${chunk.coord.cy}.json`;
      zip.file(filename, json);
      exported++;
    }

    // Generate and download ZIP
    const blob = await zip.generateAsync({ type: 'blob' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'map_export.zip';
    a.click();
    URL.revokeObjectURL(url);

    markAllClean();
    alert(`Downloaded map_export.zip with ${exported} chunk(s).\n\nExtract to: rust-server/maps/world_0/`);
  };

  const handleZoomIn = () => {
    setViewport({ zoom: Math.min(4, viewport.zoom * 1.25) });
  };

  const handleZoomOut = () => {
    setViewport({ zoom: Math.max(0.25, viewport.zoom / 1.25) });
  };

  const handleResetView = () => {
    setViewport({ offsetX: 400, offsetY: 200, zoom: 1 });
  };

  const handleResetToServer = async () => {
    const confirmed = window.confirm(
      'This will discard all local changes and reload from the server.\n\nAre you sure?'
    );
    if (!confirmed) return;

    try {
      // Clear IndexedDB storage
      await storage.clearLocal();

      // Clear ChunkManager cache
      chunkManager.clear();

      // Reload chunks from server
      const knownChunks = [
        { cx: 0, cy: 0 },
        { cx: 0, cy: -1 },
        { cx: 1, cy: 0 },
        { cx: -1, cy: 0 },
        { cx: -1, cy: -1 },
        { cx: -2, cy: 0 },
      ];

      for (const coord of knownChunks) {
        try {
          const chunk = await chunkManager.loadChunk(
            `/maps/chunk_${coord.cx}_${coord.cy}.json`,
            coord
          );
          if (chunk) {
            chunkManager.addChunk(chunk);
          }
        } catch {
          // Chunk doesn't exist
        }
      }

      // If no chunks loaded, create a default chunk
      if (chunkManager.getAllChunks().length === 0) {
        chunkManager.createEmptyChunk({ cx: 0, cy: 0 });
      }

      // Update store
      const newChunks = new Map<string, ReturnType<typeof chunkManager.getChunk>>();
      for (const chunk of chunkManager.getAllChunks()) {
        newChunks.set(chunkKey(chunk.coord), chunk);
      }
      setChunks(newChunks as Map<string, NonNullable<ReturnType<typeof chunkManager.getChunk>>>, true);
      setWorldBounds(chunkManager.getBounds());

      // Clear undo history
      history.clear();

      alert('Reset complete. Loaded fresh data from server.');
    } catch (err) {
      console.error('Reset failed:', err);
      alert(`Reset failed: ${(err as Error).message}`);
    }
  };

  const handleClearLocalData = async () => {
    const confirmed = window.confirm(
      'This will clear all locally saved map data.\n\nYour current session will not be affected, but changes will be lost on refresh.\n\nAre you sure?'
    );
    if (!confirmed) return;

    try {
      await storage.clearLocal();
      alert('Local data cleared.');
    } catch (err) {
      console.error('Clear failed:', err);
      alert(`Clear failed: ${(err as Error).message}`);
    }
  };

  const handleExportMap = async () => {
    try {
      const jsonData = await storage.exportMapData();
      const blob = new Blob([jsonData], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `map-export-${new Date().toISOString().split('T')[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Export failed:', err);
      alert(`Export failed: ${(err as Error).message}`);
    }
  };

  const handleImportMap = () => {
    importInputRef.current?.click();
  };

  const handleImportFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const confirmed = window.confirm(
      'This will replace all map data with the imported file.\n\nAre you sure?'
    );
    if (!confirmed) {
      e.target.value = '';
      return;
    }

    try {
      const text = await file.text();
      const count = await storage.importMapData(text);

      // Reload chunks into editor
      const loadedChunks = await storage.loadAllChunks();
      setChunks(loadedChunks, true);

      // Recalculate bounds
      let minCx = Infinity, maxCx = -Infinity;
      let minCy = Infinity, maxCy = -Infinity;
      for (const chunk of loadedChunks.values()) {
        minCx = Math.min(minCx, chunk.coord.cx);
        maxCx = Math.max(maxCx, chunk.coord.cx);
        minCy = Math.min(minCy, chunk.coord.cy);
        maxCy = Math.max(maxCy, chunk.coord.cy);
      }
      setWorldBounds({
        minCx: minCx === Infinity ? 0 : minCx,
        maxCx: maxCx === -Infinity ? 0 : maxCx,
        minCy: minCy === Infinity ? 0 : minCy,
        maxCy: maxCy === -Infinity ? 0 : maxCy,
      });

      alert(`Imported ${count} chunks successfully.`);
    } catch (err) {
      console.error('Import failed:', err);
      alert(`Import failed: ${(err as Error).message}`);
    }

    e.target.value = '';
  };

  const handleSyncToServer = async () => {
    if (!isConnected) {
      alert('Not connected to server. Changes are saved locally.');
      return;
    }

    try {
      const success = await storage.saveAllChunksToServer(chunks);
      if (success) {
        alert(`Synced ${chunks.size} chunks to server.`);
      } else {
        alert('Failed to sync to server.');
      }
    } catch (err) {
      console.error('Sync failed:', err);
      alert(`Sync failed: ${(err as Error).message}`);
    }
  };

  return (
    <div className={styles.menuBar}>
      <input
        ref={importInputRef}
        type="file"
        accept=".json"
        style={{ display: 'none' }}
        onChange={handleImportFile}
      />
      <div className={styles.menu}>
        <div className={styles.menuItem}>
          <span className={styles.menuTitle}>File</span>
          <div className={styles.dropdown}>
            <button className={styles.dropdownItem} onClick={handleSyncToServer}>
              Sync to Server
            </button>
            <button className={styles.dropdownItem} onClick={handleSaveAll}>
              Download Modified ({getDirtyChunks().length})
            </button>
            <button className={styles.dropdownItem} onClick={handleExportToServer}>
              Export to Directory...
            </button>
            <div className={styles.separator} />
            <button className={styles.dropdownItem} onClick={handleExportMap}>
              Export Map (JSON)
            </button>
            <button className={styles.dropdownItem} onClick={handleImportMap}>
              Import Map (JSON)
            </button>
            <div className={styles.separator} />
            <button className={styles.dropdownItem} onClick={handleResetToServer}>
              Reset to Server Data
            </button>
            <button className={styles.dropdownItem} onClick={handleClearLocalData}>
              Clear Local Storage
            </button>
          </div>
        </div>

        <div className={styles.menuItem}>
          <span className={styles.menuTitle}>Edit</span>
          <div className={styles.dropdown}>
            <button
              className={styles.dropdownItem}
              onClick={() => history.undo()}
              disabled={!history.canUndo()}
            >
              Undo {history.getUndoDescription() ? `(${history.getUndoDescription()})` : ''}
            </button>
            <button
              className={styles.dropdownItem}
              onClick={() => history.redo()}
              disabled={!history.canRedo()}
            >
              Redo {history.getRedoDescription() ? `(${history.getRedoDescription()})` : ''}
            </button>
          </div>
        </div>

        <div className={styles.menuItem}>
          <span className={styles.menuTitle}>View</span>
          <div className={styles.dropdown}>
            <button className={styles.dropdownItem} onClick={toggleGrid}>
              {showGrid ? '✓ ' : '  '}Show Grid
            </button>
            <button className={styles.dropdownItem} onClick={toggleChunkBounds}>
              {showChunkBounds ? '✓ ' : '  '}Show Chunk Bounds
            </button>
            <div className={styles.separator} />
            <button className={styles.dropdownItem} onClick={handleZoomIn}>
              Zoom In
            </button>
            <button className={styles.dropdownItem} onClick={handleZoomOut}>
              Zoom Out
            </button>
            <button className={styles.dropdownItem} onClick={handleResetView}>
              Reset View
            </button>
          </div>
        </div>
      </div>

      <div className={styles.status}>
        <span className={styles.statusItem}>Zoom: {Math.round(viewport.zoom * 100)}%</span>
        <span className={styles.statusItem}>
          {getDirtyChunks().length > 0 && `${getDirtyChunks().length} unsaved`}
        </span>
        <span className={`${styles.statusItem} ${styles.connectionStatus} ${isConnected ? styles.connected : styles.disconnected}`}>
          {isConnected ? 'Connected' : 'Offline'}
        </span>
      </div>
    </div>
  );
}
