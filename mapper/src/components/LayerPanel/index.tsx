import { useEditorStore } from '@/state/store';
import { Layer } from '@/types';
import styles from './LayerPanel.module.css';

const layers: { id: Layer; label: string }[] = [
  { id: Layer.Ground, label: 'Ground' },
  { id: Layer.Objects, label: 'Objects' },
  { id: Layer.Overhead, label: 'Overhead' },
  { id: Layer.Collision, label: 'Collision' },
  { id: Layer.Entities, label: 'Entities' },
];

export function LayerPanel() {
  const {
    activeLayer,
    setActiveLayer,
    visibleLayers,
    setLayerVisibility,
    showCollision,
    showEntities,
    toggleCollisionOverlay,
    toggleEntitiesOverlay,
  } = useEditorStore();

  const isLayerVisible = (layer: Layer): boolean => {
    switch (layer) {
      case Layer.Ground:
        return visibleLayers.ground;
      case Layer.Objects:
        return visibleLayers.objects;
      case Layer.Overhead:
        return visibleLayers.overhead;
      case Layer.Collision:
        return showCollision;
      case Layer.Entities:
        return showEntities;
      default:
        return true;
    }
  };

  const toggleVisibility = (layer: Layer) => {
    switch (layer) {
      case Layer.Ground:
        setLayerVisibility('ground', !visibleLayers.ground);
        break;
      case Layer.Objects:
        setLayerVisibility('objects', !visibleLayers.objects);
        break;
      case Layer.Overhead:
        setLayerVisibility('overhead', !visibleLayers.overhead);
        break;
      case Layer.Collision:
        toggleCollisionOverlay();
        break;
      case Layer.Entities:
        toggleEntitiesOverlay();
        break;
    }
  };

  return (
    <div className={styles.panel}>
      <div className={styles.title}>Layers</div>
      <div className={styles.layers}>
        {layers.map((layer) => (
          <div
            key={layer.id}
            className={`${styles.layer} ${activeLayer === layer.id ? styles.active : ''}`}
          >
            <input
              type="checkbox"
              className={styles.checkbox}
              checked={isLayerVisible(layer.id)}
              onChange={() => toggleVisibility(layer.id)}
            />
            <button
              className={styles.layerButton}
              onClick={() => setActiveLayer(layer.id)}
            >
              {layer.label}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
