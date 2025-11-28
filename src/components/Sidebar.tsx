import { Download, RefreshCw } from 'lucide-react';

interface SidebarProps {
  activeSection: 'download' | 'convert';
  onSectionChange: (section: 'download' | 'convert') => void;
}

function Sidebar({ activeSection, onSectionChange }: SidebarProps) {
  const sections = [
    { id: 'download' as const, label: 'Download', icon: Download },
    { id: 'convert' as const, label: 'Convert', icon: RefreshCw },
  ];

  return (
    <aside className="w-48 md:w-56 lg:w-64 border-r border-slate-700 bg-slate-900/30 p-2 md:p-3 lg:p-4 flex-shrink-0">
      <nav className="space-y-1.5 md:space-y-2">
        {sections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <button
              key={section.id}
              onClick={() => onSectionChange(section.id)}
              className={`w-full flex items-center gap-2 md:gap-3 px-3 md:px-4 py-2 md:py-3 rounded-lg font-medium transition-all text-sm md:text-base ${
                isActive
                  ? 'bg-gradient-to-r from-cyan-500/20 to-blue-500/20 text-cyan-400 border border-cyan-500/30 shadow-lg shadow-cyan-500/10'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
              }`}
            >
              <Icon className="w-4 h-4 md:w-5 md:h-5" />
              {section.label}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}

export default Sidebar;
