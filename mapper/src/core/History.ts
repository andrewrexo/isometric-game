export interface HistoryAction {
  type: string;
  description: string;
  undo: () => void;
  redo: () => void;
}

export class History {
  private undoStack: HistoryAction[] = [];
  private redoStack: HistoryAction[] = [];
  private maxDepth: number;
  private currentGroup: HistoryAction[] | null = null;
  private groupDescription: string = '';

  constructor(maxDepth: number = 100) {
    this.maxDepth = maxDepth;
  }

  // Push a single action
  push(action: HistoryAction): void {
    if (this.currentGroup) {
      this.currentGroup.push(action);
    } else {
      this.undoStack.push(action);
      this.redoStack = []; // Clear redo stack on new action

      // Trim if over max depth
      while (this.undoStack.length > this.maxDepth) {
        this.undoStack.shift();
      }
    }
  }

  // Start grouping actions (e.g., for paint strokes)
  beginGroup(description: string): void {
    if (this.currentGroup) {
      this.endGroup(); // End previous group if exists
    }
    this.currentGroup = [];
    this.groupDescription = description;
  }

  // End grouping and push as single undoable action
  endGroup(): void {
    if (!this.currentGroup || this.currentGroup.length === 0) {
      this.currentGroup = null;
      return;
    }

    const actions = this.currentGroup;
    const description = this.groupDescription;

    const groupAction: HistoryAction = {
      type: 'group',
      description,
      undo: () => {
        // Undo in reverse order
        for (let i = actions.length - 1; i >= 0; i--) {
          actions[i].undo();
        }
      },
      redo: () => {
        // Redo in original order
        for (const action of actions) {
          action.redo();
        }
      },
    };

    this.currentGroup = null;
    this.push(groupAction);
  }

  // Cancel current group without saving
  cancelGroup(): void {
    if (this.currentGroup) {
      // Undo all actions in current group
      for (let i = this.currentGroup.length - 1; i >= 0; i--) {
        this.currentGroup[i].undo();
      }
    }
    this.currentGroup = null;
  }

  undo(): boolean {
    if (this.undoStack.length === 0) return false;

    const action = this.undoStack.pop()!;
    action.undo();
    this.redoStack.push(action);
    return true;
  }

  redo(): boolean {
    if (this.redoStack.length === 0) return false;

    const action = this.redoStack.pop()!;
    action.redo();
    this.undoStack.push(action);
    return true;
  }

  canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
    this.currentGroup = null;
  }

  getUndoDescription(): string | null {
    if (this.undoStack.length === 0) return null;
    return this.undoStack[this.undoStack.length - 1].description;
  }

  getRedoDescription(): string | null {
    if (this.redoStack.length === 0) return null;
    return this.redoStack[this.redoStack.length - 1].description;
  }

  getUndoCount(): number {
    return this.undoStack.length;
  }

  getRedoCount(): number {
    return this.redoStack.length;
  }
}

// Singleton instance
export const history = new History();
