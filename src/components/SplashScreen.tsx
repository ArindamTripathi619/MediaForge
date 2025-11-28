import { useEffect, useState } from 'react';

interface SplashScreenProps {
  onComplete: () => void;
  minDuration?: number;
}

function SplashScreen({ onComplete, minDuration = 2000 }: SplashScreenProps) {
  const [progress, setProgress] = useState(0);
  const [fadeOut, setFadeOut] = useState(false);

  useEffect(() => {
    // Simulate loading progress
    const progressInterval = setInterval(() => {
      setProgress((prev) => {
        if (prev >= 100) {
          clearInterval(progressInterval);
          return 100;
        }
        return prev + 2;
      });
    }, minDuration / 50);

    // Minimum display duration
    const timer = setTimeout(() => {
      setFadeOut(true);
      setTimeout(onComplete, 500); // Wait for fade out animation
    }, minDuration);

    return () => {
      clearInterval(progressInterval);
      clearTimeout(timer);
    };
  }, [onComplete, minDuration]);

  return (
    <div
      className={`fixed inset-0 z-50 flex flex-col items-center justify-center bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 transition-opacity duration-500 ${
        fadeOut ? 'opacity-0' : 'opacity-100'
      }`}
    >
      {/* Animated background glow */}
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] sm:w-[600px] sm:h-[600px] lg:w-[800px] lg:h-[800px] bg-cyan-500/10 rounded-full blur-3xl animate-pulse" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[300px] h-[300px] sm:w-[400px] sm:h-[400px] lg:w-[500px] lg:h-[500px] bg-blue-600/10 rounded-full blur-2xl animate-pulse" style={{ animationDelay: '0.5s' }} />
      </div>

      {/* Logo container with glow effect */}
      <div className="relative z-10 mb-8 sm:mb-12">
        {/* Pulsing glow behind logo */}
        <div className="absolute inset-0 -m-4 sm:-m-6 lg:-m-8 bg-gradient-to-r from-cyan-500/20 to-blue-600/20 rounded-full blur-xl animate-pulse" />
        
        {/* Logo */}
        <div className="relative">
          <img
            src="/MediaForge_logo_cropped.webp"
            alt="MediaForge Logo"
            className="w-32 h-32 sm:w-40 sm:h-40 lg:w-48 lg:h-48 object-contain animate-fade-in"
          />
        </div>
      </div>

      {/* App Name */}
      <h1 className="text-3xl sm:text-4xl lg:text-5xl font-bold text-white mb-2 sm:mb-3 animate-fade-in-up tracking-tight">
        MediaForge
      </h1>

      {/* Tagline */}
      <p className="text-sm sm:text-base lg:text-lg text-slate-400 mb-8 sm:mb-12 lg:mb-16 animate-fade-in-up tracking-wide" style={{ animationDelay: '0.2s' }}>
        Download & Convert Media
      </p>

      {/* Loading indicator */}
      <div className="w-48 sm:w-64 lg:w-80 animate-fade-in-up" style={{ animationDelay: '0.4s' }}>
        {/* Progress bar container */}
        <div className="relative h-1.5 sm:h-2 bg-slate-800 rounded-full overflow-hidden">
          {/* Background shimmer */}
          <div className="absolute inset-0 bg-gradient-to-r from-transparent via-slate-700/50 to-transparent animate-shimmer" />
          
          {/* Progress fill */}
          <div
            className="absolute inset-y-0 left-0 bg-gradient-to-r from-cyan-500 to-blue-600 rounded-full transition-all duration-300 ease-out"
            style={{ width: `${progress}%` }}
          >
            {/* Glow effect on progress bar */}
            <div className="absolute inset-0 bg-gradient-to-r from-cyan-400 to-blue-500 opacity-50 blur-sm" />
          </div>
        </div>

        {/* Loading text */}
        <div className="mt-3 sm:mt-4 text-center">
          <span className="text-xs sm:text-sm text-slate-500 font-medium">
            Loading...
          </span>
        </div>
      </div>

      {/* Version info (optional) */}
      <div className="absolute bottom-6 sm:bottom-8 lg:bottom-10 text-xs sm:text-sm text-slate-600">
        Version 1.0.0
      </div>
    </div>
  );
}

export default SplashScreen;
