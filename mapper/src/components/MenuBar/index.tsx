import { useRef } from 'react';
import JSZip from 'jszip';
import { useEditorStore } from '@/state/store';
import { chunkManager } from '@/core/ChunkManager';
import { history } from '@/core/History';
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
  } = useEditorStore();

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

  return (
    <div className={styles.menuBar}>
      <div className={styles.menu}>
        <div className={styles.menuItem}>
          <span className={styles.menuTitle}>File</span>
          <div className={styles.dropdown}>
            <button className={styles.dropdownItem} onClick={handleSaveAll}>
              Save Modified ({getDirtyChunks().length})
            </button>
            <button className={styles.dropdownItem} onClick={handleExportToServer}>
              Export All to Server...
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
      </div>
    </div>
  );
}
