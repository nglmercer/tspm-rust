import { useState, useCallback } from 'preact/hooks';
import { useProcesses } from './hooks/useProcesses';
import { useSystemStats } from './hooks/useSystemStats';
import { useWebSocket } from './hooks/useWebSocket';
import { ProcessTable } from './components/ProcessTable';
import { LogsViewer } from './components/LogsViewer';
import { Terminal } from './components/Terminal';
import { ProcessForm } from './components/ProcessForm';
import { PortsView } from './components/PortsView';
import { StatsBar } from './components/StatsBar';
import { Dialog } from './components/Dialog';
import { 
    IconActivity, 
    IconFileText, 
    IconTerminal, 
    IconPlug, 
    IconZap, 
    IconPlus, 
    IconRefresh 
} from './components/Icons';
import type { WsMessage, ProcessLogEntry, AppPage } from './types';
import styles from './App.module.css';

export function App() {
    const [page, setPage] = useState<AppPage>('processes');
    const [showForm, setShowForm] = useState(false);
    const [logEntries, setLogEntries] = useState<ProcessLogEntry[]>([]);

    const { processes, updateFromWs: updateProcs, startProcess, stopProcess, restartProcess, deleteProcess, fetch: refresh } = useProcesses();
    const { stats: systemStats, updateFromWs: updateStats } = useSystemStats();

    const onWsMessage = useCallback((msg: WsMessage) => {
        switch (msg.type) {
            case 'process:update':
                updateProcs(msg.payload.processes);
                break;
            case 'process:log':
                setLogEntries(prev => [...prev.slice(-999), msg.payload]);
                break;
            case 'system:stats':
                updateStats(msg.payload);
                break;
        }
    }, [updateProcs, updateStats]);

    useWebSocket(onWsMessage);

    const navClass = (p: AppPage) => `${styles.navItem} ${page === p ? styles.active : ''}`;

    return (
        <div class={styles.app}>
            <aside class={styles.sidebar}>
                <div class={styles.logo}>
                    <span class={styles.logoIcon}><IconZap size={24} /></span>
                    <span class={styles.logoText}>TSPM</span>
                </div>
                <nav class={styles.nav}>
                    <button class={navClass('processes')} onClick={() => setPage('processes')}>
                        <IconActivity size={18} /> Processes
                    </button>
                    <button class={navClass('logs')} onClick={() => setPage('logs')}>
                        <IconFileText size={18} /> Logs
                    </button>
                    <button class={navClass('terminal')} onClick={() => setPage('terminal')}>
                        <IconTerminal size={18} /> Terminal
                    </button>
                    <button class={navClass('ports')} onClick={() => setPage('ports')}>
                        <IconPlug size={18} /> Ports
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
                                <IconPlus size={18} /> New Process
                            </button>
                        )}
                        <button class="btn btn-ghost" onClick={refresh}>
                            <IconRefresh size={18} /> Refresh
                        </button>
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
