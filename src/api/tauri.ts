import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import type {
  DownloadRequest,
  TaskProgress,
  ConvertRequest,
  SystemInfo,
} from '../types/tauri';

export class TauriAPI {
  // System commands
  static async checkDependencies(): Promise<SystemInfo> {
    return invoke<SystemInfo>('check_dependencies');
  }

  static async installYtdlp(): Promise<string> {
    return invoke<string>('install_ytdlp_command');
  }

  // Download commands
  static async startDownload(request: DownloadRequest): Promise<string[]> {
    return invoke<string[]>('start_download', { request });
  }

  static async getDownloadTasks(): Promise<TaskProgress[]> {
    return invoke<TaskProgress[]>('get_download_tasks');
  }

  static async getTaskProgress(taskId: string): Promise<TaskProgress | null> {
    return invoke<TaskProgress | null>('get_task_progress', { taskId });
  }

  static async pauseDownload(taskId: string): Promise<void> {
    return invoke<void>('pause_download', { taskId });
  }

  static async cancelDownload(taskId: string): Promise<void> {
    return invoke<void>('cancel_download', { taskId });
  }

  static async removeTask(taskId: string): Promise<void> {
    return invoke<void>('remove_task', { taskId });
  }

  // Conversion commands
  static async startConversion(request: ConvertRequest): Promise<string[]> {
    return invoke<string[]>('start_conversion', { request });
  }

  static async getConversionTasks(): Promise<TaskProgress[]> {
    return invoke<TaskProgress[]>('get_conversion_tasks');
  }

  static async cancelConversion(taskId: string): Promise<void> {
    return invoke<void>('cancel_conversion', { taskId });
  }

  // File system operations
  static async openFolder(path: string): Promise<void> {
    return invoke<void>('open_folder', { path });
  }

  // File dialog
  static async selectDirectory(): Promise<string | null> {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Download Location',
      });
      console.log('Directory selected:', selected);
      return selected as string | null;
    } catch (error) {
      console.error('Error selecting directory:', error);
      throw error;
    }
  }

  static async selectFiles(filters?: { name: string; extensions: string[] }[]): Promise<string[] | null> {
    try {
      const selected = await open({
        directory: false,
        multiple: true,
        title: 'Select Files to Convert',
        filters,
      });
      
      console.log('Files selected:', selected);
      
      if (Array.isArray(selected)) {
        return selected as string[];
      } else if (selected) {
        return [selected as string];
      }
      return null;
    } catch (error) {
      console.error('Error selecting files:', error);
      throw error;
    }
  }

  // Event listeners
  static onTaskUpdate(callback: (task: TaskProgress) => void) {
    return listen<TaskProgress>('task-update', (event) => {
      callback(event.payload);
    });
  }
}
