import { useRef, useEffect, useState } from 'react';
import { useEditorStore } from '@/state/store';
import { objectLoader } from '@/core/ObjectLoader';
import type { ObjectDefinition } from '@/types';
import styles from './ObjectPalette.module.css';

type Category = 'objects' | 'walls';
type WallTool = 'wallDown' | 'wallRight';

export function ObjectPalette() {
  const { selectedObjectId, setSelectedObjectId, setActiveTool, activeTool } = useEditorStore();
  const [objects, setObjects] = useState<ObjectDefinition[]>([]);
  const [walls, setWalls] = useState<ObjectDefinition[]>([]);
  const [category, setCategory] = useState<Category>('objects');
  const [filter, setFilter] = useState('');
  const [lastWallTool, setLastWallTool] = useState<WallTool>('wallDown');
  const canvasRefs = useRef<Map<number, HTMLCanvasElement>>(new Map());

  // Load objects and walls when component mounts
  useEffect(() => {
    const loadedObjects = objectLoader.getObjectsWithImages();
    const loadedWalls = objectLoader.getWallsWithImages();
    setObjects(loadedObjects);
    setWalls(loadedWalls);
  }, []);

  // Track when wall tools are used so we remember the last one
  useEffect(() => {
    if (activeTool === 'wallDown' || activeTool === 'wallRight') {
      setLastWallTool(activeTool as WallTool);
    }
  }, [activeTool]);

  // Get current items based on category
  const currentItems = category === 'objects' ? objects : walls;

  // Draw object previews
  useEffect(() => {
    for (const obj of currentItems) {
      const canvas = canvasRefs.current.get(obj.id);
      if (!canvas || !obj.image) continue;

      const ctx = canvas.getContext('2d');
      if (!ctx) continue;

      // Scale to fit in preview box while maintaining aspect ratio
      const maxSize = 64;
      const scale = Math.min(maxSize / obj.width, maxSize / obj.height, 1);
      const drawWidth = obj.width * scale;
      const drawHeight = obj.height * scale;

      canvas.width = maxSize;
      canvas.height = maxSize;

      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, maxSize, maxSize);
      ctx.drawImage(
        obj.image,
        (maxSize - drawWidth) / 2,
        maxSize - drawHeight,
        drawWidth,
        drawHeight
      );
    }
  }, [currentItems]);

  const filteredObjects = filter
    ? currentItems.filter((obj) => obj.name.toLowerCase().includes(filter.toLowerCase()))
    : currentItems;

  const setCanvasRef = (id: number, el: HTMLCanvasElement | null) => {
    if (el) {
      canvasRefs.current.set(id, el);
    } else {
      canvasRefs.current.delete(id);
    }
  };

  return (
    <div className={styles.palette}>
      <div className={styles.header}>
        <div className={styles.title}>Objects & Walls</div>
      </div>
      <div className={styles.tabs}>
        <button
          className={`${styles.tab} ${category === 'objects' ? styles.activeTab : ''}`}
          onClick={() => setCategory('objects')}
        >
          Objects ({objects.length})
        </button>
        <button
          className={`${styles.tab} ${category === 'walls' ? styles.activeTab : ''}`}
          onClick={() => setCategory('walls')}
        >
          Walls ({walls.length})
        </button>
      </div>
      <input
        type="text"
        className={styles.search}
        placeholder="Search objects..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <div className={styles.info}>
        {selectedObjectId ? `Selected: ${objects.find((o) => o.id === selectedObjectId)?.name || selectedObjectId}` : 'None selected'}
      </div>
      <div className={styles.grid}>
        {filteredObjects.map((obj) => (
          <button
            key={obj.id}
            className={`${styles.item} ${selectedObjectId === obj.id ? styles.selected : ''}`}
            onClick={() => {
              setSelectedObjectId(obj.id);
              // Use wall tool when selecting from walls tab, object tool otherwise
              setActiveTool(category === 'walls' ? lastWallTool : 'object');
            }}
            title={`${obj.name} (${obj.width}x${obj.height})`}
          >
            <canvas
              ref={(el) => setCanvasRef(obj.id, el)}
              className={styles.preview}
              width={64}
              height={64}
            />
            <span className={styles.name}>{obj.name}</span>
          </button>
        ))}
        {filteredObjects.length === 0 && (
          <div className={styles.empty}>No objects found</div>
        )}
      </div>
    </div>
  );
}
