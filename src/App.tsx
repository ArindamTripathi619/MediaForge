import { useState } from 'react';
import Header from './components/Header';
import DownloadSection from './components/DownloadSection';
import ConvertSection from './components/ConvertSection';
import DownloadQueue from './components/DownloadQueue';
import SplashScreen from './components/SplashScreen';

type ActiveSection = 'download' | 'convert';

function App() {
  const [activeSection, setActiveSection] = useState<ActiveSection>('download');
  const [showSplash, setShowSplash] = useState(true);

  if (showSplash) {
    return <SplashScreen onComplete={() => setShowSplash(false)} minDuration={2500} />;
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-white">
      <Header activeSection={activeSection} onSectionChange={setActiveSection} />

      <div className="flex h-[calc(100vh-64px)]">
        <main className="flex-1 overflow-y-auto min-w-0">
          <div className="w-full max-w-full px-4 sm:px-6 lg:px-8 py-6 lg:py-8">
            {activeSection === 'download' ? (
              <DownloadSection />
            ) : (
              <ConvertSection />
            )}
          </div>
        </main>

        <DownloadQueue />
      </div>
    </div>
  );
}

export default App;
