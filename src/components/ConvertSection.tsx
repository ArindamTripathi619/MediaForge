import { useState } from 'react';
import { Upload, Image, Film, Headphones, FileType, FolderOpen } from 'lucide-react';
import { TauriAPI } from '../api/tauri';
import type { ConversionType } from '../types/tauri';
import { useToastContext } from '../contexts/ToastContext';
import { validateFileSize, validateOutputPath } from '../utils/validation';

type MediaType = 'Image' | 'Video' | 'Audio';

function ConvertSection() {
  const { warning, success, error } = useToastContext();
  const [mediaType, setMediaType] = useState<ConversionType>('Video');
  const [outputFormat, setOutputFormat] = useState('mp4');
  const [outputPath, setOutputPath] = useState('~/Downloads');
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [isConverting, setIsConverting] = useState(false);
  
  // Image settings
  const [imageQuality, setImageQuality] = useState(85);
  const [resizeOption, setResizeOption] = useState('Keep Original');
  const [customWidth, setCustomWidth] = useState('');
  const [customHeight, setCustomHeight] = useState('');
  
  // Video settings
  const [videoResolution, setVideoResolution] = useState('Keep Original');
  const [videoBitrate, setVideoBitrate] = useState('Keep Original');
  
  // Audio settings
  const [audioBitrate, setAudioBitrate] = useState('192');
  const [audioSampleRate, setAudioSampleRate] = useState('44100');

  const formatOptions = {
    Image: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'tiff', 'ico', 'svg'],
    Video: ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv', 'mpeg', 'ogv'],
    Audio: ['mp3', 'wav', 'ogg', 'flac', 'aac', 'm4a', 'wma', 'opus'],
  };

  const handleSelectFiles = async () => {
    const filters = mediaType === 'Image' 
      ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'tiff'] }]
      : mediaType === 'Video'
      ? [{ name: 'Videos', extensions: ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv'] }]
      : [{ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'flac', 'aac', 'm4a', 'wma'] }];

    const selected = await TauriAPI.selectFiles(filters);
    if (selected) {
      setSelectedFiles(selected);
    }
  };

  const handleFileDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const files = Array.from(e.dataTransfer.files);
    
    // Validate file sizes
    const maxSize = mediaType === 'Video' ? 5 * 1024 * 1024 * 1024 : 2 * 1024 * 1024 * 1024; // 5GB for video, 2GB for others
    const invalidFiles = files.filter(file => {
      const sizeValidation = validateFileSize(file.size, maxSize);
      return !sizeValidation.isValid;
    });

    if (invalidFiles.length > 0) {
      const invalidFile = invalidFiles[0];
      const sizeValidation = validateFileSize(invalidFile.size, maxSize);
      warning('File Too Large', `${invalidFile.name}: ${sizeValidation.message}`);
      return;
    }

    // Convert File objects to paths (we'll use the name for display)
    // Note: In a real desktop app, we'd need to handle this differently
    const filePaths = files.map(file => file.name);
    setSelectedFiles(filePaths);
    
    success('Files Added', `Added ${files.length} file(s) for conversion`);
  };

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
  };

  const handleSelectDirectory = async () => {
    try {
      const selected = await TauriAPI.selectDirectory();
      if (selected) {
        setOutputPath(selected);
      }
    } catch (error) {
      console.error('Failed to open directory selector:', error);
    }
  };

  const handleStartConversion = async () => {
    if (selectedFiles.length === 0) {
      warning('No Files Selected', 'Please select files to convert');
      return;
    }

    // Validate output path
    const pathValidation = validateOutputPath(outputPath);
    if (!pathValidation.isValid) {
      warning('Invalid Output Path', pathValidation.message || 'Please select a valid output directory');
      return;
    }

    setIsConverting(true);
    try {
      // Calculate resize value for image settings
      let resizeValue: string | undefined = undefined;
      if (mediaType === 'Image') {
        if (resizeOption === '50% Size') {
          resizeValue = '50%';
        } else if (resizeOption === '25% Size') {
          resizeValue = '25%';
        } else if (resizeOption === 'Custom') {
          // Format: "WIDTHxHEIGHT" or just "WIDTH" for proportional
          if (customWidth && customHeight) {
            resizeValue = `${customWidth}x${customHeight}`;
          } else if (customWidth) {
            resizeValue = customWidth; // Proportional resize based on width
          }
        }
      }

      const request = {
        input_files: selectedFiles,
        conversion_type: mediaType,
        output_format: outputFormat,
        output_path: outputPath,
        video_settings: mediaType === 'Video' ? {
          resolution: videoResolution !== 'Keep Original' ? videoResolution : undefined,
          bitrate: videoBitrate !== 'Keep Original' ? videoBitrate : undefined,
        } : undefined,
        audio_settings: mediaType === 'Audio' ? {
          bitrate: audioBitrate,
          sample_rate: audioSampleRate,
        } : undefined,
        image_settings: mediaType === 'Image' ? {
          quality: imageQuality,
          resize: resizeValue,
        } : undefined,
      };

      const taskIds = await TauriAPI.startConversion(request);
      console.log('Conversion started:', taskIds);
      
      // Reset form
      setSelectedFiles([]);
      setCustomWidth('');
      setCustomHeight('');
      success('Conversion Started', `Started ${taskIds.length} conversion(s) successfully!`);
    } catch (err) {
      console.error('Conversion failed:', err);
      error('Conversion Failed', `Failed to start conversion: ${err}`);
    } finally {
      setIsConverting(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl sm:text-3xl font-bold mb-2">Convert Media</h2>
        <p className="text-xs sm:text-sm text-slate-400">Convert images, videos, and audio to different formats</p>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 sm:gap-4">
        {(['Image', 'Video', 'Audio'] as MediaType[]).map((type) => {
          const icons = { Image: Image, Video: Film, Audio: Headphones };
          const Icon = icons[type];
          const labels = { Image: 'Image', Video: 'Video', Audio: 'Audio' };

          return (
            <button
              key={type}
              onClick={() => {
                setMediaType(type);
                setOutputFormat(formatOptions[type][0]);
              }}
              className={`p-2 sm:p-4 rounded-lg sm:rounded-xl border-2 transition-all ${
                mediaType === type
                  ? 'border-cyan-500 bg-cyan-500/10 shadow-lg shadow-cyan-500/20'
                  : 'border-slate-700 bg-slate-800/50 hover:border-slate-600'
              }`}
            >
              <Icon className={`w-5 h-5 sm:w-6 sm:h-6 mb-1 sm:mb-2 ${mediaType === type ? 'text-cyan-400' : 'text-slate-400'}`} />
              <p className={`text-xs sm:text-sm font-semibold ${mediaType === type ? 'text-cyan-400' : 'text-slate-300'}`}>
                {labels[type]}
              </p>
            </button>
          );
        })}
      </div>

      <div className="bg-slate-800/50 rounded-lg sm:rounded-xl p-3 sm:p-6 border border-slate-700 space-y-4 sm:space-y-6">
        <div>
          <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Select Files</label>
          <div 
            onClick={handleSelectFiles}
            onDrop={handleFileDrop}
            onDragOver={handleDragOver}
            className="border-2 border-dashed border-slate-600 rounded-lg sm:rounded-xl p-4 sm:p-12 text-center hover:border-cyan-500 hover:bg-slate-800/30 transition-all cursor-pointer group"
          >
            <div className="flex flex-col items-center gap-2 sm:gap-3">
              <div className="w-10 h-10 sm:w-16 sm:h-16 bg-slate-700 rounded-full flex items-center justify-center group-hover:bg-cyan-500/20 transition-all">
                <Upload className="w-5 h-5 sm:w-8 sm:h-8 text-slate-400 group-hover:text-cyan-400 transition-colors" />
              </div>
              <div>
                <p className="text-xs sm:text-lg font-semibold text-slate-300 group-hover:text-cyan-400 transition-colors">
                  {selectedFiles.length > 0 ? `${selectedFiles.length} file(s) selected` : 'Drop files or click'}
                </p>
                <p className="text-xs text-slate-500 mt-1">
                  Multiple files supported • Max: {mediaType === 'Video' ? '5GB' : '2GB'} per file
                </p>
              </div>
            </div>
          </div>
          
          {selectedFiles.length > 0 && (
            <div className="mt-3 sm:mt-4">
              <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2">Selected Files:</label>
              <div className="space-y-1.5 sm:space-y-2 max-h-32 overflow-y-auto">
                {selectedFiles.map((file, index) => {
                  const fileName = file.split('/').pop() || file.split('\\').pop() || file;
                  return (
                    <div
                      key={index}
                      className="flex items-center gap-2 px-3 py-2 bg-slate-900/50 border border-slate-700 rounded-lg text-xs sm:text-sm"
                    >
                      <div className="flex-shrink-0 w-6 h-6 sm:w-7 sm:h-7 bg-slate-700 rounded flex items-center justify-center">
                        {mediaType === 'Image' ? (
                          <Image className="w-3 h-3 sm:w-4 sm:h-4 text-cyan-400" />
                        ) : mediaType === 'Video' ? (
                          <Film className="w-3 h-3 sm:w-4 sm:h-4 text-cyan-400" />
                        ) : (
                          <Headphones className="w-3 h-3 sm:w-4 sm:h-4 text-cyan-400" />
                        )}
                      </div>
                      <span className="flex-1 text-slate-300 truncate" title={fileName}>
                        {fileName}
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedFiles(selectedFiles.filter((_, i) => i !== index));
                        }}
                        className="flex-shrink-0 text-slate-400 hover:text-red-400 transition-colors"
                        title="Remove file"
                      >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        <div className="grid grid-cols-1 gap-4 sm:gap-6">
          <div>
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">
              <div className="flex items-center gap-2">
                <FileType className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                <span className="truncate">Format</span>
              </div>
            </label>
            <div className="grid grid-cols-3 gap-1 sm:gap-2 max-h-48 overflow-y-auto p-0.5 sm:p-1">
              {formatOptions[mediaType].map((format) => (
                <button
                  key={format}
                  onClick={() => setOutputFormat(format)}
                  className={`px-2 sm:px-3 py-1.5 sm:py-2 rounded-lg font-medium text-xs sm:text-sm transition-all ${
                    outputFormat === format
                      ? 'bg-gradient-to-r from-cyan-500 to-blue-600 text-white shadow-lg'
                      : 'bg-slate-700/50 text-slate-300 hover:bg-slate-700'
                  }`}
                >
                  {format.toUpperCase()}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Output Location</label>
            <div className="flex gap-2 min-w-0">
              <input
                type="text"
                value={outputPath}
                onChange={(e) => setOutputPath(e.target.value)}
                className="flex-1 min-w-0 px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
              />
              <button 
                type="button"
                onClick={handleSelectDirectory}
                className="flex-shrink-0 px-2 sm:px-4 bg-slate-700 hover:bg-slate-600 rounded-lg transition-all"
              >
                <FolderOpen className="w-4 h-4 sm:w-5 sm:h-5" />
              </button>
            </div>
          </div>
        </div>

        {mediaType === 'Video' && (
          <div className="border-t border-slate-700 pt-4 sm:pt-6">
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Video Settings</label>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 sm:gap-4">
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Resolution</label>
                <select 
                  value={videoResolution}
                  onChange={(e) => setVideoResolution(e.target.value)}
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                >
                  <option>Keep Original</option>
                  <option>3840x2160</option>
                  <option>2560x1440</option>
                  <option>1920x1080</option>
                  <option>1280x720</option>
                </select>
              </div>
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Bitrate</label>
                <select 
                  value={videoBitrate}
                  onChange={(e) => setVideoBitrate(e.target.value)}
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                >
                  <option>Keep Original</option>
                  <option>8000k</option>
                  <option>5000k</option>
                  <option>2500k</option>
                  <option>1000k</option>
                </select>
              </div>
            </div>
          </div>
        )}

        {mediaType === 'Audio' && (
          <div className="border-t border-slate-700 pt-4 sm:pt-6">
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Audio Settings</label>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 sm:gap-4">
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Bitrate</label>
                <select 
                  value={audioBitrate}
                  onChange={(e) => setAudioBitrate(e.target.value)}
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                >
                  <option value="320">320 kbps</option>
                  <option value="256">256 kbps</option>
                  <option value="192">192 kbps</option>
                  <option value="128">128 kbps</option>
                </select>
              </div>
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Sample Rate</label>
                <select 
                  value={audioSampleRate}
                  onChange={(e) => setAudioSampleRate(e.target.value)}
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                >
                  <option value="48000">48000 Hz</option>
                  <option value="44100">44100 Hz</option>
                  <option value="22050">22050 Hz</option>
                </select>
              </div>
            </div>
          </div>
        )}

        {mediaType === 'Image' && (
          <div className="border-t border-slate-700 pt-4 sm:pt-6">
            <label className="block text-xs sm:text-sm font-medium text-slate-300 mb-2 sm:mb-3">Image Settings</label>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 sm:gap-4">
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Quality</label>
                <input
                  type="range"
                  min="1"
                  max="100"
                  value={imageQuality}
                  onChange={(e) => setImageQuality(parseInt(e.target.value))}
                  className="w-full accent-cyan-500"
                />
                <div className="flex justify-between text-xs text-slate-500 mt-1">
                  <span>Low</span>
                  <span>{imageQuality}%</span>
                  <span>High</span>
                </div>
              </div>
              <div className="min-w-0">
                <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Resize</label>
                <select 
                  value={resizeOption}
                  onChange={(e) => setResizeOption(e.target.value)}
                  className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                >
                  <option>Keep Original</option>
                  <option>50% Size</option>
                  <option>25% Size</option>
                  <option>Custom</option>
                </select>
              </div>
            </div>
            {/* Custom Resize Inputs */}
            {resizeOption === 'Custom' && (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 sm:gap-4 mt-3 sm:mt-4 pt-3 sm:pt-4 border-t border-slate-700/50">
                <div className="min-w-0">
                  <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Width (pixels)</label>
                  <input
                    type="number"
                    value={customWidth}
                    onChange={(e) => setCustomWidth(e.target.value)}
                    placeholder="e.g., 1920"
                    min="1"
                    className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                  />
                </div>
                <div className="min-w-0">
                  <label className="block text-xs text-slate-400 mb-1.5 sm:mb-2">Height (pixels)</label>
                  <input
                    type="number"
                    value={customHeight}
                    onChange={(e) => setCustomHeight(e.target.value)}
                    placeholder="e.g., 1080"
                    min="1"
                    className="w-full px-2 sm:px-4 py-1.5 sm:py-2 text-xs sm:text-sm bg-slate-900/50 border border-slate-600 rounded-lg focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/20 transition-all"
                  />
                </div>
                <div className="col-span-1 sm:col-span-2">
                  <p className="text-xs text-slate-500">
                    💡 Leave height empty for proportional resize based on width only
                  </p>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      <button 
        onClick={handleStartConversion}
        disabled={isConverting || selectedFiles.length === 0}
        className="w-full py-3 sm:py-4 text-xs sm:text-sm lg:text-base bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-600 hover:to-blue-700 disabled:from-slate-600 disabled:to-slate-700 disabled:cursor-not-allowed text-white font-semibold rounded-lg sm:rounded-xl shadow-lg shadow-cyan-500/30 hover:shadow-cyan-500/50 transition-all transform hover:scale-[1.02] disabled:scale-100"
      >
        {isConverting ? 'Starting Conversion...' : 'Start Conversion'}
      </button>
    </div>
  );
}

export default ConvertSection;
