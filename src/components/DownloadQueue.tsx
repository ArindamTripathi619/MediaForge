import { useState, useEffect } from 'react';
import { ChevronRight, ChevronLeft, Pause, X, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { TauriAPI } from '../api/tauri';
import type { TaskProgress, TaskStatus } from '../types/tauri';

function DownloadQueue() {
  const [isExpanded, setIsExpanded] = useState(true);
  const [tasks, setTasks] = useState<TaskProgress[]>([]);

  // Helper function to extract filename from path
  const getFileName = (path: string): string => {
    // Handle both Windows and Unix paths
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
  };

  useEffect(() => {
    // Load initial tasks
    const loadTasks = async () => {
      try {
        const downloadTasks = await TauriAPI.getDownloadTasks();
        const conversionTasks = await TauriAPI.getConversionTasks();
        setTasks([...downloadTasks, ...conversionTasks]);
      } catch (error) {
        console.error('Failed to load tasks:', error);
      }
    };

    loadTasks();

    // Subscribe to task updates
    const unsubscribe = TauriAPI.onTaskUpdate((updatedTask) => {
      setTasks((prevTasks) => {
        const index = prevTasks.findIndex(t => t.task_id === updatedTask.task_id);
        if (index >= 0) {
          const newTasks = [...prevTasks];
          newTasks[index] = updatedTask;
          return newTasks;
        } else {
          return [...prevTasks, updatedTask];
        }
      });
    });

    return () => {
      unsubscribe.then(unsub => unsub());
    };
  }, []);

  const handlePause = async (taskId: string) => {
    try {
      await TauriAPI.pauseDownload(taskId);
    } catch (error) {
      console.error('Failed to pause task:', error);
    }
  };

  const handleCancel = async (taskId: string) => {
    try {
      await TauriAPI.cancelDownload(taskId);
    } catch (error) {
      console.error('Failed to cancel task:', error);
    }
  };

  const handleRemove = async (taskId: string) => {
    try {
      await TauriAPI.removeTask(taskId);
      setTasks(prevTasks => prevTasks.filter(t => t.task_id !== taskId));
    } catch (error) {
      console.error('Failed to remove task:', error);
    }
  };

  const handleClearCompleted = () => {
    const completedTasks = tasks.filter(t => t.status === 'Completed' || t.status === 'Failed' || t.status === 'Cancelled');
    completedTasks.forEach(task => {
      handleRemove(task.task_id);
    });
  };

  const getStatusIcon = (status: TaskStatus) => {
    switch (status) {
      case 'Downloading':
      case 'Processing':
        return <Loader2 className="w-4 h-4 text-cyan-400 animate-spin" />;
      case 'Paused':
        return <Pause className="w-4 h-4 text-yellow-400" />;
      case 'Completed':
        return <CheckCircle className="w-4 h-4 text-green-400" />;
      case 'Failed':
        return <AlertCircle className="w-4 h-4 text-red-400" />;
      case 'Queued':
        return <Loader2 className="w-4 h-4 text-slate-400" />;
      case 'Cancelled':
        return <X className="w-4 h-4 text-slate-400" />;
    }
  };

  const getProgressColor = (status: TaskStatus) => {
    switch (status) {
      case 'Downloading':
      case 'Processing':
        return 'bg-cyan-500';
      case 'Paused':
        return 'bg-yellow-500';
      case 'Completed':
        return 'bg-green-500';
      case 'Failed':
        return 'bg-red-500';
      case 'Queued':
        return 'bg-slate-500';
      case 'Cancelled':
        return 'bg-slate-600';
    }
  };

  return (
    <aside className={`hidden sm:flex border-l border-slate-700 bg-slate-900/30 transition-all duration-300 ${isExpanded ? 'w-64 lg:w-80' : 'w-12'} relative flex-col flex-shrink-0`}>
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="absolute -left-3 top-4 w-6 h-6 bg-slate-700 hover:bg-slate-600 rounded-full flex items-center justify-center transition-all shadow-lg z-10"
      >
        {isExpanded ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
      </button>

      {isExpanded && (
        <div className="flex flex-col h-full min-w-0">
          <div className="p-3 lg:p-4 border-b border-slate-700">
            <h3 className="font-semibold text-sm lg:text-lg">Queue</h3>
            <p className="text-xs text-slate-400 mt-1">{tasks.length} tasks</p>
          </div>

          <div className="flex-1 overflow-y-auto p-2 lg:p-4 space-y-2 lg:space-y-3 min-h-0">
            {tasks.length === 0 ? (
              <div className="text-center text-slate-500 text-xs py-6">
                No active tasks
              </div>
            ) : (
              tasks.map((task) => (
                <div
                  key={task.task_id}
                  className="bg-slate-800/50 rounded-lg p-2.5 lg:p-3 border border-slate-700 hover:border-slate-600 transition-all"
                >
                  <div className="flex items-start justify-between mb-2 gap-2 min-w-0">
                    <div className="flex items-start gap-1.5 flex-1 min-w-0">
                      {getStatusIcon(task.status)}
                      <p className="text-xs lg:text-sm font-medium text-slate-200 truncate" title={task.name}>
                        {getFileName(task.name)}
                      </p>
                    </div>
                    <div className="flex gap-1 flex-shrink-0">
                      {task.status === 'Downloading' && (
                        <button 
                          onClick={() => handlePause(task.task_id)}
                          className="p-0.5 hover:bg-slate-700 rounded transition-all"
                        >
                          <Pause className="w-3 h-3 lg:w-3.5 lg:h-3.5 text-slate-400" />
                        </button>
                      )}
                      {task.status !== 'Completed' && task.status !== 'Cancelled' && (
                        <button 
                          onClick={() => handleCancel(task.task_id)}
                          className="p-0.5 hover:bg-slate-700 rounded transition-all"
                        >
                          <X className="w-3 h-3 lg:w-3.5 lg:h-3.5 text-slate-400" />
                        </button>
                      )}
                      {(task.status === 'Completed' || task.status === 'Failed' || task.status === 'Cancelled') && (
                        <button 
                          onClick={() => handleRemove(task.task_id)}
                          className="p-0.5 hover:bg-slate-700 rounded transition-all"
                        >
                          <X className="w-3 h-3 lg:w-3.5 lg:h-3.5 text-slate-400" />
                        </button>
                      )}
                    </div>
                  </div>

                  <div className="space-y-1.5">
                    <div className="w-full h-1.5 lg:h-2 bg-slate-900 rounded-full overflow-hidden">
                      <div
                        className={`h-full ${getProgressColor(task.status)} transition-all duration-300 rounded-full`}
                        style={{ width: `${task.progress}%` }}
                      />
                    </div>

                    <div className="flex justify-between text-[10px] lg:text-xs text-slate-400">
                      <span>{task.progress.toFixed(1)}%</span>
                      {task.speed && <span className="hidden lg:inline truncate">{task.speed}</span>}
                      {task.eta && <span className="hidden lg:inline truncate">ETA: {task.eta}</span>}
                    </div>

                    {task.error && (
                      <div className="mt-1.5 p-2 bg-red-500/10 border border-red-500/30 rounded text-[10px] lg:text-xs text-red-400 break-words">
                        {task.error}
                      </div>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>

          <div className="p-2 lg:p-3 border-t border-slate-700 space-y-2 flex-shrink-0">
            <button 
              onClick={handleClearCompleted}
              className="w-full py-1.5 lg:py-2 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-400 rounded-lg text-xs lg:text-sm font-medium transition-all"
            >
              Clear Completed
            </button>
          </div>
        </div>
      )}
    </aside>
  );
}

export default DownloadQueue;
