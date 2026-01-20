import { useEditorStore } from '@/state/store';
import { chunkManager } from '@/core/ChunkManager';
import { history } from '@/core/History';
import styles from './MenuBar.module.css';

export function MenuBar() {
  const {
    showGrid,
    showChunkBounds,
    toggleGrid,
    toggleChunkBounds,
    viewport,
    setViewport,
    getDirtyChunks,
    markAllClean,
  } = useEditorStore();

  const handleSaveAll = async () => {
    const dirtyChunks = getDirtyChunks();
    if (dirtyChunks.length === 0) {
      alert('No changes to save');
      return;
    }

    // Export all dirty chunks
    const exports: { filename: string; content: string }[] = [];
    for (const chunk of dirtyChunks) {
      const json = chunkManager.exportChunkToJSON(chunk.coord);
      if (json) {
        exports.push({
          filename: `chunk_${chunk.coord.cx}_${chunk.coord.cy}.json`,
          content: json,
        });
      }
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
              Save All ({getDirtyChunks().length} modified)
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
