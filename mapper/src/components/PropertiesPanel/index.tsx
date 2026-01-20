import { useState, useEffect } from 'react';
import { useEditorStore } from '@/state/store';
import type { EntitySpawn, ChunkCoord } from '@/types';
import styles from './PropertiesPanel.module.css';

interface SelectedEntity {
  spawn: EntitySpawn;
  chunkCoord: ChunkCoord;
}

export function PropertiesPanel() {
  const { chunks, updateEntity, removeEntity } = useEditorStore();
  const [selectedEntity, setSelectedEntity] = useState<SelectedEntity | null>(null);

  // Find selected entity from all chunks (this would be improved with proper selection state)
  // For now, show a placeholder
  useEffect(() => {
    // This would be connected to actual entity selection
    // For demo, we'll just show the first entity found
    for (const chunk of chunks.values()) {
      if (chunk.entities.length > 0) {
        setSelectedEntity({
          spawn: chunk.entities[0],
          chunkCoord: chunk.coord,
        });
        return;
      }
    }
    setSelectedEntity(null);
  }, [chunks]);

  if (!selectedEntity) {
    return (
      <div className={styles.panel}>
        <div className={styles.title}>Properties</div>
        <div className={styles.empty}>Select an entity to view properties</div>
      </div>
    );
  }

  const { spawn, chunkCoord } = selectedEntity;

  const handleChange = (field: keyof EntitySpawn, value: string | number | boolean) => {
    updateEntity(chunkCoord, spawn.id, { [field]: value });
  };

  const handleDelete = () => {
    if (confirm(`Delete entity "${spawn.name}"?`)) {
      removeEntity(chunkCoord, spawn.id);
      setSelectedEntity(null);
    }
  };

  return (
    <div className={styles.panel}>
      <div className={styles.title}>Properties</div>
      <div className={styles.content}>
        <div className={styles.field}>
          <label className={styles.label}>Entity ID</label>
          <input
            type="text"
            className={styles.input}
            value={spawn.entityId}
            onChange={(e) => handleChange('entityId', e.target.value)}
          />
        </div>

        <div className={styles.field}>
          <label className={styles.label}>Name</label>
          <input
            type="text"
            className={styles.input}
            value={spawn.name}
            onChange={(e) => handleChange('name', e.target.value)}
          />
        </div>

        <div className={styles.field}>
          <label className={styles.label}>Level</label>
          <input
            type="number"
            className={styles.input}
            value={spawn.level}
            min={1}
            onChange={(e) => handleChange('level', parseInt(e.target.value) || 1)}
          />
        </div>

        <div className={styles.field}>
          <label className={styles.label}>Unique ID</label>
          <input
            type="text"
            className={styles.input}
            value={spawn.uniqueId || ''}
            placeholder="Optional"
            onChange={(e) => handleChange('uniqueId', e.target.value)}
          />
        </div>

        <div className={styles.field}>
          <label className={styles.label}>Facing</label>
          <select
            className={styles.select}
            value={spawn.facing || ''}
            onChange={(e) => handleChange('facing', e.target.value)}
          >
            <option value="">Default</option>
            <option value="north">North</option>
            <option value="south">South</option>
            <option value="east">East</option>
            <option value="west">West</option>
          </select>
        </div>

        <div className={styles.field}>
          <label className={styles.checkboxLabel}>
            <input
              type="checkbox"
              checked={spawn.respawn ?? false}
              onChange={(e) => handleChange('respawn', e.target.checked)}
            />
            Respawn
          </label>
        </div>

        <div className={styles.info}>
          Position: {spawn.x}, {spawn.y}
          <br />
          Chunk: {chunkCoord.cx}, {chunkCoord.cy}
        </div>

        <div className={styles.actions}>
          <button className={styles.deleteButton} onClick={handleDelete}>
            Delete Entity
          </button>
        </div>
      </div>
    </div>
  );
}
