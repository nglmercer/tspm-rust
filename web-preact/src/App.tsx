import { useState, useCallback } from 'preact/hooks';
import { useProcesses } from './hooks/useProcesses';
import { useWebSocket } from './hooks/useWebSocket';
import { ProcessTable } from './components/ProcessTable';
import { LogsViewer } from './components/LogsViewer';
import { Terminal } from './components/Terminal';
import { ProcessForm } from './components/ProcessForm';
import { PortsView } from './components/PortsView';
import { StatsBar } from './components/StatsBar';
import type { WsMessage, ProcessLogEntry, SystemStats } from './types';

type Page = 'processes' | 'logs' | 'terminal' | 'ports';

export function App() {
    const [page, setPage] = useState<Page>('processes');
    const [showForm, setShowForm] = useState(false);
    const [systemStats, setSystemStats] = useState<SystemStats>({ cpu: 0, memory: 0, uptime: 0, processCount: 0 });
    const [logEntries, setLogEntries] = useState<ProcessLogEntry[]>([]);

    const { processes, updateFromWs, startProcess, stopProcess, restartProcess, deleteProcess, fetch: refresh } = useProcesses();

    const onWsMessage = useCallback((msg: WsMessage) => {
        switch (msg.type) {
            case 'process:update':
                updateFromWs(msg.payload.processes ?? []);
                break;
            case 'process:log':
                setLogEntries(prev => [...prev.slice(-999), msg.payload as ProcessLogEntry]);
                break;
            case 'system:stats':
                setSystemStats(msg.payload as SystemStats);
                break;
        }
    }, [updateFromWs]);

    useWebSocket(onWsMessage);

    const navClass = (p: Page) => `nav-item ${page === p ? 'active' : ''}`;

    return (
        <div class="app">
            <aside class="sidebar">
                <div class="logo">
                    <span class="logo-icon">⚡</span>
                    <span class="logo-text">TSPM</span>
                </div>
                <nav>
                    <button class={navClass('processes')} onClick={() => setPage('processes')}>
                        <span>📋</span> Processes
                    </button>
                    <button class={navClass('logs')} onClick={() => setPage('logs')}>
                        <span>📜</span> Logs
                    </button>
                    <button class={navClass('terminal')} onClick={() => setPage('terminal')}>
                        <span>💻</span> Terminal
                    </button>
                    <button class={navClass('ports')} onClick={() => setPage('ports')}>
                        <span>🔌</span> Ports
                    </button>
                </nav>
                <StatsBar stats={systemStats} />
            </aside>

            <main class="main">
                <header class="topbar">
                    <h1>{page === 'processes' ? 'Processes' : page === 'logs' ? 'Logs' : page === 'terminal' ? 'Terminal' : 'Ports'}</h1>
                    <div class="topbar-actions">
                        {page === 'processes' && (
                            <button class="btn btn-primary" onClick={() => setShowForm(true)}>
                                + New Process
                            </button>
                        )}
                        <button class="btn btn-ghost" onClick={refresh}>Refresh</button>
                    </div>
                </header>

                <div class="content">
                    {page === 'processes' && (
                        <ProcessTable
                            processes={processes}
                            onStart={startProcess}
                            onStop={stopProcess}
                            onRestart={restartProcess}
                            onDelete={deleteProcess}
                        />
                    )}
                    {page === 'logs' && <LogsViewer entries={logEntries} processes={processes} onClear={() => setLogEntries([])} />}
                    {page === 'terminal' && <Terminal />}
                    {page === 'ports' && <PortsView />}
                </div>
            </main>

            {showForm && <ProcessForm onClose={() => { setShowForm(false); refresh(); }} />}
        </div>
    );
}
