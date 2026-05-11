import { useState, useCallback } from 'preact/hooks';
import { useProcesses } from './hooks/useProcesses';
import { useWebSocket } from './hooks/useWebSocket';
import { ProcessTable } from './components/ProcessTable';
import { LogsViewer } from './components/LogsViewer';
import { Terminal } from './components/Terminal';
import { ProcessForm } from './components/ProcessForm';
import { PortsView } from './components/PortsView';
import { StatsBar } from './components/StatsBar';
import { Dialog } from './components/Dialog';
import type { WsMessage, ProcessLogEntry, SystemStats, AppPage } from './types';
import styles from './App.module.css';

export function App() {
    const [page, setPage] = useState<AppPage>('processes');
    const [showForm, setShowForm] = useState(false);
    const [systemStats, setSystemStats] = useState<SystemStats>({ cpu: 0, memory: 0, uptime: 0, processCount: 0 });
    const [logEntries, setLogEntries] = useState<ProcessLogEntry[]>([]);

    const { processes, updateFromWs, startProcess, stopProcess, restartProcess, deleteProcess, fetch: refresh } = useProcesses();

    const onWsMessage = useCallback((msg: WsMessage) => {
        switch (msg.type) {
            case 'process:update':
                updateFromWs(msg.payload.processes);
                break;
            case 'process:log':
                setLogEntries(prev => [...prev.slice(-999), msg.payload]);
                break;
            case 'system:stats':
                setSystemStats(msg.payload);
                break;
        }
    }, [updateFromWs]);

    useWebSocket(onWsMessage);

    const navClass = (p: AppPage) => `${styles.navItem} ${page === p ? styles.active : ''}`;

    return (
        <div class={styles.app}>
            <aside class={styles.sidebar}>
                <div class={styles.logo}>
                    <span class={styles.logoIcon}>⚡</span>
                    <span class={styles.logoText}>TSPM</span>
                </div>
                <nav class={styles.nav}>
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

            <main class={styles.main}>
                <header class={styles.topbar}>
                    <h1>{page === 'processes' ? 'Processes' : page === 'logs' ? 'Logs' : page === 'terminal' ? 'Terminal' : 'Ports'}</h1>
                    <div class={styles.topbarActions}>
                        {page === 'processes' && (
                            <button class="btn btn-primary" onClick={() => setShowForm(true)}>
                                + New Process
                            </button>
                        )}
                        <button class="btn btn-ghost" onClick={refresh}>Refresh</button>
                    </div>
                </header>

                <div class={styles.content}>
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
            <Dialog />
        </div>
    );
}
