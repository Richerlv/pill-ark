import { useState, useEffect, useMemo, useRef, type CSSProperties } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './styles.css'

interface Task {
  name: string
  pid: number
  status: string
  start_time: number
  command: string
  tool: string
  cwd: string
  session_id: string
  updated_at: number
}

type IslandStatus = 'idle' | 'running' | 'success' | 'failed'

const STARTUP_EMOJIS = ['💊', '⚡️', '✨', '🚀', '🧠', '🛠️', '🌊', '🔥']
const COMPLETION_DISPLAY_MS = 60_000
const toolLabel = (tool: string) => (tool === 'ClaudeCode' || tool === 'Claude Code' ? 'ClaudeCode' : tool)
const TOOL_TONES = [
  { color: 'rgba(96, 165, 250, 0.86)', bg: 'rgba(96, 165, 250, 0.12)', border: 'rgba(96, 165, 250, 0.22)' },
  { color: 'rgba(245, 158, 11, 0.86)', bg: 'rgba(245, 158, 11, 0.12)', border: 'rgba(245, 158, 11, 0.22)' },
  { color: 'rgba(45, 212, 191, 0.82)', bg: 'rgba(45, 212, 191, 0.11)', border: 'rgba(45, 212, 191, 0.2)' },
  { color: 'rgba(167, 139, 250, 0.82)', bg: 'rgba(167, 139, 250, 0.11)', border: 'rgba(167, 139, 250, 0.2)' },
  { color: 'rgba(244, 114, 182, 0.8)', bg: 'rgba(244, 114, 182, 0.1)', border: 'rgba(244, 114, 182, 0.18)' },
  { color: 'rgba(163, 230, 53, 0.76)', bg: 'rgba(163, 230, 53, 0.1)', border: 'rgba(163, 230, 53, 0.18)' },
]

const PREFERRED_TOOL_TONE_INDEX: Record<string, number> = {
  OpenCode: 0,
  ClaudeCode: 1,
}
const BASE_WINDOW_SIZE = {
  idle: { width: 392, height: 39 },
  running: { width: 452, height: 39 },
  success: { width: 432, height: 39 },
  expanded: { width: 472, height: 154 },
  successExpanded: { width: 472, height: 166 },
}

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)
const scaled = (value: number, scale: number) => Math.round(value * scale)

const createWindowSizes = () => {
  const screenWidth = typeof window !== 'undefined' ? window.screen?.width || 1512 : 1512
  const widthScale = clamp(screenWidth / 1512, 1, 1.08)
  const expandedHeightScale = clamp(screenWidth / 1512, 1, 1.08)

  return {
    idle: { width: scaled(BASE_WINDOW_SIZE.idle.width, widthScale), height: BASE_WINDOW_SIZE.idle.height },
    running: { width: scaled(BASE_WINDOW_SIZE.running.width, widthScale), height: BASE_WINDOW_SIZE.running.height },
    success: { width: scaled(BASE_WINDOW_SIZE.success.width, widthScale), height: BASE_WINDOW_SIZE.success.height },
    expanded: {
      width: scaled(BASE_WINDOW_SIZE.expanded.width, widthScale),
      height: scaled(BASE_WINDOW_SIZE.expanded.height, expandedHeightScale),
    },
    successExpanded: {
      width: scaled(BASE_WINDOW_SIZE.successExpanded.width, widthScale),
      height: scaled(BASE_WINDOW_SIZE.successExpanded.height, expandedHeightScale),
    },
  }
}

