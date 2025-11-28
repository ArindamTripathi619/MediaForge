/**
 * Client-side validation utilities for MediaForge
 * Provides immediate feedback to users before sending requests to backend
 */

export interface ValidationResult {
  isValid: boolean;
  message?: string;
}

/**
 * Validates YouTube URLs on the client side for immediate feedback
 * Note: Backend performs more comprehensive security validation
 */
export const validateYouTubeUrl = (url: string): ValidationResult => {
  if (!url || url.trim() === '') {
    return { isValid: false, message: 'URL cannot be empty' };
  }

  const trimmedUrl = url.trim();

  // Check for obviously malicious patterns
  if (
    trimmedUrl.includes('\n') ||
    trimmedUrl.includes('\r') ||
    trimmedUrl.includes(';') ||
    trimmedUrl.includes('|') ||
    trimmedUrl.includes('`') ||
    trimmedUrl.includes('$(') ||
    trimmedUrl.startsWith('file://') ||
    trimmedUrl.startsWith('javascript:') ||
    trimmedUrl.startsWith('data:')
  ) {
    return { isValid: false, message: 'URL contains invalid characters or schemes' };
  }

  // Valid YouTube URL patterns (client-side check)
  const youtubePatterns = [
    /^https?:\/\/(www\.)?youtube\.com\/watch\?v=[\w-]{11}(&.*)?$/,
    /^https?:\/\/youtu\.be\/[\w-]{11}(\?.*)?$/,
    /^https?:\/\/(www\.)?youtube\.com\/playlist\?list=[\w-]+(&.*)?$/,
    /^https?:\/\/(music\.)?youtube\.com\/watch\?v=[\w-]{11}(&.*)?$/,
    /^https?:\/\/(www\.)?youtube\.com\/shorts\/[\w-]{11}(\?.*)?$/,
  ];

  const isValidYouTube = youtubePatterns.some(pattern => pattern.test(trimmedUrl));
  
  if (!isValidYouTube) {
    return { isValid: false, message: 'Please enter a valid YouTube URL' };
  }

  return { isValid: true };
};

/**
 * Validates multiple URLs and returns validation results for each
 */
export const validateUrls = (urls: string[]): { results: ValidationResult[]; hasErrors: boolean } => {
  const results = urls.map(url => validateYouTubeUrl(url));
  const hasErrors = results.some(result => !result.isValid);
  
  return { results, hasErrors };
};

/**
 * Validates time format for trim functionality (HH:MM:SS or MM:SS)
 */
export const validateTimeFormat = (time: string): ValidationResult => {
  if (!time || time.trim() === '') {
    return { isValid: false, message: 'Time cannot be empty' };
  }

  const trimmedTime = time.trim();
  
  // Support formats: HH:MM:SS or MM:SS or SS
  const timePatterns = [
    /^([0-5]?[0-9]):([0-5][0-9]):([0-5][0-9])$/, // HH:MM:SS
    /^([0-5]?[0-9]):([0-5][0-9])$/, // MM:SS
    /^([0-9]+)$/, // SS (seconds only)
  ];

  const isValidFormat = timePatterns.some(pattern => pattern.test(trimmedTime));
  
  if (!isValidFormat) {
    return { isValid: false, message: 'Please use format: HH:MM:SS, MM:SS, or SS' };
  }

  // Additional validation: ensure times are reasonable
  const parts = trimmedTime.split(':').map(Number);
  if (parts.length === 3) {
    const [hours, minutes, seconds] = parts;
    if (hours > 24 || minutes > 59 || seconds > 59) {
      return { isValid: false, message: 'Invalid time values (hours ≤ 24, minutes/seconds ≤ 59)' };
    }
  } else if (parts.length === 2) {
    const [minutes, seconds] = parts;
    if (minutes > 59 || seconds > 59) {
      return { isValid: false, message: 'Invalid time values (minutes/seconds ≤ 59)' };
    }
  } else if (parts.length === 1) {
    const [seconds] = parts;
    if (seconds > 86400) { // Max 24 hours in seconds
      return { isValid: false, message: 'Seconds cannot exceed 86400 (24 hours)' };
    }
  }

  return { isValid: true };
};

/**
 * Validates that start time is before end time
 */
export const validateTimeRange = (startTime: string, endTime: string): ValidationResult => {
  const startResult = validateTimeFormat(startTime);
  const endResult = validateTimeFormat(endTime);
  
  if (!startResult.isValid) {
    return { isValid: false, message: `Start time: ${startResult.message}` };
  }
  
  if (!endResult.isValid) {
    return { isValid: false, message: `End time: ${endResult.message}` };
  }

  // Convert to seconds for comparison
  const startSeconds = timeToSeconds(startTime);
  const endSeconds = timeToSeconds(endTime);
  
  if (startSeconds >= endSeconds) {
    return { isValid: false, message: 'Start time must be before end time' };
  }

  return { isValid: true };
};

/**
 * Converts time string to seconds for comparison
 */
const timeToSeconds = (time: string): number => {
  const parts = time.trim().split(':').map(Number);
  
  if (parts.length === 3) {
    const [hours, minutes, seconds] = parts;
    return hours * 3600 + minutes * 60 + seconds;
  } else if (parts.length === 2) {
    const [minutes, seconds] = parts;
    return minutes * 60 + seconds;
  } else {
    return parts[0]; // seconds only
  }
};

/**
 * Validates file size limits (in bytes)
 */
export const validateFileSize = (fileSize: number, maxSizeBytes: number = 2 * 1024 * 1024 * 1024): ValidationResult => {
  if (fileSize > maxSizeBytes) {
    const maxSizeMB = maxSizeBytes / (1024 * 1024);
    const fileSizeMB = fileSize / (1024 * 1024);
    return {
      isValid: false,
      message: `File is too large (${fileSizeMB.toFixed(1)}MB). Maximum allowed: ${maxSizeMB.toFixed(0)}MB`
    };
  }

  return { isValid: true };
};

/**
 * Validates output path format (basic client-side check)
 */
export const validateOutputPath = (path: string): ValidationResult => {
  if (!path || path.trim() === '') {
    return { isValid: false, message: 'Output path cannot be empty' };
  }

  const trimmedPath = path.trim();
  
  // Check for obviously invalid characters (basic check)
  if (trimmedPath.includes('\n') || trimmedPath.includes('\r')) {
    return { isValid: false, message: 'Path cannot contain line breaks' };
  }

  // Check for relative path attempts that could be malicious
  if (trimmedPath.includes('../') || trimmedPath.includes('..\\')) {
    return { isValid: false, message: 'Relative paths with .. are not allowed' };
  }

  return { isValid: true };
};

/**
 * Validates image dimensions for custom resize
 */
export const validateImageDimensions = (width: string, height: string): ValidationResult => {
  if (width && width.trim() !== '') {
    const widthNum = parseInt(width.trim());
    if (isNaN(widthNum) || widthNum <= 0 || widthNum > 10000) {
      return { isValid: false, message: 'Width must be a number between 1 and 10000' };
    }
  }

  if (height && height.trim() !== '') {
    const heightNum = parseInt(height.trim());
    if (isNaN(heightNum) || heightNum <= 0 || heightNum > 10000) {
      return { isValid: false, message: 'Height must be a number between 1 and 10000' };
    }
  }

  return { isValid: true };
};

/**
 * Utility function to format file size in human-readable format
 */
export const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
};