import { useRef, useEffect, useState } from 'react';
import { useEditorStore } from '@/state/store';
import type { Tileset } from '@/types';
import styles from './TilePalette.module.css';

export function TilePalette() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { tilesets, selectedTileId, setSelectedTileId } = useEditorStore();
  const [activeTileset, setActiveTileset] = useState<Tileset | null>(null);
  const [hoveredTile, setHoveredTile] = useState<number | null>(null);

  // Set first tileset as active when tilesets load
  useEffect(() => {
    if (tilesets.length > 0 && !activeTileset) {
      setActiveTileset(tilesets[0]);
    }
  }, [tilesets, activeTileset]);

  // Render tileset preview
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !activeTileset || !activeTileset.imageElement) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size to fit tileset
    const cols = activeTileset.columns;
    const rows = activeTileset.rows;
    const tileW = activeTileset.tileWidth;
    const tileH = activeTileset.tileHeight;

    canvas.width = cols * tileW;
    canvas.height = rows * tileH;

    // Draw tileset
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(activeTileset.imageElement, 0, 0);

    // Highlight selected tile
    const selectedLocalId = selectedTileId - activeTileset.firstGid;
    if (selectedLocalId >= 0 && selectedLocalId < activeTileset.tileCount) {
      const col = selectedLocalId % cols;
      const row = Math.floor(selectedLocalId / cols);

      ctx.strokeStyle = '#5c7cfa';
      ctx.lineWidth = 2;
      ctx.strokeRect(col * tileW + 1, row * tileH + 1, tileW - 2, tileH - 2);
    }

    // Highlight hovered tile
    if (hoveredTile !== null) {
      const col = hoveredTile % cols;
      const row = Math.floor(hoveredTile / cols);

      ctx.strokeStyle = '#ffd93d';
      ctx.lineWidth = 1;
      ctx.strokeRect(col * tileW + 1, row * tileH + 1, tileW - 2, tileH - 2);
    }
  }, [activeTileset, selectedTileId, hoveredTile]);

  const handleCanvasClick = (e: React.MouseEvent) => {
    if (!activeTileset) return;

    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const y = (e.clientY - rect.top) * scaleY;

    const col = Math.floor(x / activeTileset.tileWidth);
    const row = Math.floor(y / activeTileset.tileHeight);
    const localId = row * activeTileset.columns + col;

    if (localId >= 0 && localId < activeTileset.tileCount) {
      setSelectedTileId(activeTileset.firstGid + localId);
    }
  };

  const handleCanvasMove = (e: React.MouseEvent) => {
    if (!activeTileset) return;

    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const y = (e.clientY - rect.top) * scaleY;

    const col = Math.floor(x / activeTileset.tileWidth);
    const row = Math.floor(y / activeTileset.tileHeight);
    const localId = row * activeTileset.columns + col;

    if (localId >= 0 && localId < activeTileset.tileCount) {
      setHoveredTile(localId);
    } else {
      setHoveredTile(null);
    }
  };

  return (
    <div className={styles.palette}>
      <div className={styles.header}>
        <div className={styles.title}>Tiles</div>
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
      <div className={styles.canvasContainer}>
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