function App() {
  const windowSize = useMemo(createWindowSizes, [])
  const startupEmojiRef = useRef(STARTUP_EMOJIS[Math.floor(Math.random() * STARTUP_EMOJIS.length)])
  const [tasks, setTasks] = useState<Task[]>([])
  const [completedTasks, setCompletedTasks] = useState<Task[]>([])
  const [isExpanded, setIsExpanded] = useState(false)
  const [status, setStatus] = useState<IslandStatus>('idle')
  const previousTasksRef = useRef<Task[]>([])
  const completionTimerRef = useRef<number | null>(null)
  const isShowingCompletionRef = useRef(false)
  const toolToneByLabel = useMemo(() => {
    const labels = [...completedTasks, ...tasks].map((task) => toolLabel(task.tool))
    const uniqueLabels = [...new Set(labels)]
    const usedToneIndexes = new Set<number>()
    const toneByLabel = new Map<string, (typeof TOOL_TONES)[number]>()

    uniqueLabels.forEach((label, labelIndex) => {
      const preferredIndex = PREFERRED_TOOL_TONE_INDEX[label]
      const availableIndex = TOOL_TONES.findIndex((_, toneIndex) => !usedToneIndexes.has(toneIndex))
      const toneIndex = preferredIndex !== undefined && !usedToneIndexes.has(preferredIndex)
        ? preferredIndex
        : availableIndex >= 0
          ? availableIndex
          : labelIndex % TOOL_TONES.length

      usedToneIndexes.add(toneIndex)
      toneByLabel.set(label, TOOL_TONES[toneIndex])
    })

    return toneByLabel
  }, [completedTasks, tasks])

  const toolToneStyle = (tool: string) => {
    const label = toolLabel(tool)
    const tone = toolToneByLabel.get(label) ?? TOOL_TONES[PREFERRED_TOOL_TONE_INDEX[label] ?? 0]

    return {
      '--tool-color': tone.color,
      '--tool-bg': tone.bg,
      '--tool-border': tone.border,
    } as CSSProperties
  }

  const clearCompletionTimer = () => {
    if (completionTimerRef.current) {
      window.clearTimeout(completionTimerRef.current)
      completionTimerRef.current = null
    }
  }

  const dismissCompletion = () => {
    clearCompletionTimer()
    isShowingCompletionRef.current = false
    setCompletedTasks([])
    setIsExpanded(false)
    setStatus(previousTasksRef.current.length > 0 ? 'running' : 'idle')
  }

  useEffect(() => {
    // 当状态变化时，调整窗口大小并居中
    const adjustWindow = async () => {
      try {
        if (isExpanded) {
          await invoke('set_window_size_and_center', status === 'success' ? windowSize.successExpanded : windowSize.expanded)
        } else if (status === 'running') {
          await invoke('set_window_size_and_center', windowSize.running)
        } else if (status === 'success') {
          await invoke('set_window_size_and_center', windowSize.success)
        } else {
          await invoke('set_window_size_and_center', windowSize.idle)
        }
      } catch (e) {
        console.error('Failed to adjust window:', e)
      }
    }

    adjustWindow()
  }, [status, isExpanded, windowSize])

  useEffect(() => {
    const fetchTasks = async () => {
      try {
        const result = await invoke<Task[]>('get_running_tasks')
        const previousTasks = previousTasksRef.current
        const currentSessionIds = new Set(result.map((task) => task.session_id))
        const finishedTasks = previousTasks.filter((task) => !currentSessionIds.has(task.session_id))
        const hasRunningTasks = result.length > 0

        if (finishedTasks.length > 0) {
          clearCompletionTimer()

          setCompletedTasks((currentCompletedTasks) => {
            const knownCompletedIds = new Set(currentCompletedTasks.map((task) => task.session_id))
            const newFinishedTasks = finishedTasks.filter((task) => !knownCompletedIds.has(task.session_id))

            return [...currentCompletedTasks, ...newFinishedTasks]
          })
          setStatus('success')
          setIsExpanded(true)
          isShowingCompletionRef.current = true

          completionTimerRef.current = window.setTimeout(() => {
            dismissCompletion()
          }, COMPLETION_DISPLAY_MS)
        } else if (!isShowingCompletionRef.current && hasRunningTasks) {
          setStatus('running')
        } else if (!isShowingCompletionRef.current) {
          setIsExpanded(false)
          setStatus('idle')
        }

        setTasks(result)
        previousTasksRef.current = result
      } catch (e) {
        console.error('Failed to fetch tasks:', e)
      }
    }

    fetchTasks()
    const interval = setInterval(fetchTasks, 1000)
    return () => {
      clearInterval(interval)
      clearCompletionTimer()
    }
  }, [])

  const toggleExpand = () => {
    if (isShowingCompletionRef.current) {
      dismissCompletion()
      return
    }

    setIsExpanded(!isExpanded)
  }

  const openTask = async (event: React.MouseEvent, task: Task) => {
    event.stopPropagation()

    try {
      await invoke('open_task_destination', { task })
    } catch (e) {
      console.error('Failed to open task destination:', e)
    }
  }

  const currentWindowSize = isExpanded
    ? status === 'success'
      ? windowSize.successExpanded
      : windowSize.expanded
    : windowSize[status === 'failed' ? 'idle' : status]
  const pillStyle = {
    '--pill-width': `${currentWindowSize.width}px`,
    '--pill-height': `${currentWindowSize.height}px`,
  } as CSSProperties

  return (
    <div className={`pill ${status} ${isExpanded ? 'expanded' : ''}`} style={pillStyle} onClick={toggleExpand}>
      <div className="pill-content">
        {status === 'idle' && (
          <div className="island-grid idle-state">
            <span className="status-mark idle-mark">{startupEmojiRef.current}</span>
            <span className="notch-space" />
            <span className="island-label">PillArk</span>
          </div>
        )}
        {status === 'running' && (
          <div className="island-grid running-state">
            <span className="status-mark status-emoji running-emoji">⏳</span>
            <span className="notch-space" />
            <span className="count">{tasks.length} Running</span>
          </div>
        )}
        {status === 'success' && (
          <div className="island-grid success-state">
            <span className="status-mark status-emoji success-emoji">✅</span>
            <span className="notch-space" />
            <span className="count">
              {completedTasks.length} Done
              {tasks.length > 0 && ` · ${tasks.length} running`}
            </span>
          </div>
        )}
      </div>

      {isExpanded && (
        <div className="task-list" onClick={(event) => event.stopPropagation()}>
          {completedTasks.length > 0 && (
            <div className="task-section">
              <div className="section-title">Completed</div>
              {completedTasks.map((task) => (
                <button key={`done-${task.session_id}`} className="task-item done" type="button" onClick={(event) => openTask(event, task)}>
                  <span className="task-name">
                    <span className="task-tool" style={toolToneStyle(task.tool)}>{toolLabel(task.tool)}</span> {task.name}
                  </span>
                  <span className="task-pid">{task.pid > 0 ? `PID: ${task.pid}` : 'Session task'}</span>
                </button>
              ))}
            </div>
          )}

          {tasks.length > 0 && (
            <div className="task-section">
              <div className="section-title">Running</div>
              {tasks.map((task) => (
                <button key={task.session_id} className="task-item" type="button" onClick={(event) => openTask(event, task)}>
                  <span className="task-name">
                    <span className="task-tool" style={toolToneStyle(task.tool)}>{toolLabel(task.tool)}</span> {task.name}
                  </span>
                  <span className="task-pid">{task.pid > 0 ? `PID: ${task.pid}` : 'Session task'}</span>
                </button>
              ))}
            </div>
          )}

          {tasks.length === 0 && completedTasks.length === 0 && <div className="empty">No active tasks</div>}
        </div>
      )}
    </div>
  )
}

export default App
