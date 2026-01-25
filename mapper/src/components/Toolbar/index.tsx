import { useEditorStore } from '@/state/store';
import { Tool } from '@/types';
import styles from './Toolbar.module.css';

const tools: { id: Tool; label: string; shortcut: string; group?: string; mode?: 'overworld' | 'interior' }[] = [
  { id: Tool.Select, label: 'Select', shortcut: 'V' },
  { id: Tool.Paint, label: 'Paint', shortcut: 'B' },
  { id: Tool.Fill, label: 'Fill', shortcut: 'G' },
  { id: Tool.MagicWand, label: 'Magic Wand', shortcut: 'W' },
  { id: Tool.Eraser, label: 'Eraser', shortcut: 'E' },
  { id: Tool.Collision, label: 'Collision', shortcut: 'C' },
  { id: Tool.Entity, label: 'Entity', shortcut: 'N' },
  { id: Tool.Object, label: 'Object', shortcut: 'O' },
  { id: Tool.Portal, label: 'Portal', shortcut: 'P', mode: 'overworld' },
  { id: Tool.Eyedropper, label: 'Eyedropper', shortcut: 'I' },
  { id: Tool.WallDown, label: 'Wall Down', shortcut: 'D', group: 'walls' },
  { id: Tool.WallRight, label: 'Wall Right', shortcut: 'R', group: 'walls' },
  { id: Tool.SpawnPoint, label: 'Spawn Point', shortcut: 'S', group: 'interior', mode: 'interior' },
  { id: Tool.ExitPortal, label: 'Exit Portal', shortcut: 'X', group: 'interior', mode: 'interior' },
];

export function Toolbar() {
  const { activeTool, setActiveTool, editorMode } = useEditorStore();

  // Filter tools based on editor mode
  const regularTools = tools.filter((t) => !t.group && (!t.mode || t.mode === editorMode));
  const wallTools = tools.filter((t) => t.group === 'walls');
  const interiorTools = tools.filter((t) => t.group === 'interior' && editorMode === 'interior');

  return (
    <div className={styles.toolbar}>
      <div className={styles.title}>Tools</div>
      <div className={styles.tools}>
        {regularTools.map((tool) => (
          <button
            key={tool.id}
            className={`${styles.tool} ${activeTool === tool.id ? styles.active : ''}`}
            onClick={() => setActiveTool(tool.id)}
            title={`${tool.label} (${tool.shortcut})`}
          >
            <span className={styles.label}>{tool.label}</span>
            <span className={styles.shortcut}>{tool.shortcut}</span>
          </button>
        ))}
        <div className={styles.separator} />
        <div className={styles.wallGroup}>
          {wallTools.map((tool) => (
            <button
              key={tool.id}
              className={`${styles.tool} ${activeTool === tool.id ? styles.active : ''}`}
              onClick={() => setActiveTool(tool.id)}
              title={`${tool.label} (${tool.shortcut})`}
            >
              <span className={styles.label}>{tool.label}</span>
              <span className={styles.shortcut}>{tool.shortcut}</span>
            </button>
          ))}
        </div>
        {interiorTools.length > 0 && (
          <>
            <div className={styles.separator} />
            <div className={styles.interiorGroup}>
              {interiorTools.map((tool) => (
                <button
                  key={tool.id}
                  className={`${styles.tool} ${styles.interiorTool} ${activeTool === tool.id ? styles.active : ''}`}
                  onClick={() => setActiveTool(tool.id)}
                  title={`${tool.label} (${tool.shortcut})`}
                >
                  <span className={styles.label}>{tool.label}</span>
                  <span className={styles.shortcut}>{tool.shortcut}</span>
                </button>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
