import { useEditorStore } from '@/state/store';
import { Tool } from '@/types';
import styles from './Toolbar.module.css';

const tools: { id: Tool; label: string; shortcut: string; group?: string }[] = [
  { id: Tool.Select, label: 'Select', shortcut: 'V' },
  { id: Tool.Paint, label: 'Paint', shortcut: 'B' },
  { id: Tool.Fill, label: 'Fill', shortcut: 'G' },
  { id: Tool.MagicWand, label: 'Magic Wand', shortcut: 'W' },
  { id: Tool.Eraser, label: 'Eraser', shortcut: 'E' },
  { id: Tool.Collision, label: 'Collision', shortcut: 'C' },
  { id: Tool.Entity, label: 'Entity', shortcut: 'N' },
  { id: Tool.Object, label: 'Object', shortcut: 'O' },
  { id: Tool.Eyedropper, label: 'Eyedropper', shortcut: 'I' },
  { id: Tool.WallDown, label: 'Wall Down', shortcut: 'D', group: 'walls' },
  { id: Tool.WallRight, label: 'Wall Right', shortcut: 'R', group: 'walls' },
];

export function Toolbar() {
  const { activeTool, setActiveTool } = useEditorStore();

  const regularTools = tools.filter((t) => !t.group);
  const wallTools = tools.filter((t) => t.group === 'walls');

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
      </div>
    </div>
  );
}
