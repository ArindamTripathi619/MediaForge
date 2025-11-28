import { useState } from 'react';
import { Link, Music, Video, List, Scissors, FolderOpen, Plus, X } from 'lucide-react';
import { TauriAPI } from '../api/tauri';
import type { DownloadType, MediaFormat } from '../types/tauri';

function DownloadSection() {
  const [downloadType, setDownloadType] = useState<DownloadType>('Single');
  const [format, setFormat] = useState<MediaFormat>('mp4');
  const [urls, setUrls] = useState(['']);
  const [quality, setQuality] = useState('1080');
  const [audioQuality, setAudioQuality] = useState('192');
  const [enableTrim, setEnableTrim] = useState(false);
  const [startTime, setStartTime] = useState('00:00:00');
  const [endTime, setEndTime] = useState('00:00:00');
  const [downloadPath, setDownloadPath] = useState('~/Downloads');
  const [isDownloading, setIsDownloading] = useState(false);

  const addUrlField = () => setUrls([...urls, '']);
  const removeUrlField = (index: number) => {
    if (urls.length > 1) {
      setUrls(urls.filter((_, i) => i !== index));
    }
  };
  const updateUrl = (index: number, value: string) => {
    const newUrls = [...urls];
    newUrls[index] = value;
    setUrls(newUrls);
  };

  const handleSelectDirectory = async () => {
    try {
      const selected = await TauriAPI.selectDirectory();
      if (selected) {
        setDownloadPath(selected);
      }
    } catch (error) {
      console.error('Failed to open directory selector:', error);
    }
  };

  const handleStartDownload = async () => {
    const validUrls = urls.filter(url => url.trim() !== '');
    if (validUrls.length === 0) {
      alert('Please enter at least one valid URL');
      return;
    }

    setIsDownloading(true);
    try {
      const request = {
        urls: validUrls,
        download_type: downloadType,
        format,
        quality: format === 'mp4' ? quality : undefined,
        audio_quality: format === 'mp3' ? audioQuality : undefined,
        download_path: downloadPath,
        trim: enableTrim ? {
          start_time: startTime,
          end_time: endTime,
        } : undefined,
      };

      const taskIds = await TauriAPI.startDownload(request);
      console.log('Download started:', taskIds);
      
      // Reset form
      setUrls(['']);
      setEnableTrim(false);
      alert(`Started ${taskIds.length} download(s) successfully!`);
    } catch (error) {
      console.error('Download failed:', error);
      alert(`Failed to start download: ${error}`);
    } finally {
      setIsDownloading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl sm:text-3xl font-bold mb-2">Download Media</h2>
        <p className="text-xs sm:text-sm text-slate-400">Download videos and audio from YouTube and other platforms</p>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 sm:gap-4">
        {(['Single', 'Bulk', 'Playlist'] as DownloadType[]).map((type) => {
          const icons = { Single: Link, Bulk: List, Playlist: Video };
          const Icon = icons[type];
          const labels = { Single: 'Single', Bulk: 'Bulk', Playlist: 'Playlist' };

          return (
            <button
              key={type}
              onClick={() => setDownloadType(type)}
              className={`p-2 sm:p-4 rounded-lg sm:rounded-xl border-2 transition-all ${
                downloadType === type
                  ? 'border-cyan-500 bg-cyan-500/10 shadow-lg shadow-cyan-500/20'
                  : 'border-slate-700 bg-slate-800/50 hover:border-slate-600'
              }`}
            >
              <Icon className={`w-5 h-5 sm:w-6 sm:h-6 mb-1 sm:mb-2 ${downloadType === type ? 'text-cyan-400' : 'text-slate-400'}`} />
              <p className={`text-xs sm:text-sm font-semibold ${downloadType === type ? 'text-cyan-400' : 'text-slate-300'}`}>
                {labels[type]}
              </p>
            </button>
          );
        })}
      </div>

      <div className="bg-slate-800/50 rounded-lg sm:rounded-xl p-3 sm:p-6 border border-slate-700 space-y-4 sm:space-y-6">
        <div>
          <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Format</label>
          <div className="grid grid-cols-2 gap-2 sm:gap-4">
            {(['mp4', 'mp3'] as MediaFormat[]).map((fmt) => {
              const Icon = fmt === 'mp4' ? Video : Music;
              return (
                <button
                  key={fmt}
                  onClick={() => setFormat(fmt)}
                  className={`flex items-center justify-center gap-1 sm:gap-2 px-2 sm:px-4 py-2 sm:py-3 rounded-lg font-medium text-xs sm:text-sm transition-all ${
                    format === fmt
                      ? 'bg-gradient-to-r from-cyan-500 to-blue-600 text-white shadow-lg'
                      : 'bg-slate-700/50 text-slate-300 hover:bg-slate-700'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  {fmt.toUpperCase()}
                </button>
              );
            })}
          </div>
        </div>

        <div>
          <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">
            {downloadType === 'Playlist' ? 'Playlist URL' : downloadType === 'Bulk' ? 'URLs' : 'URL'}
          </label>
          <div className="space-y-2 sm:space-y-3">
            {(downloadType === 'Bulk' ? urls : urls.slice(0, 1)).map((url, index) => (
              <div key={index} className="flex gap-2 min-w-0">
                <input
                  type="text"
                  value={url}
                  onChange={(e) => updateUrl(index, e.target.value)}
                  placeholder={`Enter ${downloadType === 'Playlist' ? 'playlist' : 'video'} URL`}
                  className="flex-1 min-w-0 px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                />
                {downloadType === 'Bulk' && urls.length > 1 && (
                  <button
                    onClick={() => removeUrlField(index)}
                    className="flex-shrink-0 px-2 sm:px-3 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-400 rounded-lg transition-all"
                  >
                    <X className="w-4 h-4 sm:w-5 sm:h-5" />
                  </button>
                )}
              </div>
            ))}
          </div>
          {downloadType === 'Bulk' && (
            <button
              onClick={addUrlField}
              className="mt-2 sm:mt-3 flex items-center gap-2 px-3 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-700/50 hover:bg-slate-700 text-slate-300 rounded-lg transition-all"
            >
              <Plus className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
              Add URL
            </button>
          )}
        </div>

        <div className="grid grid-cols-1 gap-4 sm:gap-6">
          <div>
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">
              {format === 'mp4' ? 'Video Quality' : 'Audio Quality'}
            </label>
            {format === 'mp4' ? (
              <select
                value={quality}
                onChange={(e) => setQuality(e.target.value)}
                style={{ backgroundColor: '#0f172a', color: 'white' }}
                className="w-full px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
              >
                <option value="2160" style={{ backgroundColor: '#0f172a', color: 'white' }}>4K (2160p)</option>
                <option value="1440" style={{ backgroundColor: '#0f172a', color: 'white' }}>2K (1440p)</option>
                <option value="1080" style={{ backgroundColor: '#0f172a', color: 'white' }}>Full HD (1080p)</option>
                <option value="720" style={{ backgroundColor: '#0f172a', color: 'white' }}>HD (720p)</option>
                <option value="480" style={{ backgroundColor: '#0f172a', color: 'white' }}>SD (480p)</option>
                <option value="360" style={{ backgroundColor: '#0f172a', color: 'white' }}>360p</option>
              </select>
            ) : (
              <select
                value={audioQuality}
                onChange={(e) => setAudioQuality(e.target.value)}
                style={{ backgroundColor: '#0f172a', color: 'white' }}
                className="w-full px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
              >
                <option value="320" style={{ backgroundColor: '#0f172a', color: 'white' }}>320 kbps</option>
                <option value="256" style={{ backgroundColor: '#0f172a', color: 'white' }}>256 kbps</option>
                <option value="192" style={{ backgroundColor: '#0f172a', color: 'white' }}>192 kbps</option>
                <option value="128" style={{ backgroundColor: '#0f172a', color: 'white' }}>128 kbps</option>
              </select>
            )}
          </div>

          <div>
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Download Location</label>
            <div className="flex gap-2 min-w-0">
              <input
                type="text"
                value={downloadPath}
                onChange={(e) => setDownloadPath(e.target.value)}
                className="flex-1 min-w-0 px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
              />
              <button 
                type="button"
                onClick={handleSelectDirectory}
                className="flex-shrink-0 px-3 sm:px-4 bg-slate-700 hover:bg-slate-600 rounded-lg transition-all"
              >
                <FolderOpen className="w-4 h-4 sm:w-5 sm:h-5" />
              </button>
            </div>
          </div>
        </div>

        <div className="border-t border-slate-700 pt-4 sm:pt-6">
          <label className="flex items-center gap-2 sm:gap-3 cursor-pointer group">
            <input
              type="checkbox"
              checked={enableTrim}
              onChange={(e) => setEnableTrim(e.target.checked)}
              className="w-4 h-4 sm:w-5 sm:h-5 rounded border-slate-600 bg-slate-900/50 text-cyan-500 focus:ring-2 focus:ring-cyan-500/20"
            />
            <div className="flex items-center gap-1 sm:gap-2">
              <Scissors className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-400 group-hover:text-cyan-400 transition-colors" />
              <span className="text-xs sm:text-base font-medium text-slate-300 group-hover:text-slate-100 transition-colors">
                Trim media
              </span>
            </div>
          </label>

          {enableTrim && (
            <div className="mt-3 sm:mt-4 grid grid-cols-2 gap-2 sm:gap-4 pl-4 sm:pl-8">
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1 sm:mb-2">Start</label>
                <input
                  type="text"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  placeholder="00:00:00"
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                />
              </div>
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1 sm:mb-2">End</label>
                <input
                  type="text"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  placeholder="00:00:00"
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                />
              </div>
            </div>
          )}
        </div>
      </div>

      <button 
        onClick={handleStartDownload}
        disabled={isDownloading}
        className="w-full py-3 sm:py-4 text-xs sm:text-sm lg:text-base bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-600 hover:to-blue-700 disabled:from-slate-600 disabled:to-slate-700 disabled:cursor-not-allowed text-white font-semibold rounded-lg sm:rounded-xl shadow-lg shadow-cyan-500/30 hover:shadow-cyan-500/50 transition-all transform hover:scale-[1.02] disabled:scale-100"
      >
        {isDownloading ? 'Starting Download...' : 'Start Download'}
      </button>
    </div>
  );
}

export default DownloadSection;
