import { useRef, useEffect, useState, useCallback } from 'react';
import { useEditorStore } from '@/state/store';
import type { Tileset } from '@/types';
import styles from './TilePalette.module.css';

export function TilePalette() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const { tilesets, selectedTileId, setSelectedTileId, setActiveTool, setActiveLayer } = useEditorStore();
  const [activeTileset, setActiveTileset] = useState<Tileset | null>(null);
  const [hoveredTile, setHoveredTile] = useState<number | null>(null);
  const [zoom, setZoom] = useState(1);
  const [containerWidth, setContainerWidth] = useState(200);
  const [displayCols, setDisplayCols] = useState(1);

  // Set first tileset as active when tilesets load
  useEffect(() => {
    if (tilesets.length > 0 && !activeTileset) {
      setActiveTileset(tilesets[0]);
    }
  }, [tilesets, activeTileset]);

  // Track container width
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });

    observer.observe(container);
    setContainerWidth(container.clientWidth);

    return () => observer.disconnect();
  }, []);

  // Calculate display columns based on container width
  useEffect(() => {
    if (!activeTileset) return;
    const tileDisplayWidth = activeTileset.tileWidth * zoom;
    const cols = Math.max(1, Math.floor(containerWidth / tileDisplayWidth));
    setDisplayCols(cols);
  }, [activeTileset, containerWidth, zoom]);

  // Render tileset preview with dynamic columns
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !activeTileset || !activeTileset.imageElement) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const srcCols = activeTileset.columns;
    const tileW = activeTileset.tileWidth;
    const tileH = activeTileset.tileHeight;
    const tileCount = activeTileset.tileCount;

    // Calculate rows needed for display columns
    const displayRows = Math.ceil(tileCount / displayCols);

    // Set canvas size to fit rearranged tiles
    canvas.width = displayCols * tileW * zoom;
    canvas.height = displayRows * tileH * zoom;

    ctx.imageSmoothingEnabled = false;

    // Draw each tile individually in the new grid layout
    for (let i = 0; i < tileCount; i++) {
      // Source position in original tileset
      const srcCol = i % srcCols;
      const srcRow = Math.floor(i / srcCols);
      const srcX = srcCol * tileW;
      const srcY = srcRow * tileH;

      // Destination position in display grid
      const dstCol = i % displayCols;
      const dstRow = Math.floor(i / displayCols);
      const dstX = dstCol * tileW * zoom;
      const dstY = dstRow * tileH * zoom;

      ctx.drawImage(
        activeTileset.imageElement,
        srcX, srcY, tileW, tileH,
        dstX, dstY, tileW * zoom, tileH * zoom
      );
    }

    // Highlight selected tile
    const selectedLocalId = selectedTileId - activeTileset.firstGid;
    if (selectedLocalId >= 0 && selectedLocalId < tileCount) {
      const col = selectedLocalId % displayCols;
      const row = Math.floor(selectedLocalId / displayCols);

      ctx.strokeStyle = '#5c7cfa';
      ctx.lineWidth = 2;
      ctx.strokeRect(col * tileW * zoom + 1, row * tileH * zoom + 1, tileW * zoom - 2, tileH * zoom - 2);
    }

    // Highlight hovered tile
    if (hoveredTile !== null && hoveredTile < tileCount) {
      const col = hoveredTile % displayCols;
      const row = Math.floor(hoveredTile / displayCols);

      ctx.strokeStyle = '#ffd93d';
      ctx.lineWidth = 1;
      ctx.strokeRect(col * tileW * zoom + 1, row * tileH * zoom + 1, tileW * zoom - 2, tileH * zoom - 2);
    }
  }, [activeTileset, selectedTileId, hoveredTile, zoom, displayCols]);

  const getTileFromEvent = useCallback((e: React.MouseEvent): number | null => {
    if (!activeTileset) return null;

    const canvas = canvasRef.current;
    if (!canvas) return null;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const tileW = activeTileset.tileWidth * zoom;
    const tileH = activeTileset.tileHeight * zoom;

    const col = Math.floor(x / tileW);
    const row = Math.floor(y / tileH);

    if (col < 0 || col >= displayCols) return null;

    const localId = row * displayCols + col;

    if (localId >= 0 && localId < activeTileset.tileCount) {
      return localId;
    }
    return null;
  }, [activeTileset, zoom, displayCols]);

  const handleCanvasClick = (e: React.MouseEvent) => {
    const localId = getTileFromEvent(e);
    if (localId !== null && activeTileset) {
      setSelectedTileId(activeTileset.firstGid + localId);
      setActiveTool('paint');
      setActiveLayer('ground');
    }
  };

  const handleCanvasMove = (e: React.MouseEvent) => {
    const localId = getTileFromEvent(e);
    setHoveredTile(localId);
  };

  const zoomIn = () => setZoom((prev) => Math.min(4, prev + 0.5));
  const zoomOut = () => setZoom((prev) => Math.max(0.5, prev - 0.5));

  return (
    <div className={styles.palette}>
      <div className={styles.header}>
        <div className={styles.title}>Tiles</div>
        <div className={styles.zoomControls}>
          <button className={styles.zoomButton} onClick={zoomOut} title="Zoom Out">
            -
          </button>
          <span className={styles.zoomLevel}>{Math.round(zoom * 100)}%</span>
          <button className={styles.zoomButton} onClick={zoomIn} title="Zoom In">
            +
          </button>
        </div>
        {tilesets.length > 1 && (
          <select
            className={styles.select}
            value={activeTileset?.name || ''}
            onChange={(e) => {
              const ts = tilesets.find((t) => t.name === e.target.value);
              setActiveTileset(ts || null);
            }}
          >
            {tilesets.map((ts) => (
              <option key={ts.name} value={ts.name}>
                {ts.name}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className={styles.info}>
        Selected: {selectedTileId}
        {hoveredTile !== null && activeTileset && (
          <span> | Hover: {activeTileset.firstGid + hoveredTile}</span>
        )}
      </div>
      <div ref={containerRef} className={styles.canvasContainer}>
        <canvas
          ref={canvasRef}
          className={styles.canvas}
          onClick={handleCanvasClick}
          onMouseMove={handleCanvasMove}
          onMouseLeave={() => setHoveredTile(null)}
        />
      </div>
    </div>
  );
}
