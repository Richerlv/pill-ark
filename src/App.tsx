import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './styles.css'

interface Task {
  name: string
  pid: number
  status: string
  start_time: number
}

function App() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [isExpanded, setIsExpanded] = useState(false)
  const [status, setStatus] = useState<'idle' | 'running' | 'success' | 'failed'>('idle')
  const prevStatusRef = useRef(status)

  useEffect(() => {
    // 当状态变化时，调整窗口大小
    const adjustWindowSize = async () => {
      try {
        if (isExpanded) {
          await invoke('set_window_size', { width: 280, height: 300 })
        } else if (status === 'running' && prevStatusRef.current !== 'running') {
          await invoke('set_window_size', { width: 240, height: 56 })
        } else if (status === 'idle' && prevStatusRef.current !== 'idle') {
          await invoke('set_window_size', { width: 120, height: 50 })
        }
        prevStatusRef.current = status
      } catch (e) {
        console.error('Failed to adjust window size:', e)
      }
    }

    adjustWindowSize()
  }, [status, isExpanded])

  useEffect(() => {
    const fetchTasks = async () => {
      try {
        const result = await invoke<Task[]>('get_running_tasks')
        setTasks(result)
        setStatus(result.length > 0 ? 'running' : 'idle')
      } catch (e) {
        console.error('Failed to fetch tasks:', e)
      }
    }

    fetchTasks()
    const interval = setInterval(fetchTasks, 2000)
    return () => clearInterval(interval)
  }, [])

  const toggleExpand = () => {
    setIsExpanded(!isExpanded)
  }

  return (
    <div className={`pill ${status} ${isExpanded ? 'expanded' : ''}`}>
      <div className="ocean">
        <div className="waves" />
        <div className="whale-container">
          <img src="/whale.png" alt="whale" className="whale" />
        </div>
      </div>
      <div className="pill-content" onClick={toggleExpand}>
        {status === 'idle' && (
          <div className="idle-state">
            <span className="text">PillArk</span>
          </div>
        )}
        {status === 'running' && (
          <div className="running-state">
            <div className="energy-bar" />
            <span className="count">{tasks.length} Agent{tasks.length > 1 ? 's' : ''} Running...</span>
          </div>
        )}
      </div>

      {isExpanded && (
        <div className="task-list">
          {tasks.length === 0 ? (
            <div className="empty">No active tasks</div>
          ) : (
            tasks.map((task, i) => (
              <div key={i} className="task-item">
                <span className="task-name">{task.name}</span>
                <span className="task-pid">PID: {task.pid}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  )
}

export default App