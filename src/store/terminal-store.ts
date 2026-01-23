import { create } from 'zustand'
import { getFilename } from '@/lib/path-utils'

/** A single terminal instance */
export interface TerminalInstance {
  id: string
  worktreeId: string
  command: string | null
  label: string
}

interface TerminalState {
  // Terminal instances per worktree (worktreeId -> terminals)
  terminals: Record<string, TerminalInstance[]>
  // Active terminal ID per worktree
  activeTerminalIds: Record<string, string>
  // Set of running terminal IDs (have active PTY process)
  runningTerminals: Set<string>
  // Whether terminal panel is expanded (false = collapsed/minimized)
  terminalVisible: boolean
  // Whether terminal panel is open at all (false = completely hidden via X button)
  terminalPanelOpen: boolean
  terminalHeight: number

  setTerminalVisible: (visible: boolean) => void
  setTerminalPanelOpen: (open: boolean) => void
  toggleTerminal: () => void
  setTerminalHeight: (height: number) => void

  // Terminal instance management
  addTerminal: (worktreeId: string, command?: string | null, label?: string) => string
  removeTerminal: (worktreeId: string, terminalId: string) => void
  setActiveTerminal: (worktreeId: string, terminalId: string) => void
  getTerminals: (worktreeId: string) => TerminalInstance[]
  getActiveTerminal: (worktreeId: string) => TerminalInstance | null

  // Running state (terminal has active PTY)
  setTerminalRunning: (terminalId: string, running: boolean) => void
  isTerminalRunning: (terminalId: string) => boolean

  // Start a run command (creates new terminal with command)
  startRun: (worktreeId: string, command: string) => string

  // Close all terminals for a worktree (returns terminal IDs that need to be stopped)
  closeAllTerminals: (worktreeId: string) => string[]
}

function generateTerminalId(): string {
  return crypto.randomUUID()
}

function getDefaultLabel(command: string | null): string {
  if (!command) return 'Shell'
  // Extract first word or command name
  const firstWord = command.split(' ')[0] ?? command
  // Remove path if present (cross-platform)
  const name = getFilename(firstWord)
  return name.length > 20 ? name.slice(0, 17) + '...' : name
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  terminals: {},
  activeTerminalIds: {},
  runningTerminals: new Set(),
  terminalVisible: false,
  terminalPanelOpen: false,
  terminalHeight: 30,

  setTerminalVisible: visible => set({ terminalVisible: visible }),

  setTerminalPanelOpen: open => set({ terminalPanelOpen: open }),

  toggleTerminal: () =>
    set(state => ({
      terminalVisible: !state.terminalVisible,
      // Also open the panel if making visible
      terminalPanelOpen: !state.terminalVisible ? true : state.terminalPanelOpen,
    })),

  setTerminalHeight: height => set({ terminalHeight: height }),

  addTerminal: (worktreeId, command = null, label) => {
    const id = generateTerminalId()
    const terminal: TerminalInstance = {
      id,
      worktreeId,
      command,
      label: label ?? getDefaultLabel(command),
    }

    set(state => {
      const existing = state.terminals[worktreeId] ?? []
      return {
        terminals: {
          ...state.terminals,
          [worktreeId]: [...existing, terminal],
        },
        activeTerminalIds: {
          ...state.activeTerminalIds,
          [worktreeId]: id,
        },
        terminalPanelOpen: true,
        terminalVisible: true,
      }
    })

    return id
  },

  removeTerminal: (worktreeId, terminalId) =>
    set(state => {
      const existing = state.terminals[worktreeId] ?? []
      const filtered = existing.filter(t => t.id !== terminalId)

      // Update running terminals
      const newRunning = new Set(state.runningTerminals)
      newRunning.delete(terminalId)

      // Update active terminal if needed
      const currentActiveId = state.activeTerminalIds[worktreeId] ?? ''
      const newActiveId =
        currentActiveId === terminalId
          ? (filtered[filtered.length - 1]?.id ?? '')
          : currentActiveId

      return {
        terminals: {
          ...state.terminals,
          [worktreeId]: filtered,
        },
        activeTerminalIds: {
          ...state.activeTerminalIds,
          [worktreeId]: newActiveId,
        },
        runningTerminals: newRunning,
      }
    }),

  setActiveTerminal: (worktreeId, terminalId) =>
    set(state => ({
      activeTerminalIds: {
        ...state.activeTerminalIds,
        [worktreeId]: terminalId,
      },
    })),

  getTerminals: worktreeId => get().terminals[worktreeId] ?? [],

  getActiveTerminal: worktreeId => {
    const terminals = get().terminals[worktreeId] ?? []
    const activeId = get().activeTerminalIds[worktreeId]
    return terminals.find(t => t.id === activeId) ?? null
  },

  setTerminalRunning: (terminalId, running) =>
    set(state => {
      const newSet = new Set(state.runningTerminals)
      if (running) {
        newSet.add(terminalId)
      } else {
        newSet.delete(terminalId)
      }
      return { runningTerminals: newSet }
    }),

  isTerminalRunning: terminalId => get().runningTerminals.has(terminalId),

  startRun: (worktreeId, command) => {
    const state = get()
    const terminals = state.terminals[worktreeId] ?? []

    // Check if there's already a running terminal with this command
    const existingTerminal = terminals.find(
      t => t.command === command && state.runningTerminals.has(t.id)
    )

    if (existingTerminal) {
      // Focus the existing terminal instead of creating a new one
      set({
        activeTerminalIds: {
          ...state.activeTerminalIds,
          [worktreeId]: existingTerminal.id,
        },
        terminalVisible: true,
        terminalPanelOpen: true,
      })
      return existingTerminal.id
    }

    // No existing running terminal, create a new one
    const id = get().addTerminal(worktreeId, command)
    set({ terminalVisible: true, terminalPanelOpen: true })
    return id
  },

  closeAllTerminals: worktreeId => {
    const state = get()
    const terminals = state.terminals[worktreeId] ?? []
    const terminalIds = terminals.map(t => t.id)

    // Remove all running terminal IDs for this worktree
    const newRunning = new Set(state.runningTerminals)
    for (const id of terminalIds) {
      newRunning.delete(id)
    }

    set({
      terminals: {
        ...state.terminals,
        [worktreeId]: [],
      },
      activeTerminalIds: {
        ...state.activeTerminalIds,
        [worktreeId]: '',
      },
      runningTerminals: newRunning,
      terminalPanelOpen: false,
      terminalVisible: false,
    })

    return terminalIds
  },
}))
