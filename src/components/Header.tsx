import { Sparkles, Download, RefreshCw } from 'lucide-react';

interface HeaderProps {
  activeSection: 'download' | 'convert';
  onSectionChange: (section: 'download' | 'convert') => void;
}

function Header({ activeSection, onSectionChange }: HeaderProps) {
  const sections = [
    { id: 'download' as const, label: 'Download', icon: Download },
    { id: 'convert' as const, label: 'Convert', icon: RefreshCw },
  ];

  return (
    <header className="h-16 border-b border-slate-700 bg-slate-900/50 backdrop-blur-sm sticky top-0 z-50">
      <div className="h-full px-3 sm:px-6 lg:px-8 flex items-center justify-between gap-2 lg:gap-6">
        <div className="flex items-center gap-2 sm:gap-3 min-w-0 flex-shrink-0">
          <div className="w-9 h-9 sm:w-10 sm:h-10 bg-gradient-to-br from-cyan-500 to-blue-600 rounded-lg flex items-center justify-center shadow-lg shadow-cyan-500/20">
            <Sparkles className="w-5 h-5 sm:w-6 sm:h-6" />
          </div>
          <div className="hidden xs:block min-w-0">
            <h1 className="text-sm sm:text-base lg:text-xl font-bold bg-gradient-to-r from-cyan-400 to-blue-500 bg-clip-text text-transparent truncate">
              MediaForge
            </h1>
            <p className="text-xs text-slate-400 truncate hidden sm:block">Download & Convert</p>
          </div>
        </div>

        <nav className="flex items-center gap-1.5 sm:gap-3">
          {sections.map((section) => {
            const Icon = section.icon;
            const isActive = activeSection === section.id;

            return (
              <button
                key={section.id}
                onClick={() => onSectionChange(section.id)}
                className={`flex items-center gap-1 sm:gap-2 px-2.5 sm:px-4 py-2 rounded-lg font-medium text-xs sm:text-sm whitespace-nowrap transition-all ${
                  isActive
                    ? 'bg-gradient-to-r from-cyan-500/20 to-blue-500/20 text-cyan-400 border border-cyan-500/30 shadow-lg shadow-cyan-500/10'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
                }`}
              >
                <Icon className="w-4 h-4 sm:w-5 sm:h-5" />
                <span className="hidden sm:inline">{section.label}</span>
              </button>
            );
          })}
        </nav>
      </div>
    </header>
  );
}

export default Header;
