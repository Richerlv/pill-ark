import { useState, useEffect, useRef } from 'react'
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

function App() {
  const startupEmojiRef = useRef(STARTUP_EMOJIS[Math.floor(Math.random() * STARTUP_EMOJIS.length)])
  const [tasks, setTasks] = useState<Task[]>([])
  const [completedTasks, setCompletedTasks] = useState<Task[]>([])
  const [isExpanded, setIsExpanded] = useState(false)
  const [status, setStatus] = useState<IslandStatus>('idle')
  const previousTasksRef = useRef<Task[]>([])
  const completionTimerRef = useRef<number | null>(null)
  const isShowingCompletionRef = useRef(false)

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
          await invoke('set_window_size_and_center', { width: 320, height: status === 'success' ? 138 : 128 })
        } else if (status === 'running') {
          await invoke('set_window_size_and_center', { width: 240, height: 37 })
        } else if (status === 'success') {
          await invoke('set_window_size_and_center', { width: 220, height: 37 })
        } else {
          await invoke('set_window_size_and_center', { width: 185, height: 37 })
        }
      } catch (e) {
        console.error('Failed to adjust window:', e)
      }
    }

    adjustWindow()
  }, [status, isExpanded])

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

  return (
    <div className={`pill ${status} ${isExpanded ? 'expanded' : ''}`} onClick={toggleExpand}>
      <div className="pill-content">
        {status === 'idle' && (
          <div className="idle-state">
            <span className="text">{startupEmojiRef.current} PillArk</span>
          </div>
        )}
        {status === 'running' && (
          <div className="running-state">
            <div className="energy-bar" />
            <span className="count">{startupEmojiRef.current} {tasks.length} Claude task{tasks.length > 1 ? 's' : ''} running</span>
          </div>
        )}
        {status === 'success' && (
          <div className="success-state">
            <span className="success-dot" />
            <span className="count">
              {startupEmojiRef.current} {completedTasks.length} task{completedTasks.length > 1 ? 's' : ''} done
              {tasks.length > 0 && ` · ${tasks.length} running`}
            </span>
          </div>
        )}
      </div>

      {isExpanded && (
        <div className="task-list">
          {completedTasks.length > 0 && (
            <div className="task-section">
              <div className="section-title">Completed</div>
              {completedTasks.map((task) => (
                <div key={`done-${task.session_id}`} className="task-item done">
                  <span className="task-name">{task.name}</span>
                  <span className="task-pid">PID: {task.pid}</span>
                </div>
              ))}
            </div>
          )}

          {tasks.length > 0 && (
            <div className="task-section">
              <div className="section-title">Running</div>
              {tasks.map((task) => (
                <div key={task.session_id} className="task-item">
                  <span className="task-name">{task.name}</span>
                  <span className="task-pid">PID: {task.pid}</span>
                </div>
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
