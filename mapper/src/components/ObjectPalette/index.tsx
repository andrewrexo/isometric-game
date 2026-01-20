import { useRef, useEffect, useState } from 'react';
import { useEditorStore } from '@/state/store';
import { objectLoader } from '@/core/ObjectLoader';
import type { ObjectDefinition } from '@/types';
import styles from './ObjectPalette.module.css';

export function ObjectPalette() {
  const { selectedObjectId, setSelectedObjectId, setActiveTool } = useEditorStore();
  const [objects, setObjects] = useState<ObjectDefinition[]>([]);
  const [filter, setFilter] = useState('');
  const canvasRefs = useRef<Map<number, HTMLCanvasElement>>(new Map());

  // Load objects when component mounts
  useEffect(() => {
    const loadedObjects = objectLoader.getObjectsWithImages();
    setObjects(loadedObjects);
  }, []);

  // Draw object previews
  useEffect(() => {
    for (const obj of objects) {
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
  }, [objects]);

  const filteredObjects = filter
    ? objects.filter((obj) => obj.name.toLowerCase().includes(filter.toLowerCase()))
    : objects;

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
        <div className={styles.title}>Objects</div>
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
              setActiveTool('object');
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
